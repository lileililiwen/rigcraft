//! Store schema and migration tests. These used to live inside `src/store.rs`;
//! they moved out when the catalog tables pushed that file to its size limit.

use std::collections::BTreeMap;

use rigcraft::domain::{Provenance, Record, Snapshot, SnapshotSource};
use rigcraft::store::{Store, StoreError, SCHEMA_VERSION};
use rusqlite::Connection;

fn temp_store() -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(&dir.path().join("inventory.sqlite3")).unwrap();
    (dir, store)
}

fn record(id: &str) -> Record {
    let mut provenance = BTreeMap::new();
    provenance.insert(
        "sku".to_string(),
        Provenance::user("csv:test.csv", "2026-10-31T00:00:00Z"),
    );
    Record {
        id: id.to_string(),
        lot: 1,
        sku: "A".to_string(),
        category: "cpu".to_string(),
        model_raw: "X".to_string(),
        model_normalized: "X".to_string(),
        quantity: 1,
        price_minor: 100,
        currency: "USD".to_string(),
        specs: Vec::new(),
        provenance,
        match_info: Default::default(),
    }
}

fn snapshot(id: &str) -> Snapshot {
    let records = vec![record(&format!("{id}-r1"))];
    Snapshot {
        id: id.to_string(),
        created_at: "2026-10-31T00:00:00Z".to_string(),
        revision: 1,
        parent_snapshot_id: None,
        currency: "USD".to_string(),
        source: SnapshotSource {
            filename: "test.csv".to_string(),
            sha256: "abc".to_string(),
        },
        note: None,
        totals: Snapshot::totals_for(&records).unwrap(),
        records,
    }
}

#[test]
fn schema_has_a_version_and_reopens() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("inventory.sqlite3");
    {
        let _store = Store::open(&path).unwrap();
    }
    let store = Store::open(&path).unwrap();
    assert_eq!(store.schema_version().unwrap(), SCHEMA_VERSION);
    assert_eq!(SCHEMA_VERSION, 5);
}

#[test]
fn snapshot_round_trips_records_and_totals() {
    let (_dir, store) = temp_store();
    store.insert_correction(&snapshot("s1")).unwrap();
    let loaded = store.get_snapshot("s1").unwrap();
    assert_eq!(loaded.records.len(), 1);
    assert_eq!(loaded.totals.record_count, 1);
    assert_eq!(loaded.totals.value_minor, 100);
    assert_eq!(loaded.records[0].provenance["sku"].source, "user");
    assert_eq!(store.list_snapshots().unwrap().len(), 1);
}

#[test]
fn missing_snapshot_is_not_found() {
    let (_dir, store) = temp_store();
    assert!(matches!(
        store.get_snapshot("nope"),
        Err(StoreError::NotFound)
    ));
}

#[test]
fn check_reports_round_trip_with_their_revision() {
    use rigcraft::compatibility::{check_build, CheckRequest, SelectionItem};
    let (_dir, store) = temp_store();
    let snapshot = snapshot("s1");
    store.insert_correction(&snapshot).unwrap();
    let report = check_build(
        &snapshot,
        &CheckRequest {
            snapshot_id: "s1".to_string(),
            selection: vec![SelectionItem {
                sku: "A".to_string(),
                quantity: 1,
            }],
        },
    )
    .unwrap();
    store.insert_check_report(&report).unwrap();
    let loaded = store.get_check_report(&report.id).unwrap();
    assert_eq!(loaded.snapshot_id, "s1");
    assert_eq!(loaded.snapshot_revision, 1);
    assert_eq!(
        loaded.rule_set_version,
        rigcraft::compatibility::RULE_SET_VERSION
    );
    assert!(matches!(
        store.get_check_report("missing"),
        Err(StoreError::NotFound)
    ));
}

#[test]
fn schema_four_preserves_v3_data_and_adds_diagnoses() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("inventory.sqlite3");
    {
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE snapshots (
                    id TEXT PRIMARY KEY,
                    created_at TEXT NOT NULL,
                    revision INTEGER NOT NULL,
                    parent_snapshot_id TEXT REFERENCES snapshots(id),
                    currency TEXT NOT NULL,
                    source_filename TEXT NOT NULL,
                    source_sha256 TEXT NOT NULL,
                    note TEXT,
                    unit_count INTEGER NOT NULL,
                    value_minor INTEGER NOT NULL
                 );
                 CREATE TABLE records (
                    id TEXT PRIMARY KEY,
                    snapshot_id TEXT NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
                    position INTEGER NOT NULL,
                    lot INTEGER NOT NULL,
                    sku TEXT NOT NULL,
                    category TEXT NOT NULL,
                    model_raw TEXT NOT NULL,
                    model_normalized TEXT NOT NULL,
                    quantity INTEGER NOT NULL,
                    price_minor INTEGER NOT NULL,
                    currency TEXT NOT NULL,
                    specs_json TEXT NOT NULL,
                    provenance_json TEXT NOT NULL,
                    match_json TEXT NOT NULL
                 );
                 INSERT INTO snapshots
                    (id, created_at, revision, currency, source_filename, source_sha256,
                     unit_count, value_minor)
                    VALUES ('v3', '2026-10-30T00:00:00Z', 1, 'USD', 'v3.csv', 'sha', 1, 100);
                 INSERT INTO records
                    (id, snapshot_id, position, lot, sku, category, model_raw,
                     model_normalized, quantity, price_minor, currency, specs_json,
                     provenance_json, match_json)
                    VALUES ('v3-r1', 'v3', 0, 1, 'V3', 'cpu', 'V3 CPU', 'V3 CPU', 1, 100,
                            'USD', '[]', '{}',
                            '{\"basis\":\"lexical\",\"confidence\":\"unverified\"}');",
            )
            .unwrap();
        connection.pragma_update(None, "user_version", 3).unwrap();
    }
    let store = Store::open(&path).unwrap();
    assert_eq!(store.schema_version().unwrap(), SCHEMA_VERSION);
    let snapshot = store.get_snapshot("v3").unwrap();
    assert_eq!(snapshot.records[0].sku, "V3");
    assert!(matches!(
        store.get_diagnosis("missing"),
        Err(StoreError::NotFound)
    ));
}

#[test]
fn schema_five_preserves_v4_data_and_adds_the_catalog_tables() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("inventory.sqlite3");
    {
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE snapshots (
                    id TEXT PRIMARY KEY,
                    created_at TEXT NOT NULL,
                    revision INTEGER NOT NULL,
                    parent_snapshot_id TEXT REFERENCES snapshots(id),
                    currency TEXT NOT NULL,
                    source_filename TEXT NOT NULL,
                    source_sha256 TEXT NOT NULL,
                    note TEXT,
                    unit_count INTEGER NOT NULL,
                    value_minor INTEGER NOT NULL
                 );
                 CREATE TABLE records (
                    id TEXT PRIMARY KEY,
                    snapshot_id TEXT NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
                    position INTEGER NOT NULL,
                    lot INTEGER NOT NULL,
                    sku TEXT NOT NULL,
                    category TEXT NOT NULL,
                    model_raw TEXT NOT NULL,
                    model_normalized TEXT NOT NULL,
                    quantity INTEGER NOT NULL,
                    price_minor INTEGER NOT NULL,
                    currency TEXT NOT NULL,
                    specs_json TEXT NOT NULL,
                    provenance_json TEXT NOT NULL,
                    match_json TEXT NOT NULL
                 );
                 INSERT INTO snapshots
                    (id, created_at, revision, currency, source_filename, source_sha256,
                     unit_count, value_minor)
                    VALUES ('v4', '2026-10-30T00:00:00Z', 1, 'USD', 'v4.csv', 'sha', 1, 100);
                 INSERT INTO records
                    (id, snapshot_id, position, lot, sku, category, model_raw,
                     model_normalized, quantity, price_minor, currency, specs_json,
                     provenance_json, match_json)
                    VALUES ('v4-r1', 'v4', 0, 1, 'V4', 'cpu', 'V4 CPU', 'V4 CPU', 1, 100,
                            'USD', '[]', '{}',
                            '{\"basis\":\"lexical\",\"confidence\":\"unverified\"}');
                 CREATE TABLE check_reports (
                    id TEXT PRIMARY KEY,
                    snapshot_id TEXT NOT NULL REFERENCES snapshots(id),
                    snapshot_revision INTEGER NOT NULL,
                    rule_set_version TEXT NOT NULL,
                    selection_json TEXT NOT NULL,
                    overall_status TEXT NOT NULL,
                    completeness TEXT NOT NULL,
                    checked_at TEXT NOT NULL,
                    report_json TEXT NOT NULL
                 );
                 CREATE TABLE recommendations (
                    id TEXT PRIMARY KEY,
                    created_at TEXT NOT NULL,
                    snapshot_id TEXT NOT NULL REFERENCES snapshots(id),
                    snapshot_revision INTEGER NOT NULL,
                    scoring_version TEXT NOT NULL,
                    rule_set_version TEXT NOT NULL,
                    status TEXT NOT NULL,
                    request_json TEXT NOT NULL,
                    report_json TEXT NOT NULL
                 );
                 CREATE TABLE diagnoses (
                    id TEXT PRIMARY KEY,
                    created_at TEXT NOT NULL,
                    snapshot_id TEXT NOT NULL REFERENCES snapshots(id),
                    snapshot_revision INTEGER NOT NULL,
                    rule_set_version TEXT NOT NULL,
                    status TEXT NOT NULL,
                    report_json TEXT NOT NULL
                 );",
            )
            .unwrap();
        connection.pragma_update(None, "user_version", 4).unwrap();
    }
    let store = Store::open(&path).unwrap();
    assert_eq!(store.schema_version().unwrap(), 5);
    let snapshot = store.get_snapshot("v4").unwrap();
    assert_eq!(snapshot.records.len(), 1);
    assert_eq!(snapshot.records[0].sku, "V4");
    assert!(store.list_catalog_datasets().unwrap().is_empty());
    // The new tables are real: an insert and read succeed after migration.
    let catalog = rigcraft::catalog::validate_catalog(&serde_json::json!({
        "format": "rigcraft-catalog",
        "version": 1,
        "dataset": { "name": "migration-check", "license": "CC0-1.0" },
        "parts": [{ "sku": "MIG-1", "category": "cpu", "model": "Migration CPU" }]
    }))
    .unwrap();
    store
        .import_catalog(&catalog, "2026-11-01T00:00:00Z")
        .unwrap();
    assert_eq!(store.list_catalog_datasets().unwrap().len(), 1);
}

#[test]
fn latest_chain_revision_follows_corrections() {
    let (_dir, store) = temp_store();
    let first = snapshot("s1");
    store.insert_correction(&first).unwrap();
    assert_eq!(store.latest_chain_revision("s1").unwrap(), Some(1));
    let mut second = snapshot("s2");
    second.revision = 2;
    second.parent_snapshot_id = Some("s1".to_string());
    store.insert_correction(&second).unwrap();
    assert_eq!(store.latest_chain_revision("s1").unwrap(), Some(2));
    assert_eq!(store.latest_chain_revision("s2").unwrap(), Some(2));
    assert_eq!(store.latest_chain_revision("missing").unwrap(), None);
}

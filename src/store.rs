//! SQLite persistence: immutable snapshots, preview staging, and migrations.

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rusqlite::{params, Connection, OptionalExtension};
use serde_json;

use crate::compatibility::CheckReport;
use crate::diagnosis::DiagnosisReport;
use crate::domain::{Preview, Record, Snapshot, SnapshotSource, SnapshotSummary, SnapshotTotals};
use crate::recommend::RecommendationReport;

const SCHEMA_VERSION: i64 = 4;

#[derive(Debug)]
pub enum StoreError {
    NotFound,
    Conflict,
    Sql(rusqlite::Error),
    Json(serde_json::Error),
}

impl fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::NotFound => write!(formatter, "record not found"),
            StoreError::Conflict => write!(formatter, "conflicting write"),
            StoreError::Sql(error) => write!(formatter, "sqlite error: {error}"),
            StoreError::Json(error) => write!(formatter, "json error: {error}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<rusqlite::Error> for StoreError {
    fn from(error: rusqlite::Error) -> Self {
        StoreError::Sql(error)
    }
}

impl From<serde_json::Error> for StoreError {
    fn from(error: serde_json::Error) -> Self {
        StoreError::Json(error)
    }
}

pub type StoreResult<T> = Result<T, StoreError>;

#[derive(Debug, Clone)]
pub struct StoredPreview {
    pub preview: Preview,
    pub created_at: String,
    pub sha256: String,
    pub commit_hash: Option<String>,
    pub snapshot_id: Option<String>,
}

pub enum CommitOutcome {
    Created,
    Existing(String),
}

pub struct Store {
    connection: Mutex<Connection>,
    db_path: PathBuf,
}

impl Store {
    pub fn open(path: &Path) -> StoreResult<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|_| StoreError::Sql(rusqlite::Error::InvalidPath(path.into())))?;
        }
        let connection = Connection::open(path)?;
        connection.query_row("PRAGMA journal_mode=WAL", [], |row| row.get::<_, String>(0))?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        migrate(&connection)?;
        Ok(Self {
            connection: Mutex::new(connection),
            db_path: path.to_path_buf(),
        })
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.connection
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn insert_preview(&self, preview: &Preview, sha256: &str) -> StoreResult<()> {
        let json = serde_json::to_string(preview)?;
        self.lock().execute(
            "INSERT INTO previews (id, created_at, sha256, preview_json) VALUES (?1, ?2, ?3, ?4)",
            params![preview.preview_id, preview.created_at, sha256, json],
        )?;
        Ok(())
    }

    pub fn get_preview(&self, preview_id: &str) -> StoreResult<StoredPreview> {
        let connection = self.lock();
        let row = connection
            .query_row(
                "SELECT preview_json, created_at, sha256, commit_hash, snapshot_id FROM previews WHERE id = ?1",
                params![preview_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, Option<String>>(4)?,
                    ))
                },
            )
            .optional()?;
        let Some((json, created_at, sha256, commit_hash, snapshot_id)) = row else {
            return Err(StoreError::NotFound);
        };
        Ok(StoredPreview {
            preview: serde_json::from_str(&json)?,
            created_at,
            sha256,
            commit_hash,
            snapshot_id,
        })
    }

    /// Insert a snapshot and mark the preview committed in one transaction.
    /// Returns `Existing` when the same canonical decision set was replayed.
    pub fn commit_preview(
        &self,
        preview_id: &str,
        decision_hash: &str,
        snapshot: &Snapshot,
    ) -> StoreResult<CommitOutcome> {
        let mut connection = self.lock();
        let transaction = connection.transaction()?;
        let existing = transaction
            .query_row(
                "SELECT commit_hash, snapshot_id FROM previews WHERE id = ?1",
                params![preview_id],
                |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?,
                        row.get::<_, Option<String>>(1)?,
                    ))
                },
            )
            .optional()?;
        let Some((commit_hash, snapshot_id)) = existing else {
            return Err(StoreError::NotFound);
        };
        if let Some(previous_hash) = commit_hash {
            if previous_hash == decision_hash {
                if let Some(snapshot_id) = snapshot_id {
                    return Ok(CommitOutcome::Existing(snapshot_id));
                }
            }
            return Err(StoreError::Conflict);
        }
        insert_snapshot(&transaction, snapshot)?;
        let changed = transaction.execute(
            "UPDATE previews SET commit_hash = ?1, snapshot_id = ?2 WHERE id = ?3 AND commit_hash IS NULL",
            params![decision_hash, snapshot.id, preview_id],
        )?;
        if changed != 1 {
            return Err(StoreError::Conflict);
        }
        transaction.commit()?;
        Ok(CommitOutcome::Created)
    }

    pub fn insert_correction(&self, snapshot: &Snapshot) -> StoreResult<()> {
        let mut connection = self.lock();
        let transaction = connection.transaction()?;
        insert_snapshot(&transaction, snapshot)?;
        transaction.commit()?;
        Ok(())
    }

    pub fn get_snapshot(&self, snapshot_id: &str) -> StoreResult<Snapshot> {
        let connection = self.lock();
        let header = connection
            .query_row(
                "SELECT id, created_at, revision, parent_snapshot_id, currency,
                        source_filename, source_sha256, note, unit_count, value_minor
                 FROM snapshots WHERE id = ?1",
                params![snapshot_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, String>(6)?,
                        row.get::<_, Option<String>>(7)?,
                        row.get::<_, i64>(8)?,
                        row.get::<_, i64>(9)?,
                    ))
                },
            )
            .optional()?;
        let Some((
            id,
            created_at,
            revision,
            parent_snapshot_id,
            currency,
            filename,
            sha256,
            note,
            unit_count,
            value_minor,
        )) = header
        else {
            return Err(StoreError::NotFound);
        };
        let mut statement = connection.prepare(
            "SELECT id, lot, sku, category, model_raw, model_normalized, quantity,
                    price_minor, currency, specs_json, provenance_json, match_json
             FROM records WHERE snapshot_id = ?1 ORDER BY position",
        )?;
        let rows = statement
            .query_map(params![snapshot_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, String>(10)?,
                    row.get::<_, String>(11)?,
                ))
            })?
            .collect::<Result<Vec<_>, rusqlite::Error>>()?;
        let mut records = Vec::with_capacity(rows.len());
        for row in rows {
            let (
                id,
                lot,
                sku,
                category,
                model_raw,
                model_normalized,
                quantity,
                price_minor,
                currency,
                specs_json,
                provenance_json,
                match_json,
            ) = row;
            records.push(Record {
                id,
                lot,
                sku,
                category,
                model_raw,
                model_normalized,
                quantity,
                price_minor,
                currency,
                specs: serde_json::from_str(&specs_json)?,
                provenance: serde_json::from_str(&provenance_json)?,
                match_info: serde_json::from_str(&match_json)?,
            });
        }
        let totals = SnapshotTotals {
            record_count: records.len(),
            unit_count,
            value_minor,
        };
        Ok(Snapshot {
            id,
            created_at,
            revision,
            parent_snapshot_id,
            currency,
            source: SnapshotSource { filename, sha256 },
            note,
            records,
            totals,
        })
    }

    pub fn insert_check_report(&self, report: &CheckReport) -> StoreResult<()> {
        insert_check_row(&self.lock(), report)
    }

    /// Persist a recommendation and the check report of every candidate in one
    /// transaction, so a failed write never leaves a partial recommendation.
    pub fn insert_recommendation(&self, report: &RecommendationReport) -> StoreResult<()> {
        let mut connection = self.lock();
        let transaction = connection.transaction()?;
        for candidate in &report.candidates {
            insert_check_row(&transaction, &candidate.check)?;
        }
        let request_json = serde_json::to_string(&report.request)?;
        let report_json = serde_json::to_string(report)?;
        transaction.execute(
            "INSERT INTO recommendations
             (id, created_at, snapshot_id, snapshot_revision, scoring_version,
              rule_set_version, status, request_json, report_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                report.id,
                report.created_at,
                report.snapshot_id,
                report.snapshot_revision,
                report.scoring_version,
                report.rule_set_version,
                serde_json::to_string(&report.status)?,
                request_json,
                report_json,
            ],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn get_recommendation(&self, id: &str) -> StoreResult<RecommendationReport> {
        let connection = self.lock();
        let json = connection
            .query_row(
                "SELECT report_json FROM recommendations WHERE id = ?1",
                params![id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        match json {
            Some(json) => Ok(serde_json::from_str(&json)?),
            None => Err(StoreError::NotFound),
        }
    }

    /// Highest revision in the correction chain rooted at `snapshot_id`.
    /// A snapshot row is immutable, so staleness is found by following child
    /// revisions rather than by re-reading the same row.
    pub fn latest_chain_revision(&self, snapshot_id: &str) -> StoreResult<Option<i64>> {
        let connection = self.lock();
        let revision = connection
            .query_row(
                "WITH RECURSIVE chain(id, revision) AS (
                    SELECT id, revision FROM snapshots WHERE id = ?1
                    UNION ALL
                    SELECT s.id, s.revision FROM snapshots s
                    JOIN chain c ON s.parent_snapshot_id = c.id
                 )
                 SELECT MAX(revision) FROM chain",
                params![snapshot_id],
                |row| row.get::<_, Option<i64>>(0),
            )
            .optional()?
            .flatten();
        Ok(revision)
    }

    /// Persist a completed diagnosis. It is derived data: no snapshot, stock or
    /// plan state is mutated, and a failed validation never reaches this call.
    pub fn insert_diagnosis(&self, report: &DiagnosisReport) -> StoreResult<()> {
        let connection = self.lock();
        connection.execute(
            "INSERT INTO diagnoses
             (id, created_at, snapshot_id, snapshot_revision, rule_set_version, status, report_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                report.id,
                report.created_at,
                report.snapshot_id,
                report.snapshot_revision,
                report.rule_set_version,
                "diagnosed",
                serde_json::to_string(report)?,
            ],
        )?;
        Ok(())
    }

    pub fn get_diagnosis(&self, id: &str) -> StoreResult<DiagnosisReport> {
        let connection = self.lock();
        let json = connection
            .query_row(
                "SELECT report_json FROM diagnoses WHERE id = ?1",
                params![id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        match json {
            Some(json) => Ok(serde_json::from_str(&json)?),
            None => Err(StoreError::NotFound),
        }
    }

    pub fn get_check_report(&self, check_id: &str) -> StoreResult<CheckReport> {
        let connection = self.lock();
        let json = connection
            .query_row(
                "SELECT report_json FROM check_reports WHERE id = ?1",
                params![check_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        match json {
            Some(json) => Ok(serde_json::from_str(&json)?),
            None => Err(StoreError::NotFound),
        }
    }

    pub fn list_snapshots(&self) -> StoreResult<Vec<SnapshotSummary>> {
        let connection = self.lock();
        let mut statement = connection.prepare(
            "SELECT s.id, s.created_at, s.revision, s.parent_snapshot_id, s.currency,
                    s.source_filename, s.note, s.unit_count, s.value_minor,
                    (SELECT COUNT(*) FROM records r WHERE r.snapshot_id = s.id)
             FROM snapshots s ORDER BY s.created_at DESC, s.rowid DESC",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(SnapshotSummary {
                id: row.get(0)?,
                created_at: row.get(1)?,
                revision: row.get(2)?,
                parent_snapshot_id: row.get(3)?,
                currency: row.get(4)?,
                source_filename: row.get(5)?,
                note: row.get(6)?,
                unit_count: row.get(7)?,
                value_minor: row.get(8)?,
                record_count: row.get::<_, i64>(9)? as usize,
            })
        })?;
        let mut summaries = Vec::new();
        for row in rows {
            summaries.push(row?);
        }
        Ok(summaries)
    }
}

fn insert_snapshot(connection: &Connection, snapshot: &Snapshot) -> StoreResult<()> {
    connection.execute(
        "INSERT INTO snapshots
         (id, created_at, revision, parent_snapshot_id, currency, source_filename,
          source_sha256, note, unit_count, value_minor)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            snapshot.id,
            snapshot.created_at,
            snapshot.revision,
            snapshot.parent_snapshot_id,
            snapshot.currency,
            snapshot.source.filename,
            snapshot.source.sha256,
            snapshot.note,
            snapshot.totals.unit_count,
            snapshot.totals.value_minor,
        ],
    )?;
    let mut statement = connection.prepare(
        "INSERT INTO records
         (id, snapshot_id, position, lot, sku, category, model_raw, model_normalized,
          quantity, price_minor, currency, specs_json, provenance_json, match_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
    )?;
    for (position, record) in snapshot.records.iter().enumerate() {
        statement.execute(params![
            record.id,
            snapshot.id,
            position as i64,
            record.lot,
            record.sku,
            record.category,
            record.model_raw,
            record.model_normalized,
            record.quantity,
            record.price_minor,
            record.currency,
            serde_json::to_string(&record.specs)?,
            serde_json::to_string(&record.provenance)?,
            serde_json::to_string(&record.match_info)?,
        ])?;
    }
    Ok(())
}

fn insert_check_row(connection: &Connection, report: &CheckReport) -> StoreResult<()> {
    connection.execute(
        "INSERT INTO check_reports
         (id, snapshot_id, snapshot_revision, rule_set_version, selection_json,
          overall_status, completeness, checked_at, report_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            report.id,
            report.snapshot_id,
            report.snapshot_revision,
            report.rule_set_version,
            serde_json::to_string(&report.selection)?,
            serde_json::to_string(&report.overall_status)?,
            serde_json::to_string(&report.completeness.status)?,
            report.checked_at,
            serde_json::to_string(report)?,
        ],
    )?;
    Ok(())
}

fn migrate(connection: &Connection) -> StoreResult<()> {
    let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version > SCHEMA_VERSION {
        return Err(StoreError::Sql(rusqlite::Error::InvalidQuery));
    }
    if version < 1 {
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS snapshots (
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
             CREATE TABLE IF NOT EXISTS records (
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
             CREATE INDEX IF NOT EXISTS records_snapshot ON records(snapshot_id, position);
             CREATE TABLE IF NOT EXISTS previews (
                id TEXT PRIMARY KEY,
                created_at TEXT NOT NULL,
                sha256 TEXT NOT NULL,
                preview_json TEXT NOT NULL,
                commit_hash TEXT,
                snapshot_id TEXT REFERENCES snapshots(id)
             );",
        )?;
    }
    if version < 2 {
        // Compatibility checks are derived data. They reference the immutable
        // snapshot revision and rule-set version they were computed against so
        // an older verdict is detectable rather than silently reused.
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS check_reports (
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
             CREATE INDEX IF NOT EXISTS check_reports_snapshot
                ON check_reports(snapshot_id, snapshot_revision, checked_at);",
        )?;
    }
    if version < 3 {
        // Recommendations reference the immutable snapshot revision and the
        // rule/scoring versions they were computed with. Re-opening an older
        // database adds the table without touching existing snapshots, records,
        // previews or check reports.
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS recommendations (
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
             CREATE INDEX IF NOT EXISTS recommendations_snapshot
                ON recommendations(snapshot_id, snapshot_revision, created_at);",
        )?;
    }
    if version < 4 {
        // Diagnoses are derived data: the report embeds the described build and
        // its compatibility evidence, and references the snapshot revision it
        // was computed against. Adding the table preserves every earlier row.
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS diagnoses (
                id TEXT PRIMARY KEY,
                created_at TEXT NOT NULL,
                snapshot_id TEXT NOT NULL REFERENCES snapshots(id),
                snapshot_revision INTEGER NOT NULL,
                rule_set_version TEXT NOT NULL,
                status TEXT NOT NULL,
                report_json TEXT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS diagnoses_snapshot
                ON diagnoses(snapshot_id, snapshot_revision, created_at);",
        )?;
    }
    if version < SCHEMA_VERSION {
        connection.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Provenance, Record, SnapshotSource};

    fn temp_store() -> (tempfile::TempDir, Store) {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(&dir.path().join("inventory.sqlite3")).unwrap();
        (dir, store)
    }

    fn record(id: &str) -> Record {
        let mut provenance = std::collections::BTreeMap::new();
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
            totals: Snapshot::totals_for(&records),
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
        let version: i64 = store
            .lock()
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
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
        use crate::compatibility::{check_build, CheckRequest, SelectionItem};
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
            crate::compatibility::RULE_SET_VERSION
        );
        assert!(matches!(
            store.get_check_report("missing"),
            Err(StoreError::NotFound)
        ));
    }

    #[test]
    fn schema_three_preserves_v2_data_and_adds_recommendations() {
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
                     CREATE TABLE previews (
                        id TEXT PRIMARY KEY,
                        created_at TEXT NOT NULL,
                        sha256 TEXT NOT NULL,
                        preview_json TEXT NOT NULL,
                        commit_hash TEXT,
                        snapshot_id TEXT REFERENCES snapshots(id)
                     );
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
                     INSERT INTO snapshots
                        (id, created_at, revision, currency, source_filename, source_sha256,
                         unit_count, value_minor)
                        VALUES ('legacy', '2026-10-30T00:00:00Z', 1, 'USD', 'legacy.csv',
                                'sha', 1, 100);
                     INSERT INTO records
                        (id, snapshot_id, position, lot, sku, category, model_raw,
                         model_normalized, quantity, price_minor, currency, specs_json,
                         provenance_json, match_json)
                        VALUES ('legacy-r1', 'legacy', 0, 1, 'LEGACY', 'cpu', 'Legacy CPU',
                                'Legacy CPU', 1, 100, 'USD', '[]', '{}',
                                '{\"basis\":\"lexical\",\"confidence\":\"unverified\"}');",
                )
                .unwrap();
            connection.pragma_update(None, "user_version", 2).unwrap();
        }
        let store = Store::open(&path).unwrap();
        let version: i64 = store
            .lock()
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
        let snapshot = store.get_snapshot("legacy").unwrap();
        assert_eq!(snapshot.records.len(), 1);
        assert_eq!(snapshot.records[0].sku, "LEGACY");
        assert!(matches!(
            store.get_recommendation("missing"),
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
        let version: i64 = store
            .lock()
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
        let snapshot = store.get_snapshot("v3").unwrap();
        assert_eq!(snapshot.records[0].sku, "V3");
        assert!(matches!(
            store.get_diagnosis("missing"),
            Err(StoreError::NotFound)
        ));
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
}

//! HTTP contract tests for snapshot export, database backup and guarded restore.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use rigcraft::csv_import::SAMPLE_CSV;
use rigcraft::store::Store;
use rigcraft::{build_router, AppState};
use serde_json::{json, Value};
use tempfile::TempDir;
use tower::ServiceExt;

const BOUNDARY: &str = "rigcraft-portability-boundary";

struct Harness {
    dir: TempDir,
    db_path: PathBuf,
    router: Router,
}

impl Harness {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("inventory.sqlite3");
        let store = Arc::new(Store::open(&db_path).unwrap());
        let router = build_router(AppState::new(store, dir.path().join("missing-web")));
        Self {
            dir,
            db_path,
            router,
        }
    }

    async fn send(&self, request: Request<Body>) -> (StatusCode, axum::http::HeaderMap, Vec<u8>) {
        let response = self.router.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let headers = response.headers().clone();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (status, headers, bytes.to_vec())
    }

    async fn json(&self, request: Request<Body>) -> (StatusCode, Value) {
        let (status, _, bytes) = self.send(request).await;
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    async fn get(&self, uri: &str) -> (StatusCode, Value) {
        let request = Request::builder()
            .method("GET")
            .uri(uri)
            .body(Body::empty())
            .unwrap();
        self.json(request).await
    }

    async fn get_bytes(&self, uri: &str) -> (StatusCode, axum::http::HeaderMap, Vec<u8>) {
        let request = Request::builder()
            .method("GET")
            .uri(uri)
            .body(Body::empty())
            .unwrap();
        self.send(request).await
    }

    async fn post_backup(
        &self,
        uri: &str,
        file_name: &str,
        bytes: &[u8],
        confirm: Option<&str>,
    ) -> (StatusCode, Value) {
        let body = backup_multipart(file_name, bytes, confirm);
        let request = Request::builder()
            .method("POST")
            .uri(uri)
            .header(
                header::CONTENT_TYPE,
                format!("multipart/form-data; boundary={BOUNDARY}"),
            )
            .body(Body::from(body))
            .unwrap();
        self.json(request).await
    }

    async fn seed_sample(&self) -> Value {
        let preview = self
            .preview("sample.csv", SAMPLE_CSV.as_bytes(), "USD")
            .await;
        let preview_id = preview["previewId"].as_str().unwrap();
        let decisions = json!({
            "decisions": preview["rows"].as_array().unwrap().iter().map(|row| json!({
                "row": row["row"],
                "action": "accept",
            })).collect::<Vec<_>>(),
            "duplicateResolutions": [],
        });
        let request = Request::builder()
            .method("POST")
            .uri(format!("/api/imports/{preview_id}/commit"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(decisions.to_string()))
            .unwrap();
        let (status, committed) = self.json(request).await;
        assert_eq!(status, StatusCode::OK, "{committed}");
        committed["snapshot"].clone()
    }

    async fn preview(&self, filename: &str, csv: &[u8], currency: &str) -> Value {
        let body = import_multipart(filename, csv, currency);
        let request = Request::builder()
            .method("POST")
            .uri("/api/imports")
            .header(
                header::CONTENT_TYPE,
                format!("multipart/form-data; boundary={BOUNDARY}"),
            )
            .body(Body::from(body))
            .unwrap();
        let (status, preview) = self.json(request).await;
        assert_eq!(status, StatusCode::OK, "{preview}");
        preview
    }
}

fn import_multipart(filename: &str, csv: &[u8], currency: &str) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(
        format!(
            "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\nContent-Type: text/csv\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(csv);
    body.extend_from_slice(
        format!(
            "\r\n--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"currency\"\r\n\r\n{currency}\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(format!("--{BOUNDARY}--\r\n").as_bytes());
    body
}

fn backup_multipart(filename: &str, bytes: &[u8], confirm: Option<&str>) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(
        format!(
            "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\nContent-Type: application/vnd.sqlite3\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(bytes);
    if let Some(confirm) = confirm {
        body.extend_from_slice(
            format!(
                "\r\n--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"confirm\"\r\n\r\n{confirm}\r\n"
            )
            .as_bytes(),
        );
    }
    body.extend_from_slice(format!("\r\n--{BOUNDARY}--\r\n").as_bytes());
    body
}

fn header_text(headers: &axum::http::HeaderMap, name: &str) -> String {
    headers
        .get(name)
        .map(|value| value.to_str().unwrap_or_default().to_string())
        .unwrap_or_default()
}

fn pre_restore_files(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("inventory.sqlite3.pre-restore-"))
        })
        .collect()
}

#[tokio::test]
async fn csv_export_reimports_to_the_same_records_and_provenance_sources() {
    let harness = Harness::new();
    let original = harness.seed_sample().await;
    let snapshot_id = original["id"].as_str().unwrap();

    let (status, headers, bytes) = harness
        .get_bytes(&format!(
            "/api/inventory/snapshots/{snapshot_id}/export?format=csv"
        ))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(header_text(&headers, "content-type").starts_with("text/csv"));
    assert_eq!(header_text(&headers, "x-rigcraft-export-lossy"), "true");
    assert!(header_text(&headers, "x-rigcraft-export-note").contains("lossy"));
    let disposition = header_text(&headers, "content-disposition");
    assert!(
        disposition.contains("rigcraft-inventory-snapshot-r1-"),
        "{disposition}"
    );
    assert!(disposition.ends_with(".csv\""), "{disposition}");

    let csv = String::from_utf8(bytes).unwrap();
    let mut lines = csv.lines();
    assert_eq!(
        lines.next().unwrap(),
        "sku,category,model,quantity,price,currency,specs,source,sourceUrl"
    );
    assert_eq!(lines.count(), 10);

    let preview = harness
        .preview("round-trip.csv", csv.as_bytes(), "USD")
        .await;
    assert_eq!(preview["summary"]["errors"], 0, "{preview}");
    assert_eq!(preview["summary"]["total"], 10);
    let preview_id = preview["previewId"].as_str().unwrap();
    let decisions = json!({
        "decisions": preview["rows"].as_array().unwrap().iter().map(|row| json!({
            "row": row["row"],
            "action": "accept",
        })).collect::<Vec<_>>(),
        "duplicateResolutions": [],
    });
    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/imports/{preview_id}/commit"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(decisions.to_string()))
        .unwrap();
    let (status, committed) = harness.json(request).await;
    assert_eq!(status, StatusCode::OK, "{committed}");
    let reimported = &committed["snapshot"];

    let original_records = original["records"].as_array().unwrap();
    let reimported_records = reimported["records"].as_array().unwrap();
    assert_eq!(original_records.len(), reimported_records.len());
    let by_sku = |records: &[Value]| {
        records
            .iter()
            .map(|record| (record["sku"].as_str().unwrap().to_string(), record.clone()))
            .collect::<std::collections::BTreeMap<_, _>>()
    };
    let before_by_sku = by_sku(original_records);
    let after_by_sku = by_sku(reimported_records);
    assert_eq!(before_by_sku.len(), after_by_sku.len());
    for (sku, before) in &before_by_sku {
        let after = after_by_sku
            .get(sku)
            .unwrap_or_else(|| panic!("missing {sku}"));
        for field in [
            "sku",
            "category",
            "modelRaw",
            "modelNormalized",
            "quantity",
            "priceMinor",
            "currency",
        ] {
            assert_eq!(before[field], after[field], "{sku} {field}");
        }
        let before_specs = before["specs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|spec| spec["raw"].as_str().unwrap().to_string())
            .collect::<Vec<_>>();
        let after_specs = after["specs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|spec| spec["raw"].as_str().unwrap().to_string())
            .collect::<Vec<_>>();
        assert_eq!(before_specs, after_specs, "{sku} specs");
        for field in ["model", "specs"] {
            assert_eq!(
                before["provenance"][field]["source"], after["provenance"][field]["source"],
                "{sku} {field} provenance source"
            );
        }
        for field in ["sku", "quantity", "price", "currency"] {
            assert_eq!(
                after["provenance"][field]["source"], "user",
                "{sku} {field}"
            );
        }
    }
}

#[tokio::test]
async fn a_formula_leading_csv_cell_is_neutralised_while_json_keeps_the_raw_value() {
    let harness = Harness::new();
    let csv = "sku,category,model,quantity,price,currency,specs\n\
               X-1,cpu,=1+1,1,10.00,USD,socket=AM5\n\
               X-2,cpu,AMD Ryzen 7 7800X3D,1,20.00,USD,\n";
    let preview = harness.preview("formula.csv", csv.as_bytes(), "USD").await;
    assert_eq!(preview["summary"]["errors"], 0, "{preview}");
    let preview_id = preview["previewId"].as_str().unwrap();
    let decisions = json!({
        "decisions": preview["rows"].as_array().unwrap().iter().map(|row| json!({
            "row": row["row"],
            "action": "accept",
        })).collect::<Vec<_>>(),
        "duplicateResolutions": [],
    });
    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/imports/{preview_id}/commit"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(decisions.to_string()))
        .unwrap();
    let (status, committed) = harness.json(request).await;
    assert_eq!(status, StatusCode::OK, "{committed}");
    let snapshot_id = committed["snapshot"]["id"].as_str().unwrap();

    let (status, _headers, bytes) = harness
        .get_bytes(&format!(
            "/api/inventory/snapshots/{snapshot_id}/export?format=csv"
        ))
        .await;
    assert_eq!(status, StatusCode::OK);
    let csv = String::from_utf8(bytes).unwrap();
    let mut lines = csv.lines();
    lines.next().unwrap();
    // The apostrophe is the only change to the cell and to the line.
    assert_eq!(
        lines.next().unwrap(),
        "X-1,cpu,'=1+1,1,10.00,USD,socket=AM5,user,"
    );
    // A value with no formula trigger is untouched.
    assert_eq!(
        lines.next().unwrap(),
        "X-2,cpu,AMD Ryzen 7 7800X3D,1,20.00,USD,,user,"
    );

    let (status, _headers, bytes) = harness
        .get_bytes(&format!(
            "/api/inventory/snapshots/{snapshot_id}/export?format=json"
        ))
        .await;
    assert_eq!(status, StatusCode::OK);
    let export: Value = serde_json::from_slice(&bytes).unwrap();
    let records = export["snapshot"]["records"].as_array().unwrap();
    assert_eq!(records[0]["modelRaw"], "=1+1");
    assert_eq!(records[1]["modelRaw"], "AMD Ryzen 7 7800X3D");

    // The neutralisation is lossy: a re-import keeps the apostrophe.
    let reimported = harness.preview("formula.csv", csv.as_bytes(), "USD").await;
    assert_eq!(reimported["rows"][0]["values"]["model"], "'=1+1");
}

#[tokio::test]
async fn json_export_round_trips_lots_and_per_field_provenance() {
    let harness = Harness::new();
    let snapshot = harness.seed_sample().await;
    let snapshot_id = snapshot["id"].as_str().unwrap();

    let first = harness
        .get_bytes(&format!(
            "/api/inventory/snapshots/{snapshot_id}/export?format=json"
        ))
        .await;
    assert_eq!(first.0, StatusCode::OK);
    assert!(header_text(&first.1, "content-type").starts_with("application/json"));
    assert!(header_text(&first.1, "content-disposition").contains("-r1-"));

    let export: Value = serde_json::from_slice(&first.2).unwrap();
    assert_eq!(export["format"], "rigcraft.inventory.snapshot");
    assert_eq!(export["formatVersion"], 1);
    assert_eq!(export["authoritative"], true);
    assert_eq!(export["lossy"], false);
    assert_eq!(export["snapshot"]["id"], snapshot_id);
    assert_eq!(export["snapshot"]["revision"], 1);
    assert_eq!(export["snapshot"]["parentSnapshotId"], Value::Null);
    let records = export["snapshot"]["records"].as_array().unwrap();
    assert_eq!(records.len(), 10);
    let cpu = records
        .iter()
        .find(|record| record["sku"] == "CPU-7800X3D")
        .unwrap();
    assert_eq!(
        cpu["provenance"]["model"]["source"],
        "catalog:manufacturer@2026-10-31"
    );
    assert_eq!(
        cpu["specs"][0]["provenance"]["source"],
        "catalog:manufacturer@2026-10-31"
    );
    assert_eq!(cpu["provenance"]["quantity"]["source"], "user");
    assert_eq!(cpu["provenance"]["price"]["reference"], "csv:sample.csv");

    let second = harness
        .get_bytes(&format!(
            "/api/inventory/snapshots/{snapshot_id}/export?format=json"
        ))
        .await;
    assert_eq!(first.2, second.2, "the export is deterministic");
}

#[tokio::test]
async fn unknown_snapshot_and_unsupported_format_use_the_error_envelope() {
    let harness = Harness::new();
    let (status, body) = harness
        .get("/api/inventory/snapshots/missing/export?format=csv")
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"]["code"], "snapshot_not_found");

    let snapshot = harness.seed_sample().await;
    let snapshot_id = snapshot["id"].as_str().unwrap();
    let (status, body) = harness
        .get(&format!(
            "/api/inventory/snapshots/{snapshot_id}/export?format=xml"
        ))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "export_format_unsupported");
    assert_eq!(body["error"]["field"], "format");
}

#[tokio::test]
async fn backup_is_a_valid_sqlite_file_with_the_same_counts() {
    let harness = Harness::new();
    harness.seed_sample().await;
    let (status, headers, bytes) = harness.get_bytes("/api/backup").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        header_text(&headers, "content-type"),
        "application/vnd.sqlite3"
    );
    let disposition = header_text(&headers, "content-disposition");
    assert!(disposition.contains("rigcraft-backup-"), "{disposition}");
    assert!(disposition.ends_with(".sqlite3\""), "{disposition}");

    let path = harness.dir.path().join("downloaded.sqlite3");
    std::fs::write(&path, &bytes).unwrap();
    let connection = rusqlite::Connection::open(&path).unwrap();
    let integrity: String = connection
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .unwrap();
    assert_eq!(integrity, "ok");
    let version: i64 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(version, 5);
    let snapshots: i64 = connection
        .query_row("SELECT COUNT(*) FROM snapshots", [], |row| row.get(0))
        .unwrap();
    let records: i64 = connection
        .query_row("SELECT COUNT(*) FROM records", [], |row| row.get(0))
        .unwrap();
    assert_eq!(snapshots, 1);
    assert_eq!(records, 10);
}

#[tokio::test]
async fn restore_reproduces_counts_and_writes_a_pre_restore_copy() {
    let harness = Harness::new();
    let original = harness.seed_sample().await;
    let original_id = original["id"].as_str().unwrap();

    let (_, _, backup) = harness.get_bytes("/api/backup").await;

    // Change the live catalog after the backup so a restore is observable.
    let request = Request::builder()
        .method("POST")
        .uri(format!(
            "/api/inventory/snapshots/{original_id}/corrections"
        ))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "note": "after backup",
                "changes": [{ "recordId": original["records"][0]["id"], "quantity": 3 }],
            })
            .to_string(),
        ))
        .unwrap();
    let (status, corrected) = harness.json(request).await;
    assert_eq!(status, StatusCode::OK, "{corrected}");
    let (_, before) = harness.get("/api/inventory/snapshots").await;
    assert_eq!(before.as_array().unwrap().len(), 2);

    let (status, inspection) = harness
        .post_backup("/api/backup/inspect", "backup.sqlite3", &backup, None)
        .await;
    assert_eq!(status, StatusCode::OK, "{inspection}");
    assert_eq!(inspection["integrity"], "ok");
    assert_eq!(inspection["counts"]["schemaVersion"], 5);
    assert_eq!(inspection["counts"]["snapshotCount"], 1);
    assert_eq!(inspection["counts"]["latestRevision"], 1);
    assert_eq!(inspection["counts"]["recordCount"], 10);
    assert_eq!(inspection["counts"]["lotCount"], 10);

    let (status, summary) = harness
        .post_backup(
            "/api/backup/restore",
            "backup.sqlite3",
            &backup,
            Some("restore"),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{summary}");
    assert_eq!(summary["restored"], true);
    assert_eq!(summary["sourceSchemaVersion"], 5);
    assert_eq!(summary["counts"]["schemaVersion"], 5);
    assert_eq!(summary["counts"]["snapshotCount"], 1);
    assert_eq!(summary["counts"]["recordCount"], 10);
    assert_eq!(summary["counts"]["lotCount"], 10);

    let pre_restore = summary["preRestorePath"].as_str().unwrap();
    assert!(Path::new(pre_restore).exists(), "{pre_restore} must exist");
    let safety = rusqlite::Connection::open(pre_restore).unwrap();
    let safety_snapshots: i64 = safety
        .query_row("SELECT COUNT(*) FROM snapshots", [], |row| row.get(0))
        .unwrap();
    assert_eq!(safety_snapshots, 2, "the safety copy holds the later state");

    let (_, after) = harness.get("/api/inventory/snapshots").await;
    assert_eq!(after.as_array().unwrap().len(), 1);
    let (status, restored) = harness
        .get(&format!("/api/inventory/snapshots/{original_id}"))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(restored["records"].as_array().unwrap().len(), 10);
}

#[tokio::test]
async fn rejected_restores_return_their_own_codes_and_leave_data_intact() {
    let harness = Harness::new();
    harness.seed_sample().await;
    let (_, _, good_backup) = harness.get_bytes("/api/backup").await;

    let mut corrupt = good_backup.clone();
    for byte in &mut corrupt[100..200] {
        *byte = 0xFF;
    }
    let not_sqlite = b"this is not a sqlite database".to_vec();
    let empty: Vec<u8> = Vec::new();

    let newer_path = harness.dir.path().join("newer.sqlite3");
    {
        let connection = rusqlite::Connection::open(&newer_path).unwrap();
        connection
            .execute_batch("CREATE TABLE unrelated (x INTEGER);")
            .unwrap();
        connection.pragma_update(None, "user_version", 99).unwrap();
    }
    let newer = std::fs::read(&newer_path).unwrap();

    let wrong_path = harness.dir.path().join("wrong-shape.sqlite3");
    {
        let connection = rusqlite::Connection::open(&wrong_path).unwrap();
        connection
            .execute_batch("CREATE TABLE unrelated (x INTEGER);")
            .unwrap();
        connection.pragma_update(None, "user_version", 4).unwrap();
    }
    let wrong = std::fs::read(&wrong_path).unwrap();

    for (file_name, bytes, expected_code, expected_status) in [
        (
            "corrupt.sqlite3",
            corrupt.as_slice(),
            "backup_corrupt",
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            "newer.sqlite3",
            newer.as_slice(),
            "backup_schema_too_new",
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            "not-sqlite.txt",
            not_sqlite.as_slice(),
            "backup_not_sqlite",
            StatusCode::BAD_REQUEST,
        ),
        (
            "empty.sqlite3",
            empty.as_slice(),
            "backup_empty",
            StatusCode::BAD_REQUEST,
        ),
        (
            "wrong-shape.sqlite3",
            wrong.as_slice(),
            "backup_wrong_shape",
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
    ] {
        let (status, body) = harness
            .post_backup("/api/backup/inspect", file_name, bytes, None)
            .await;
        assert_eq!(status, expected_status, "inspect {file_name}: {body}");
        assert_eq!(body["error"]["code"], expected_code, "inspect {file_name}");

        let (status, body) = harness
            .post_backup("/api/backup/restore", file_name, bytes, Some("restore"))
            .await;
        assert_eq!(status, expected_status, "restore {file_name}: {body}");
        assert_eq!(body["error"]["code"], expected_code, "restore {file_name}");
    }

    let (_, snapshots) = harness.get("/api/inventory/snapshots").await;
    assert_eq!(
        snapshots.as_array().unwrap().len(),
        1,
        "no rejection replaced live data"
    );
    assert!(
        pre_restore_files(harness.dir.path()).is_empty(),
        "a rejected restore must not write a safety copy"
    );

    let (status, body) = harness
        .post_backup("/api/backup/restore", "backup.sqlite3", &good_backup, None)
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "backup_confirm_required");
    assert!(pre_restore_files(harness.dir.path()).is_empty());
}

#[tokio::test]
async fn an_older_schema_migrates_forward_on_restore() {
    let harness = Harness::new();
    harness.seed_sample().await;
    let old_path = harness.dir.path().join("legacy-v1.sqlite3");
    {
        let connection = rusqlite::Connection::open(&old_path).unwrap();
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
                 INSERT INTO snapshots
                    (id, created_at, revision, currency, source_filename, source_sha256,
                     unit_count, value_minor)
                    VALUES ('legacy', '2026-10-30T00:00:00Z', 1, 'USD', 'legacy.csv',
                            'sha', 2, 200);
                 INSERT INTO records
                    (id, snapshot_id, position, lot, sku, category, model_raw,
                     model_normalized, quantity, price_minor, currency, specs_json,
                     provenance_json, match_json)
                    VALUES ('legacy-r1', 'legacy', 0, 1, 'LEGACY', 'cpu', 'Legacy CPU',
                            'Legacy CPU', 2, 100, 'USD', '[]', '{}',
                            '{\"basis\":\"lexical\",\"confidence\":\"unverified\"}');",
            )
            .unwrap();
        connection.pragma_update(None, "user_version", 1).unwrap();
    }
    let legacy = std::fs::read(&old_path).unwrap();

    let (status, summary) = harness
        .post_backup(
            "/api/backup/restore",
            "legacy-v1.sqlite3",
            &legacy,
            Some("restore"),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{summary}");
    assert_eq!(summary["sourceSchemaVersion"], 1);
    assert_eq!(summary["counts"]["schemaVersion"], 5);
    assert_eq!(summary["counts"]["snapshotCount"], 1);
    assert_eq!(summary["counts"]["recordCount"], 1);

    let (status, snapshot) = harness.get("/api/inventory/snapshots/legacy").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(snapshot["records"][0]["sku"], "LEGACY");

    let connection = rusqlite::Connection::open(&harness.db_path).unwrap();
    let tables: Vec<String> = connection
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    for table in ["check_reports", "recommendations", "diagnoses"] {
        assert!(tables.iter().any(|name| name == table), "missing {table}");
    }
}

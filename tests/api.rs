//! End-to-end HTTP contract tests for the inventory import package.

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

struct Harness {
    _dir: TempDir,
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
            _dir: dir,
            db_path,
            router,
        }
    }

    async fn json(&self, request: Request<Body>) -> (StatusCode, Value) {
        let response = self.router.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        (status, value)
    }

    async fn text(&self, request: Request<Body>) -> (StatusCode, String, Option<String>) {
        let response = self.router.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .map(|value| value.to_str().unwrap().to_string());
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (
            status,
            String::from_utf8_lossy(&bytes).to_string(),
            content_type,
        )
    }

    async fn preview(&self, filename: &str, csv: &[u8], currency: &str) -> (StatusCode, Value) {
        let body = multipart_body(filename, csv, currency, None);
        let request = Request::builder()
            .method("POST")
            .uri("/api/imports")
            .header(
                header::CONTENT_TYPE,
                "multipart/form-data; boundary=rigcraft-test-boundary",
            )
            .body(Body::from(body))
            .unwrap();
        self.json(request).await
    }

    async fn commit(&self, preview_id: &str, decisions: &Value) -> (StatusCode, Value) {
        let request = Request::builder()
            .method("POST")
            .uri(format!("/api/imports/{preview_id}/commit"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(decisions.to_string()))
            .unwrap();
        self.json(request).await
    }

    async fn get(&self, uri: &str) -> (StatusCode, Value) {
        let request = Request::builder()
            .method("GET")
            .uri(uri)
            .body(Body::empty())
            .unwrap();
        self.json(request).await
    }

    async fn correct(&self, snapshot_id: &str, request: &Value) -> (StatusCode, Value) {
        let request = Request::builder()
            .method("POST")
            .uri(format!(
                "/api/inventory/snapshots/{snapshot_id}/corrections"
            ))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(request.to_string()))
            .unwrap();
        self.json(request).await
    }
}

fn multipart_body(filename: &str, csv: &[u8], currency: &str, mapping: Option<&str>) -> Vec<u8> {
    let boundary = "rigcraft-test-boundary";
    let mut body = Vec::new();
    body.extend_from_slice(
        format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\nContent-Type: text/csv\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(csv);
    body.extend_from_slice(
        format!(
            "\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"currency\"\r\n\r\n{currency}\r\n"
        )
        .as_bytes(),
    );
    if let Some(mapping) = mapping {
        body.extend_from_slice(
            format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"mapping\"\r\n\r\n{mapping}\r\n"
            )
            .as_bytes(),
        );
    }
    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    body
}

fn fixture(path: &str) -> Vec<u8> {
    std::fs::read(Path::new("tests/fixtures/inventory").join(path)).unwrap()
}

fn all_accept(preview: &Value) -> Value {
    let rows = preview["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            let action = if row["status"] == "error" {
                "exclude"
            } else {
                "accept"
            };
            json!({ "row": row["row"], "action": action })
        })
        .collect::<Vec<_>>();
    json!({ "decisions": rows, "duplicateResolutions": [] })
}

#[tokio::test]
async fn preview_returns_rows_mapping_and_provenance() {
    let harness = Harness::new();
    let (status, preview) = harness
        .preview("sample.csv", SAMPLE_CSV.as_bytes(), "usd")
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(preview["currency"], "USD");
    assert_eq!(preview["summary"]["total"], 10);
    assert_eq!(preview["summary"]["errors"], 0);
    assert_eq!(preview["rows"][0]["normalized"]["priceMinor"], 44900);
    assert_eq!(preview["rows"][0]["specs"][0]["status"], "confirmed");
    assert_eq!(preview["rows"][0]["provenance"]["sku"]["source"], "user");
    assert_eq!(
        preview["rows"][0]["provenance"]["model"]["source"],
        "catalog:manufacturer@2026-10-31"
    );
    assert!(preview["rows"][0]["provenance"]["specs"]["reference"]
        .as_str()
        .unwrap()
        .starts_with("https://"));
    assert_eq!(
        preview["rows"][0]["provenance"]["modelNormalized"]["source"],
        "rigcraft-normalizer"
    );
    assert_eq!(
        preview["rows"][0]["match"]["confidence"],
        "catalog-declared"
    );
    assert!(preview["previewId"].as_str().unwrap().len() > 10);
}

#[tokio::test]
async fn source_columns_flow_from_preview_to_snapshot_without_touching_stock_provenance() {
    let harness = Harness::new();
    let csv = "sku,category,model,quantity,price,currency,specs,source,sourceUrl\n\
SRC-1,cpu,Catalog CPU,2,10.00,USD,socket=AM5;tdp=120W,catalog:buildcores@abc1234,https://example.test/cpu\n";
    let (status, preview) = harness.preview("catalog.csv", csv.as_bytes(), "USD").await;
    assert_eq!(status, StatusCode::OK, "{preview}");
    assert_eq!(preview["summary"]["errors"], 0);
    assert_eq!(
        preview["rows"][0]["provenance"]["model"]["source"],
        "catalog:buildcores@abc1234"
    );
    assert_eq!(
        preview["rows"][0]["provenance"]["specs"]["reference"],
        "https://example.test/cpu"
    );
    assert_eq!(
        preview["rows"][0]["provenance"]["quantity"]["source"],
        "user"
    );
    assert_eq!(
        preview["rows"][0]["match"]["confidence"],
        "catalog-declared"
    );

    let preview_id = preview["previewId"].as_str().unwrap();
    let (status, committed) = harness.commit(preview_id, &all_accept(&preview)).await;
    assert_eq!(status, StatusCode::OK, "{committed}");
    let record = &committed["snapshot"]["records"][0];
    assert_eq!(
        record["provenance"]["model"]["source"],
        "catalog:buildcores@abc1234"
    );
    assert_eq!(
        record["specs"][0]["provenance"]["source"],
        "catalog:buildcores@abc1234"
    );
    assert_eq!(record["provenance"]["sku"]["source"], "user");
    assert_eq!(record["provenance"]["price"]["source"], "user");
}

#[tokio::test]
async fn csv_without_source_columns_keeps_user_provenance() {
    let harness = Harness::new();
    let csv = "sku,category,model,quantity,price\nPLAIN-1,cpu,Plain CPU,1,10\n";
    let (status, preview) = harness.preview("plain.csv", csv.as_bytes(), "USD").await;
    assert_eq!(status, StatusCode::OK, "{preview}");
    assert_eq!(preview["rows"][0]["provenance"]["model"]["source"], "user");
    assert_eq!(
        preview["rows"][0]["provenance"]["model"]["reference"],
        "csv:plain.csv"
    );
    assert_eq!(preview["rows"][0]["match"]["confidence"], "unverified");
}

#[tokio::test]
async fn invalid_source_values_are_field_errors() {
    let harness = Harness::new();
    let csv = "sku,category,model,quantity,price,source,sourceUrl\n\
BAD-1,cpu,Bad Source,1,10,shop,https://example.test\n\
BAD-2,cpu,Bad URL,1,10,catalog:buildcores@abc1234,ftp://example.test\n";
    let (status, preview) = harness.preview("bad.csv", csv.as_bytes(), "USD").await;
    assert_eq!(status, StatusCode::OK, "{preview}");
    assert_eq!(preview["summary"]["errors"], 2);
    assert_eq!(preview["rows"][0]["errors"][0]["code"], "source_format");
    assert_eq!(preview["rows"][1]["errors"][0]["code"], "source_url_format");
    assert_eq!(preview["rows"][1]["errors"][0]["field"], "sourceUrl");
}

#[tokio::test]
async fn mixed_fixture_names_every_bad_row_and_duplicate_group() {
    let harness = Harness::new();
    let (status, preview) = harness
        .preview("mixed.csv", &fixture("mixed.csv"), "USD")
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(preview["summary"]["total"], 6);
    assert_eq!(preview["summary"]["errors"], 3);
    assert_eq!(preview["summary"]["duplicateRows"], 2);
    let groups = preview["duplicateGroups"].as_array().unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0]["rows"], json!([2, 3]));
    let errors = preview["rows"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|row| row["errors"].as_array().unwrap().iter())
        .map(|error| error["code"].as_str().unwrap().to_string())
        .collect::<Vec<_>>();
    assert!(errors.contains(&"quantity_format".to_string()));
    assert!(errors.contains(&"required".to_string()));
    assert!(errors.contains(&"currency_mismatch".to_string()));
    // The row number is the file line, so the operator can open the CSV at it.
    assert_eq!(preview["rows"][0]["row"], 2);
}

#[tokio::test]
async fn a_semicolon_delimited_header_names_the_likely_delimiter() {
    let harness = Harness::new();
    let csv = "sku;category;model;quantity;price;currency\n\
               X-1;cpu;Thing;1;10.00;USD\n";
    let (status, error) = harness
        .preview("semicolon.csv", csv.as_bytes(), "USD")
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["error"]["code"], "mapping_missing");
    assert_eq!(error["error"]["field"], "sku");
    assert_eq!(
        error["error"]["details"]["missing"],
        json!(["sku", "category", "model", "quantity", "price"])
    );
    assert_eq!(error["error"]["details"]["delimiter"], "semicolon");
    let message = error["error"]["message"].as_str().unwrap();
    assert!(
        message.contains("semicolon-delimited"),
        "the message names the delimiter: {message}"
    );
    assert!(
        message.contains("comma-separated"),
        "the message names what is expected: {message}"
    );
    // It names the cause, it does not echo the file.
    assert!(!message.contains("X-1"));
    assert!(!message.contains("Thing"));
}

#[tokio::test]
async fn a_comma_csv_with_a_genuine_missing_column_gets_no_delimiter_hint() {
    let harness = Harness::new();
    let csv = "sku,model,quantity,price,currency\nX-1,Thing,1,10.00,USD\n";
    let (status, error) = harness.preview("missing.csv", csv.as_bytes(), "USD").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["error"]["code"], "mapping_missing");
    assert_eq!(error["error"]["field"], "category");
    assert!(error["error"]["details"]["delimiter"].is_null());
    assert!(!error["error"]["message"]
        .as_str()
        .unwrap()
        .contains("semicolon"));
}

#[tokio::test]
async fn non_utf8_and_oversized_files_are_rejected() {
    let harness = Harness::new();
    let (status, error) = harness.preview("bad.csv", &[0xff, 0xfe], "USD").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["error"]["code"], "not_utf8");

    let oversized = vec![b'a'; 10 * 1024 * 1024 + 1];
    let (status, error) = harness.preview("huge.csv", &oversized, "USD").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["error"]["code"], "file_too_large");
    assert_eq!(error["error"]["field"], "file");
}

#[tokio::test]
async fn commit_requires_duplicate_resolution_and_rejects_invalid_rows() {
    let harness = Harness::new();
    let (_, preview) = harness
        .preview("mixed.csv", &fixture("mixed.csv"), "USD")
        .await;
    let preview_id = preview["previewId"].as_str().unwrap();

    let (status, error) = harness.commit(preview_id, &all_accept(&preview)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(error["error"]["code"], "duplicate_unresolved");

    let mut request = all_accept(&preview);
    request["decisions"][2]["action"] = json!("accept");
    let (status, error) = harness.commit(preview_id, &request).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(error["error"]["code"], "invalid_row");
    assert_eq!(error["error"]["details"]["rows"][0]["row"], 4);
}

#[tokio::test]
async fn commit_is_idempotent_and_rejects_a_different_decision_set() {
    let harness = Harness::new();
    let (_, preview) = harness
        .preview("mixed.csv", &fixture("mixed.csv"), "USD")
        .await;
    let preview_id = preview["previewId"].as_str().unwrap();
    let mut request = all_accept(&preview);
    request["duplicateResolutions"] = json!([{ "groupId": "dup-1", "action": "merge" }]);

    let (status, committed) = harness.commit(preview_id, &request).await;
    assert_eq!(status, StatusCode::OK);
    let snapshot_id = committed["snapshotId"].as_str().unwrap().to_string();
    assert_eq!(committed["snapshot"]["totals"]["recordCount"], 2);
    assert_eq!(committed["snapshot"]["totals"]["unitCount"], 4);
    assert_eq!(committed["snapshot"]["totals"]["valueMinor"], 67600);
    let merged = committed["snapshot"]["records"]
        .as_array()
        .unwrap()
        .iter()
        .find(|record| record["sku"] == "CPU-1")
        .unwrap();
    assert_eq!(merged["quantity"], 3);
    assert_eq!(merged["provenance"]["sku"]["reference"], "csv:mixed.csv");

    let (status, replay) = harness.commit(preview_id, &request).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(replay["snapshotId"], snapshot_id);

    let mut changed = request.clone();
    changed["duplicateResolutions"][0]["action"] = json!("keep_separate");
    let (status, error) = harness.commit(preview_id, &changed).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(error["error"]["code"], "preview_already_committed");

    let (status, snapshots) = harness.get("/api/inventory/snapshots").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(snapshots.as_array().unwrap().len(), 1);
    assert_eq!(snapshots[0]["id"], snapshot_id);
}

#[tokio::test]
async fn a_correction_creates_a_new_revision_and_the_old_snapshot_is_unchanged() {
    let harness = Harness::new();
    let (_, preview) = harness
        .preview("sample.csv", SAMPLE_CSV.as_bytes(), "USD")
        .await;
    let preview_id = preview["previewId"].as_str().unwrap();
    let (status, committed) = harness.commit(preview_id, &all_accept(&preview)).await;
    assert_eq!(status, StatusCode::OK);
    let snapshot = &committed["snapshot"];
    let snapshot_id = snapshot["id"].as_str().unwrap().to_string();
    let record_id = snapshot["records"][0]["id"].as_str().unwrap().to_string();
    let original_quantity = snapshot["records"][0]["quantity"].as_i64().unwrap();

    let (status, corrected) = harness
        .correct(
            &snapshot_id,
            &json!({
                "note": "Stock recount",
                "changes": [{ "recordId": record_id, "quantity": 7 }]
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(corrected["revision"], 2);
    assert_eq!(corrected["parentSnapshotId"], snapshot_id);
    assert_eq!(corrected["records"][0]["quantity"], 7);
    assert_ne!(corrected["records"][0]["id"], record_id);
    assert_eq!(corrected["note"], "Stock recount");
    assert_eq!(
        corrected["records"][0]["provenance"]["quantity"]["reference"],
        format!("correction:{}", corrected["id"].as_str().unwrap())
    );

    let (status, original) = harness
        .get(&format!("/api/inventory/snapshots/{snapshot_id}"))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(original["records"][0]["quantity"], original_quantity);
    assert_eq!(original["revision"], 1);

    let (_, snapshots) = harness.get("/api/inventory/snapshots").await;
    assert_eq!(snapshots.as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn expired_preview_and_unknown_ids_have_stable_codes() {
    let harness = Harness::new();
    let (_, preview) = harness
        .preview("sample.csv", SAMPLE_CSV.as_bytes(), "USD")
        .await;
    let preview_id = preview["previewId"].as_str().unwrap().to_string();
    let connection = rusqlite::Connection::open(&harness.db_path).unwrap();
    connection
        .execute(
            "UPDATE previews SET created_at = '2020-01-01T00:00:00Z' WHERE id = ?1",
            [preview_id.clone()],
        )
        .unwrap();
    let (status, error) = harness.commit(&preview_id, &all_accept(&preview)).await;
    assert_eq!(status, StatusCode::GONE);
    assert_eq!(error["error"]["code"], "preview_expired");

    let (status, error) = harness.get("/api/inventory/snapshots/does-not-exist").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(error["error"]["code"], "snapshot_not_found");

    let (status, error) = harness
        .commit("does-not-exist", &json!({ "decisions": [] }))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(error["error"]["code"], "preview_not_found");
}

#[tokio::test]
async fn a_committed_preview_replays_after_expiry_and_rejects_other_decisions() {
    let harness = Harness::new();
    let (_, preview) = harness
        .preview("sample.csv", SAMPLE_CSV.as_bytes(), "USD")
        .await;
    let preview_id = preview["previewId"].as_str().unwrap().to_string();
    let request = all_accept(&preview);
    let (status, committed) = harness.commit(&preview_id, &request).await;
    assert_eq!(status, StatusCode::OK);
    let snapshot_id = committed["snapshotId"].as_str().unwrap().to_string();

    let connection = rusqlite::Connection::open(&harness.db_path).unwrap();
    connection
        .execute(
            "UPDATE previews SET created_at = '2020-01-01T00:00:00Z' WHERE id = ?1",
            [preview_id.clone()],
        )
        .unwrap();

    let (status, replay) = harness.commit(&preview_id, &request).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(replay["snapshotId"], snapshot_id);

    let mut changed = request.clone();
    changed["decisions"][0]["action"] = json!("exclude");
    let (status, error) = harness.commit(&preview_id, &changed).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(error["error"]["code"], "preview_already_committed");
}

#[tokio::test]
async fn sample_endpoint_serves_a_valid_download() {
    let harness = Harness::new();
    let request = Request::builder()
        .method("GET")
        .uri("/api/imports/sample")
        .body(Body::empty())
        .unwrap();
    let (status, body, content_type) = harness.text(request).await;
    assert_eq!(status, StatusCode::OK);
    assert!(content_type.unwrap().starts_with("text/csv"));
    assert_eq!(body.lines().count(), 11);
    assert!(body.starts_with("sku,category,model,quantity,price"));
    assert!(body.contains("source,sourceUrl"));
    assert!(!body.contains("CPU-5800X"));
    assert!(!body.contains("socket=AM5;tdp=105W"));
}

#[tokio::test]
async fn failed_commit_leaves_no_snapshot() {
    let harness = Harness::new();
    let (_, preview) = harness
        .preview("mixed.csv", &fixture("mixed.csv"), "USD")
        .await;
    let preview_id = preview["previewId"].as_str().unwrap();
    let (status, _) = harness.commit(preview_id, &all_accept(&preview)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let (_, snapshots) = harness.get("/api/inventory/snapshots").await;
    assert!(snapshots.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn a_commit_whose_line_total_overflows_is_rejected() {
    let harness = Harness::new();
    let csv = b"sku,category,model,quantity,price,currency\nBIG,cpu,Fixture Big CPU,2,92233720368547758.07,USD\n";
    let (status, preview) = harness.preview("overflow.csv", csv, "USD").await;
    assert_eq!(status, StatusCode::OK, "{preview}");
    assert_eq!(preview["rows"][0]["status"], "valid", "{preview}");
    assert_eq!(
        preview["rows"][0]["normalized"]["priceMinor"],
        i64::MAX as u64
    );

    let preview_id = preview["previewId"].as_str().unwrap();
    let (status, error) = harness.commit(preview_id, &all_accept(&preview)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(error["error"]["code"], "price_overflow");
    assert_eq!(error["error"]["field"], "price");
    let (_, snapshots) = harness.get("/api/inventory/snapshots").await;
    assert!(snapshots.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn a_correction_whose_line_total_overflows_writes_no_revision() {
    let harness = Harness::new();
    let (_, preview) = harness
        .preview("sample.csv", SAMPLE_CSV.as_bytes(), "USD")
        .await;
    let preview_id = preview["previewId"].as_str().unwrap();
    let (status, committed) = harness.commit(preview_id, &all_accept(&preview)).await;
    assert_eq!(status, StatusCode::OK);
    let snapshot = &committed["snapshot"];
    let snapshot_id = snapshot["id"].as_str().unwrap().to_string();
    let record_id = snapshot["records"][0]["id"].as_str().unwrap().to_string();

    let (status, error) = harness
        .correct(
            &snapshot_id,
            &json!({
                "changes": [{
                    "recordId": record_id,
                    "quantity": 2,
                    "priceMinor": i64::MAX,
                }]
            }),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(error["error"]["code"], "price_overflow");
    assert_eq!(error["error"]["field"], "price");

    let (_, snapshots) = harness.get("/api/inventory/snapshots").await;
    assert_eq!(snapshots.as_array().unwrap().len(), 1);
    let (_, original) = harness
        .get(&format!("/api/inventory/snapshots/{snapshot_id}"))
        .await;
    assert_eq!(
        original["totals"]["valueMinor"],
        snapshot["totals"]["valueMinor"]
    );
}

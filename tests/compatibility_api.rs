//! End-to-end HTTP contract tests for sourced compatibility verdicts.

use std::path::Path;
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
    router: Router,
}

impl Harness {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(Store::open(&dir.path().join("inventory.sqlite3")).unwrap());
        let router = build_router(AppState::new(store, dir.path().join("missing-web")));
        Self { _dir: dir, router }
    }

    async fn json(&self, request: Request<Body>) -> (StatusCode, Value) {
        let response = self.router.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    async fn get(&self, uri: &str) -> (StatusCode, Value) {
        self.json(
            Request::builder()
                .method("GET")
                .uri(uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
    }

    async fn post(&self, uri: &str, body: &Value) -> (StatusCode, Value) {
        self.json(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
    }

    async fn import(&self, filename: &str, csv: &[u8]) -> Value {
        let boundary = "rigcraft-compat-boundary";
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
                "\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"currency\"\r\n\r\nUSD\r\n--{boundary}--\r\n"
            )
            .as_bytes(),
        );
        let (status, preview) = self
            .json(
                Request::builder()
                    .method("POST")
                    .uri("/api/imports")
                    .header(
                        header::CONTENT_TYPE,
                        format!("multipart/form-data; boundary={boundary}"),
                    )
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "preview failed: {preview}");
        let preview_id = preview["previewId"].as_str().unwrap().to_string();
        let decisions = preview["rows"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| json!({ "row": row["row"], "action": "accept" }))
            .collect::<Vec<_>>();
        let (status, committed) = self
            .post(
                &format!("/api/imports/{preview_id}/commit"),
                &json!({ "decisions": decisions, "duplicateResolutions": [] }),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "commit failed: {committed}");
        committed["snapshot"].clone()
    }

    async fn fixture_snapshot(&self, name: &str) -> Value {
        let path = Path::new("tests/fixtures/compatibility").join(name);
        let bytes = std::fs::read(path).unwrap();
        self.import(name, &bytes).await
    }

    async fn check(&self, snapshot_id: &str, selection: Value) -> (StatusCode, Value) {
        self.post(
            "/api/builds/check",
            &json!({ "snapshotId": snapshot_id, "selection": selection }),
        )
        .await
    }
}

fn all_skus(snapshot: &Value) -> Value {
    Value::Array(
        snapshot["records"]
            .as_array()
            .unwrap()
            .iter()
            .map(|record| json!({ "sku": record["sku"], "quantity": 1 }))
            .collect(),
    )
}

#[tokio::test]
async fn shipped_sample_build_reports_no_conflict() {
    let harness = Harness::new();
    let snapshot = harness
        .import("rigcraft-inventory-sample.csv", SAMPLE_CSV.as_bytes())
        .await;
    let (status, report) = harness
        .check(snapshot["id"].as_str().unwrap(), all_skus(&snapshot))
        .await;
    assert_eq!(status, StatusCode::OK, "{report}");
    assert_eq!(report["completeness"]["status"], "complete");
    assert_eq!(report["summary"]["incompatible"], 0);
    // Two facts are genuinely unpublished by the cited pages: BIOS support and
    // peak/transient power. They stay unknown with a follow-up.
    assert_eq!(report["summary"]["unknown"], 2);
    assert_eq!(report["overallStatus"], "unknown");
    let unknown = report["rules"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|rule| rule["status"] == "unknown")
        .map(|rule| rule["ruleId"].as_str().unwrap().to_string())
        .collect::<Vec<_>>();
    assert_eq!(unknown, vec!["cpu-bios", "psu-capacity"]);
    let socket = report["rules"]
        .as_array()
        .unwrap()
        .iter()
        .find(|rule| rule["ruleId"] == "cpu-socket")
        .unwrap();
    assert_eq!(socket["status"], "compatible");
    assert_eq!(socket["evidence"][0]["value"], "AM5");
    assert_eq!(socket["evidence"][1]["value"], "AM5");
    assert_eq!(
        socket["evidence"][0]["provenance"]["source"],
        "catalog:manufacturer@2026-10-31"
    );
}

#[tokio::test]
async fn compatible_fixture_returns_sourced_passing_verdict() {
    let harness = Harness::new();
    let snapshot = harness.fixture_snapshot("compatible.csv").await;
    let (status, report) = harness
        .check(snapshot["id"].as_str().unwrap(), all_skus(&snapshot))
        .await;
    assert_eq!(status, StatusCode::OK, "{report}");
    assert_eq!(report["overallStatus"], "compatible");
    assert_eq!(report["completeness"]["status"], "complete");
    assert_eq!(report["ruleSetVersion"], "compat-rules-v1");
    assert_eq!(report["snapshotRevision"], 1);
    assert_eq!(report["summary"]["incompatible"], 0);
    assert_eq!(report["summary"]["unknown"], 0);
    assert_eq!(report["rules"].as_array().unwrap().len(), 14);
    for rule in report["rules"].as_array().unwrap() {
        assert_eq!(rule["ruleVersion"], "compat-rules-v1");
        assert!(rule["ruleId"].as_str().unwrap().contains('-'));
        assert!(rule["followUp"].is_null());
        for evidence in rule["evidence"].as_array().unwrap() {
            assert_eq!(evidence["provenance"]["source"], "user");
            assert_eq!(evidence["provenance"]["reference"], "csv:compatible.csv");
        }
    }
    let socket = report["rules"]
        .as_array()
        .unwrap()
        .iter()
        .find(|rule| rule["ruleId"] == "cpu-socket")
        .unwrap();
    assert_eq!(socket["status"], "compatible");
    assert_eq!(socket["evidence"][0]["value"], "AM5");
    assert_eq!(socket["evidence"][1]["value"], "AM5");

    let (status, stored) = harness
        .get(&format!(
            "/api/builds/checks/{}",
            report["id"].as_str().unwrap()
        ))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(stored["id"], report["id"]);
    assert_eq!(stored["stale"], false);
    assert_eq!(stored["currentSnapshotRevision"], 1);
}

#[tokio::test]
async fn conflict_fixture_returns_named_conflicts() {
    let harness = Harness::new();
    let snapshot = harness.fixture_snapshot("conflict.csv").await;
    let (status, report) = harness
        .check(snapshot["id"].as_str().unwrap(), all_skus(&snapshot))
        .await;
    assert_eq!(status, StatusCode::OK, "{report}");
    assert_eq!(report["overallStatus"], "incompatible");
    assert_eq!(report["completeness"]["status"], "complete");
    let rules = report["rules"].as_array().unwrap();
    for id in [
        "cpu-socket",
        "ram-type",
        "gpu-length",
        "cooler-socket",
        "cooler-clearance",
        "board-case-form-factor",
        "psu-gpu-connectors",
        "psu-case-mount",
        "storage-interface",
    ] {
        let rule = rules.iter().find(|rule| rule["ruleId"] == id).unwrap();
        assert_eq!(rule["status"], "incompatible", "rule {id} should conflict");
        assert!(rule["explanation"].as_str().unwrap().len() > 10);
    }
    let connector = rules
        .iter()
        .find(|rule| rule["ruleId"] == "psu-gpu-connectors")
        .unwrap();
    assert!(connector["explanation"]
        .as_str()
        .unwrap()
        .contains("2×8-pin"));
    assert!(report["summary"]["incompatible"].as_u64().unwrap() >= 8);
}

#[tokio::test]
async fn unknown_fixture_returns_specific_follow_ups_without_conflicts() {
    let harness = Harness::new();
    let snapshot = harness.fixture_snapshot("unknown.csv").await;
    let (status, report) = harness
        .check(snapshot["id"].as_str().unwrap(), all_skus(&snapshot))
        .await;
    assert_eq!(status, StatusCode::OK, "{report}");
    assert_eq!(report["overallStatus"], "unknown");
    assert_eq!(report["summary"]["incompatible"], 0);
    let rules = report["rules"].as_array().unwrap();
    let bios = rules
        .iter()
        .find(|rule| rule["ruleId"] == "cpu-bios")
        .unwrap();
    assert_eq!(bios["status"], "unknown");
    assert!(bios["followUp"].as_str().unwrap().contains("BIOS"));
    let clearance = rules
        .iter()
        .find(|rule| rule["ruleId"] == "cooler-clearance")
        .unwrap();
    assert_eq!(clearance["status"], "unknown");
    assert!(clearance["followUp"].as_str().unwrap().contains("radiator"));
    assert!(clearance["evidence"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry["value"].is_null()));
}

#[tokio::test]
async fn a_new_snapshot_revision_marks_the_stored_report_stale() {
    let harness = Harness::new();
    let snapshot = harness.fixture_snapshot("compatible.csv").await;
    let snapshot_id = snapshot["id"].as_str().unwrap().to_string();
    let (status, report) = harness.check(&snapshot_id, all_skus(&snapshot)).await;
    assert_eq!(status, StatusCode::OK);
    let record_id = snapshot["records"][0]["id"].as_str().unwrap();
    let (status, corrected) = harness
        .post(
            &format!("/api/inventory/snapshots/{snapshot_id}/corrections"),
            &json!({
                "note": "Fixture stock recount",
                "changes": [{ "recordId": record_id, "quantity": 1 }]
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(corrected["revision"], 2);

    let (status, stored) = harness
        .get(&format!(
            "/api/builds/checks/{}",
            report["id"].as_str().unwrap()
        ))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(stored["snapshotRevision"], 1);
    assert_eq!(stored["currentSnapshotRevision"], 2);
    assert_eq!(stored["stale"], true);
    assert!(stored["staleReason"]
        .as_str()
        .unwrap()
        .contains("revision 2"));
    assert_eq!(stored["overallStatus"], "compatible");
}

#[tokio::test]
async fn malformed_selection_and_unknown_snapshot_have_stable_codes() {
    let harness = Harness::new();
    let snapshot = harness.fixture_snapshot("compatible.csv").await;
    let snapshot_id = snapshot["id"].as_str().unwrap();

    let (status, error) = harness
        .check(
            snapshot_id,
            json!([{ "sku": "FIX-CPU-AM5", "quantity": 0 }]),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["error"]["code"], "selection_invalid");

    let (status, error) = harness.check(snapshot_id, json!([])).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["error"]["code"], "selection_empty");

    let (status, error) = harness
        .check(
            snapshot_id,
            json!([
                { "sku": "FIX-CPU-AM5", "quantity": 1 },
                { "sku": "FIX-CPU-AM5", "quantity": 1 }
            ]),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["error"]["code"], "duplicate_selection");

    let (status, error) = harness
        .check(snapshot_id, json!([{ "sku": "NOPE", "quantity": 1 }]))
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(error["error"]["code"], "sku_not_found");

    let (status, error) = harness
        .check(
            snapshot_id,
            json!([{ "sku": "FIX-CPU-AM5", "quantity": 99 }]),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(error["error"]["code"], "insufficient_stock");
    assert_eq!(error["error"]["details"]["available"], 2);
    assert!(
        error["error"]["message"]
            .as_str()
            .unwrap()
            .contains("needs 99 units"),
        "{}",
        error["error"]["message"]
    );

    let (status, error) = harness
        .check("missing-snapshot", json!([{ "sku": "X", "quantity": 1 }]))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(error["error"]["code"], "snapshot_not_found");

    let (status, error) = harness.get("/api/builds/checks/missing-check").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(error["error"]["code"], "check_not_found");
}

#[tokio::test]
async fn a_second_cpu_in_one_build_is_rejected() {
    let harness = Harness::new();
    let csv = b"sku,category,model,quantity,price,specs\nFIX-CPU-A,cpu,Fixture CPU A,1,10.00,socket=AM5\nFIX-CPU-B,cpu,Fixture CPU B,1,10.00,socket=AM5\n";
    let snapshot = harness.import("two-cpus.csv", csv).await;
    let (status, error) = harness
        .check(
            snapshot["id"].as_str().unwrap(),
            json!([
                { "sku": "FIX-CPU-A", "quantity": 1 },
                { "sku": "FIX-CPU-B", "quantity": 1 }
            ]),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(error["error"]["code"], "category_multiple");
    assert!(error["error"]["details"]["skus"].as_array().unwrap().len() == 2);
}

#[tokio::test]
async fn spa_routes_serve_the_shell_with_200() {
    let dir = tempfile::tempdir().unwrap();
    let web = dir.path().join("web");
    std::fs::create_dir_all(&web).unwrap();
    std::fs::write(
        web.join("index.html"),
        "<!doctype html><html><body><div id=\"root\">RigCraft shell</div></body></html>",
    )
    .unwrap();
    let store = Arc::new(Store::open(&dir.path().join("inventory.sqlite3")).unwrap());
    let router = build_router(AppState::new(store, web));
    for path in ["/inventory", "/builds/check"] {
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(path)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "path {path}");
        assert!(response
            .headers()
            .get(header::CONTENT_TYPE)
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("text/html"));
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        assert!(String::from_utf8_lossy(&bytes).contains("RigCraft shell"));
    }
    // A path below a client route is not that route: it is a real 404, not a
    // 200 shell that renders the Inventory page.
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/builds/check/anything")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn an_incomplete_selection_is_unknown_not_compatible() {
    let harness = Harness::new();
    let snapshot = harness.fixture_snapshot("compatible.csv").await;
    let (status, report) = harness
        .check(
            snapshot["id"].as_str().unwrap(),
            json!([
                { "sku": "FIX-CPU-AM5", "quantity": 1 },
                { "sku": "FIX-MB-B650", "quantity": 1 },
                { "sku": "FIX-RAM-DDR5", "quantity": 1 },
                { "sku": "FIX-GPU-PCIE4", "quantity": 1 },
                { "sku": "FIX-PSU-850", "quantity": 1 },
                { "sku": "FIX-CASE-ATX", "quantity": 1 },
                { "sku": "FIX-COOLER-AM5", "quantity": 1 }
            ]),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(report["overallStatus"], "unknown");
    assert_eq!(report["completeness"]["status"], "incomplete");
    let missing = report["completeness"]["missing"].as_array().unwrap();
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0]["category"], "storage");
    assert!(missing[0]["followUp"].as_str().unwrap().contains("storage"));
    assert_eq!(report["summary"]["incompatible"], 0);
}

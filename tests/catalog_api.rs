//! HTTP contract tests for operator-supplied catalog ingestion, registry,
//! search, matching and import enrichment.
//!
//! Every fixture is a small synthetic catalog. No third-party dataset is read,
//! stored or committed.

use std::path::Path;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
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
        let value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        (status, value)
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

    async fn send(&self, method: &str, uri: &str, body: Value) -> (StatusCode, Value) {
        self.json(
            Request::builder()
                .method(method)
                .uri(uri)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
    }

    async fn upload(&self, catalog: &Value) -> (StatusCode, Value) {
        self.send(
            "POST",
            "/api/catalog/datasets",
            json!({ "acknowledged": true, "catalog": catalog }),
        )
        .await
    }

    async fn preview(&self, csv: &str) -> (StatusCode, Value) {
        let boundary = "rigcraft-catalog-test";
        let body = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"synthetic.csv\"\r\nContent-Type: text/csv\r\n\r\n{csv}\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"currency\"\r\n\r\nUSD\r\n--{boundary}--\r\n"
        );
        self.json(
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
        .await
    }

    async fn match_rows(&self, rows: Value) -> (StatusCode, Value) {
        self.send("POST", "/api/catalog/match", json!({ "rows": rows }))
            .await
    }
}

fn fixture() -> Value {
    let path = Path::new("tests/fixtures/catalog/synthetic.json");
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

async fn import_fixture(harness: &Harness) -> Value {
    let (status, report) = harness.upload(&fixture()).await;
    assert_eq!(status, StatusCode::OK, "{report}");
    report
}

fn match_row(line: &Value) -> &Value {
    &line["matches"][0]
}

#[tokio::test]
async fn a_valid_document_is_accepted_and_reports_counts() {
    let harness = Harness::new();
    let (status, report) = harness.upload(&fixture()).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(report["accepted"], true);
    assert_eq!(report["replaced"], false);
    assert_eq!(report["dataset"]["name"], "synthetic-fixture");
    assert_eq!(report["dataset"]["license"], "CC0-1.0");
    assert_eq!(report["dataset"]["version"], "fixture-1");
    assert_eq!(report["partCount"], 3);
    assert_eq!(report["categoryCounts"]["cpu"], 2);
    assert_eq!(report["categoryCounts"]["gpu"], 1);
    assert_eq!(report["dataset"]["active"], true);
    assert!(report["specKeys"]
        .as_array()
        .unwrap()
        .iter()
        .any(|key| key == "socket"));

    let (status, datasets) = harness.get("/api/catalog/datasets").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(datasets.as_array().unwrap().len(), 1);
    assert_eq!(datasets[0]["license"], "CC0-1.0");
    assert_eq!(datasets[0]["rowCount"], 3);
}

#[tokio::test]
async fn the_acknowledgement_is_required() {
    let harness = Harness::new();
    let (status, body) = harness
        .send(
            "POST",
            "/api/catalog/datasets",
            json!({ "catalog": fixture() }),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "catalog_acknowledgement_required");
    let (_, datasets) = harness.get("/api/catalog/datasets").await;
    assert!(datasets.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn unknown_format_and_version_are_refused() {
    let harness = Harness::new();
    let mut wrong_format = fixture();
    wrong_format["format"] = Value::String("open-db".to_string());
    let (status, body) = harness.upload(&wrong_format).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "catalog_format_unsupported");

    let mut wrong_version = fixture();
    wrong_version["version"] = Value::from(2);
    let (status, body) = harness.upload(&wrong_version).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "catalog_version_unsupported");
    let (_, datasets) = harness.get("/api/catalog/datasets").await;
    assert!(datasets.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn an_invalid_row_names_row_and_field_and_stores_nothing() {
    let harness = Harness::new();
    let mut bad = fixture();
    bad["parts"][1]["model"] = Value::String(String::new());
    let (status, body) = harness.upload(&bad).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"]["code"], "catalog_invalid");
    let rows = body["error"]["details"]["rows"].as_array().unwrap();
    assert_eq!(rows[0]["row"], 1);
    assert_eq!(rows[0]["field"], "model");
    assert!(rows[0]["message"].as_str().unwrap().contains("required"));

    let (_, datasets) = harness.get("/api/catalog/datasets").await;
    assert!(datasets.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn duplicate_skus_are_refused_naming_both_rows() {
    let harness = Harness::new();
    let mut duplicate = fixture();
    duplicate["parts"][1]["sku"] = Value::String("SYN-CPU-1".to_string());
    let (status, body) = harness.upload(&duplicate).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let rows = body["error"]["details"]["rows"].as_array().unwrap();
    assert!(rows.iter().any(|row| row["row"] == 0));
    assert!(rows.iter().any(|row| row["row"] == 1));
    assert!(rows
        .iter()
        .any(|row| row["code"] == "catalog_duplicate_sku"));
    let (_, datasets) = harness.get("/api/catalog/datasets").await;
    assert!(datasets.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn an_unknown_spec_key_is_refused_with_the_accepted_list() {
    let harness = Harness::new();
    let mut bad = fixture();
    bad["parts"][0]["specs"] = json!({ "socket_size": "AM5" });
    let (status, body) = harness.upload(&bad).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let row = &body["error"]["details"]["rows"][0];
    assert_eq!(row["field"], "specs.socket_size");
    let message = row["message"].as_str().unwrap();
    assert!(message.contains("socket"));
    assert!(message.contains("tdp"));
    let accepted = body["error"]["details"]["specKeys"].as_array().unwrap();
    assert!(accepted.iter().any(|key| key == "socket"));
    assert!(!accepted.iter().any(|key| key == "socket_size"));
}

#[tokio::test]
async fn re_import_replaces_one_dataset_and_leaves_the_other_alone() {
    let harness = Harness::new();
    let first = import_fixture(&harness).await;
    let first_id = first["dataset"]["id"].as_str().unwrap().to_string();

    let second = json!({
        "format": "rigcraft-catalog",
        "version": 1,
        "dataset": { "name": "second-catalog", "license": "CC0-1.0" },
        "parts": [{ "sku": "OTHER-1", "category": "ram", "model": "Other RAM" }]
    });
    let (status, second_report) = harness.upload(&second).await;
    assert_eq!(status, StatusCode::OK);
    let second_id = second_report["dataset"]["id"].as_str().unwrap().to_string();
    let (_, before) = harness.get("/api/catalog/datasets").await;
    let second_before = before
        .as_array()
        .unwrap()
        .iter()
        .find(|dataset| dataset["id"] == second_id.as_str())
        .unwrap()
        .clone();

    let mut replacement = fixture();
    replacement["parts"] = json!([{
        "sku": "SYN-CPU-9",
        "category": "cpu",
        "model": "Synthetic CPU Nine",
        "specs": { "socket": "AM5" }
    }]);
    let (status, report) = harness.upload(&replacement).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(report["replaced"], true);
    assert_eq!(report["dataset"]["id"], first_id.as_str());
    assert_eq!(report["partCount"], 1);

    let (_, datasets) = harness.get("/api/catalog/datasets").await;
    assert_eq!(datasets.as_array().unwrap().len(), 2);
    let second_after = datasets
        .as_array()
        .unwrap()
        .iter()
        .find(|dataset| dataset["id"] == second_id.as_str())
        .unwrap();
    assert_eq!(second_after["rowCount"], second_before["rowCount"]);
    assert_eq!(second_after["license"], second_before["license"]);
    assert_eq!(second_after["version"], second_before["version"]);

    let (status, search) = harness
        .get("/api/catalog/search?q=SYN-CPU-9&category=cpu")
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(search["total"], 1);
    assert_eq!(search["hits"][0]["dataset"]["name"], "synthetic-fixture");
    let (_, hidden) = harness.get("/api/catalog/search?q=SYN-CPU-1").await;
    assert!(hidden["hits"]
        .as_array()
        .unwrap()
        .iter()
        .all(|hit| hit["part"]["sku"] != "SYN-CPU-1"));
}

#[tokio::test]
async fn deleting_a_dataset_leaves_inventory_untouched() {
    let harness = Harness::new();
    let (status, preview) = harness
        .preview(
            "sku,category,model,quantity,price,currency,specs\n\
             INV-1,cpu,Operator CPU,1,10.00,USD,socket=AM5\n",
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{preview}");
    let commit = json!({
        "decisions": [{ "row": preview["rows"][0]["row"], "action": "accept" }],
        "duplicateResolutions": []
    });
    let (status, committed) = harness
        .send(
            "POST",
            &format!(
                "/api/imports/{}/commit",
                preview["previewId"].as_str().unwrap()
            ),
            commit,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{committed}");
    let snapshot_id = committed["snapshotId"].as_str().unwrap().to_string();

    let report = import_fixture(&harness).await;
    let dataset_id = report["dataset"]["id"].as_str().unwrap().to_string();

    let (status, body) = harness
        .send(
            "DELETE",
            &format!("/api/catalog/datasets/{dataset_id}"),
            Value::Null,
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "catalog_confirm_required");

    let (status, body) = harness
        .send(
            "DELETE",
            &format!("/api/catalog/datasets/{dataset_id}?confirm=remove"),
            Value::Null,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["removed"], true);

    let (status, snapshot) = harness
        .get(&format!("/api/inventory/snapshots/{snapshot_id}"))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(snapshot["records"].as_array().unwrap().len(), 1);
    assert_eq!(snapshot["records"][0]["sku"], "INV-1");
    let (_, datasets) = harness.get("/api/catalog/datasets").await;
    assert!(datasets.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn search_ranks_exactly_and_pages_deterministically() {
    let harness = Harness::new();
    import_fixture(&harness).await;

    let (status, first) = harness
        .get("/api/catalog/search?q=SYN-CPU-1&category=cpu")
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(first["hits"][0]["part"]["sku"], "SYN-CPU-1");
    assert!(first["total"].as_u64().unwrap() >= 1);
    assert_eq!(first["hits"][0]["dataset"]["license"], "CC0-1.0");

    let (_, repeated) = harness
        .get("/api/catalog/search?q=SYN-CPU-1&category=cpu")
        .await;
    assert_eq!(first, repeated);

    let (status, token) = harness
        .get("/api/catalog/search?q=synthetic&limit=10")
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(token["total"], 3);
    let (_, page_one) = harness
        .get("/api/catalog/search?q=synthetic&limit=1&offset=0")
        .await;
    let (_, page_two) = harness
        .get("/api/catalog/search?q=synthetic&limit=1&offset=1")
        .await;
    assert_eq!(
        page_one["hits"][0]["part"]["id"],
        token["hits"][0]["part"]["id"]
    );
    assert_eq!(
        page_two["hits"][0]["part"]["id"],
        token["hits"][1]["part"]["id"]
    );
    assert_ne!(
        page_one["hits"][0]["part"]["id"],
        page_two["hits"][0]["part"]["id"]
    );

    let (status, body) = harness.get("/api/catalog/search").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "catalog_query_required");
    let (status, body) = harness.get("/api/catalog/search?q=x&limit=101").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "catalog_limit_invalid");
}

#[tokio::test]
async fn matching_is_three_state_and_has_no_default() {
    let harness = Harness::new();
    import_fixture(&harness).await;

    let (status, matched) = harness
        .match_rows(json!([{
            "row": 2,
            "sku": "SYN-CPU-1",
            "category": "cpu",
            "model": "Synthetic CPU One"
        }]))
        .await;
    assert_eq!(status, StatusCode::OK, "{matched}");
    assert_eq!(match_row(&matched)["status"], "matched");
    assert_eq!(match_row(&matched)["basis"], "sku");
    assert_eq!(
        match_row(&matched)["candidates"][0]["part"]["sku"],
        "SYN-CPU-1"
    );

    let (_, ambiguous) = harness
        .match_rows(json!([{
            "row": 3,
            "sku": "UNKNOWN",
            "category": "cpu",
            "model": "Synthetic CPU One"
        }]))
        .await;
    assert_eq!(match_row(&ambiguous)["status"], "ambiguous");
    assert_eq!(
        match_row(&ambiguous)["candidates"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert!(match_row(&ambiguous)["proposals"]["specs"]
        .as_object()
        .unwrap()
        .is_empty());

    let (_, unmatched) = harness
        .match_rows(json!([{
            "row": 4,
            "sku": "NOPE-1",
            "category": "cpu",
            "model": "Totally Different Part"
        }]))
        .await;
    assert_eq!(match_row(&unmatched)["status"], "unmatched");
    assert!(match_row(&unmatched)["candidates"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn enrichment_proposes_missing_fields_and_reports_conflicts() {
    let harness = Harness::new();
    import_fixture(&harness).await;

    let (_, matched) = harness
        .match_rows(json!([{
            "row": 2,
            "sku": "SYN-CPU-1",
            "category": "cpu",
            "model": "Synthetic CPU One",
            "specs": "socket=AM4"
        }]))
        .await;
    let result = match_row(&matched);
    let proposals = result["proposals"]["specs"].as_object().unwrap();
    assert!(!proposals.contains_key("socket"));
    assert!(proposals.contains_key("tdp"));
    assert!(result["proposals"].get("model").is_none());
    let conflicts = result["conflicts"].as_array().unwrap();
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0]["field"], "specs.socket");
    assert_eq!(conflicts[0]["operatorValue"], "AM4");
    assert_eq!(conflicts[0]["catalogValue"], "AM5");

    let part_id = result["candidates"][0]["part"]["id"].as_str().unwrap();

    let (status, preview) = harness
        .preview(
            "sku,category,model,quantity,price,currency,specs\n\
             SYN-CPU-1,cpu,Synthetic CPU One,1,10.00,USD,socket=AM4\n",
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{preview}");
    let commit = json!({
        "decisions": [{
            "row": preview["rows"][0]["row"],
            "action": "accept",
            "catalog": { "partId": part_id }
        }],
        "duplicateResolutions": []
    });
    let (status, committed) = harness
        .send(
            "POST",
            &format!(
                "/api/imports/{}/commit",
                preview["previewId"].as_str().unwrap()
            ),
            commit,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{committed}");
    let record = &committed["snapshot"]["records"][0];
    let specs = record["specs"].as_array().unwrap();
    let socket = specs
        .iter()
        .find(|spec| spec["key"] == "socket")
        .expect("operator socket");
    assert_eq!(socket["text"], "AM4");
    assert_eq!(socket["provenance"]["source"], "user");
    let catalog_spec = specs
        .iter()
        .find(|spec| spec["key"] == "tdp")
        .expect("catalog tdp");
    assert_eq!(
        catalog_spec["provenance"]["source"],
        "catalog:synthetic-fixture@fixture-1"
    );
    assert_eq!(
        catalog_spec["provenance"]["reference"],
        "https://example.invalid/cpu-one"
    );
    // The operator's own values are unchanged, and the row is not relabelled.
    assert_eq!(record["sku"], "SYN-CPU-1");
    assert_eq!(record["quantity"], 1);
    assert_eq!(record["provenance"]["sku"]["source"], "user");
    assert_eq!(record["provenance"]["quantity"]["source"], "user");
    assert_eq!(record["provenance"]["specs"]["source"], "user");
    assert_eq!(record["match"]["confidence"], "catalog-declared");
}

#[tokio::test]
async fn a_missing_catalog_part_refuses_the_commit() {
    let harness = Harness::new();
    let (status, preview) = harness
        .preview(
            "sku,category,model,quantity,price,currency,specs\n\
             SYN-CPU-1,cpu,Synthetic CPU One,1,10.00,USD\n",
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let commit = json!({
        "decisions": [{
            "row": preview["rows"][0]["row"],
            "action": "accept",
            "catalog": { "partId": "does-not-exist" }
        }],
        "duplicateResolutions": []
    });
    let (status, body) = harness
        .send(
            "POST",
            &format!(
                "/api/imports/{}/commit",
                preview["previewId"].as_str().unwrap()
            ),
            commit,
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"]["code"], "catalog_part_not_found");
    let (_, snapshots) = harness.get("/api/inventory/snapshots").await;
    assert!(snapshots.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn deactivating_a_dataset_removes_it_from_matching_and_search() {
    let harness = Harness::new();
    let report = import_fixture(&harness).await;
    let dataset_id = report["dataset"]["id"].as_str().unwrap();

    let (status, patched) = harness
        .send(
            "PATCH",
            &format!("/api/catalog/datasets/{dataset_id}"),
            json!({ "active": false }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{patched}");
    assert_eq!(patched["active"], false);

    let (_, matched) = harness
        .match_rows(json!([{
            "row": 1,
            "sku": "SYN-CPU-1",
            "category": "cpu",
            "model": "Synthetic CPU One"
        }]))
        .await;
    assert_eq!(match_row(&matched)["status"], "unmatched");

    let (_, search) = harness.get("/api/catalog/search?q=SYN-CPU-1").await;
    assert_eq!(search["total"], 0);

    let (status, _) = harness
        .send(
            "PATCH",
            &format!("/api/catalog/datasets/{dataset_id}"),
            json!({ "active": true }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let (_, matched) = harness
        .match_rows(json!([{
            "row": 1,
            "sku": "SYN-CPU-1",
            "category": "cpu",
            "model": "Synthetic CPU One"
        }]))
        .await;
    assert_eq!(match_row(&matched)["status"], "matched");
}

#[tokio::test]
async fn a_stored_part_exposes_its_specs_and_provenance() {
    let harness = Harness::new();
    import_fixture(&harness).await;
    let (_, search) = harness.get("/api/catalog/search?q=SYN-CPU-1").await;
    let part_id = search["hits"][0]["part"]["id"].as_str().unwrap();
    let (status, detail) = harness.get(&format!("/api/catalog/parts/{part_id}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(detail["part"]["sku"], "SYN-CPU-1");
    assert_eq!(detail["part"]["manufacturer"], "Synthetic Labs");
    assert_eq!(
        detail["provenance"]["model"]["source"],
        "catalog:synthetic-fixture@fixture-1"
    );
    assert_eq!(
        detail["provenance"]["specs"]["reference"],
        "https://example.invalid/cpu-one"
    );
    assert_eq!(
        detail["part"]["specs"][0]["provenance"]["source"],
        "catalog:synthetic-fixture@fixture-1"
    );

    let (status, body) = harness.get("/api/catalog/parts/nope").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"]["code"], "catalog_part_not_found");
}

#[tokio::test]
async fn unknown_dataset_and_part_ids_are_not_found() {
    let harness = Harness::new();
    let (status, body) = harness
        .send(
            "PATCH",
            "/api/catalog/datasets/nope",
            json!({ "active": false }),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"]["code"], "catalog_dataset_not_found");
    let (status, body) = harness
        .send(
            "DELETE",
            "/api/catalog/datasets/nope?confirm=remove",
            Value::Null,
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"]["code"], "catalog_dataset_not_found");
}

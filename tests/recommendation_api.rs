//! End-to-end HTTP contract tests for budget- and stock-constrained builds.

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

    async fn raw(&self, request: Request<Body>) -> (StatusCode, String, String) {
        let response = self.router.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_string();
        let disposition = response
            .headers()
            .get(header::CONTENT_DISPOSITION)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_string();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (
            status,
            content_type,
            format!("{}\n{}", disposition, String::from_utf8_lossy(&bytes)),
        )
    }

    async fn json(&self, request: Request<Body>) -> (StatusCode, Value) {
        let (status, _, body) = self.raw(request).await;
        let json_part = body.split_once('\n').map(|(_, rest)| rest).unwrap_or("");
        (
            status,
            serde_json::from_str(json_part).unwrap_or(Value::Null),
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
        let boundary = "rigcraft-recommendation-boundary";
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

    async fn fixture(&self, name: &str) -> Value {
        let path = Path::new("tests/fixtures/recommendation").join(name);
        let bytes = std::fs::read(path).unwrap();
        self.import(name, &bytes).await
    }

    async fn recommend(&self, snapshot_id: &str, budget: i64) -> (StatusCode, Value) {
        self.post(
            "/api/recommendations",
            &json!({
                "snapshotId": snapshot_id,
                "budgetMinor": budget,
                "currency": "USD",
                "useCase": "gaming",
                "inventoryOnly": true,
                "lockedSkus": [],
                "excludedSkus": []
            }),
        )
        .await
    }
}

fn part_skus(candidate: &Value) -> Vec<String> {
    let mut skus: Vec<String> = candidate["parts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|part| part["sku"].as_str().unwrap().to_string())
        .collect();
    skus.sort();
    skus
}

fn projection(report: &Value) -> Value {
    json!({
        "status": report["status"],
        "budgetMinor": report["budgetMinor"],
        "noSolution": report["noSolution"],
        "search": {
            "limit": report["search"]["limit"],
            "leavesEvaluated": report["search"]["leavesEvaluated"],
            "validBuilds": report["search"]["validBuilds"],
            "limited": report["search"]["limited"],
        },
        "candidates": report["candidates"].as_array().unwrap().iter().map(|candidate| json!({
            "rank": candidate["rank"],
            "objective": candidate["objective"],
            "parts": candidate["parts"].as_array().unwrap().iter().map(|part| json!({
                "sku": part["sku"], "quantity": part["quantity"], "owned": part["owned"],
            })).collect::<Vec<_>>(),
            "totalMinor": candidate["totalMinor"],
            "budgetDeltaMinor": candidate["budgetDeltaMinor"],
            "overallStatus": candidate["overallStatus"],
            "completeness": candidate["completeness"],
            "scores": candidate["scores"],
        })).collect::<Vec<_>>(),
    })
}

#[tokio::test]
async fn feasible_inventory_returns_ranked_valid_builds() {
    let harness = Harness::new();
    let snapshot = harness.fixture("feasible.csv").await;
    let (status, report) = harness
        .recommend(snapshot["id"].as_str().unwrap(), 200_000)
        .await;
    assert_eq!(status, StatusCode::OK, "{report}");
    assert_eq!(report["status"], "ok");
    assert_eq!(report["scoringVersion"], "recommend-score-v1");
    assert_eq!(report["ruleSetVersion"], "compat-rules-v1");
    assert_eq!(report["snapshotRevision"], 1);
    assert_eq!(report["search"]["limit"], 100_000);
    assert_eq!(report["search"]["limited"], false);
    assert!(report["search"]["validBuilds"].as_u64().unwrap() > 0);

    let candidates = report["candidates"].as_array().unwrap();
    assert!(!candidates.is_empty() && candidates.len() <= 3);
    let snapshot_prices: std::collections::BTreeMap<String, i64> = snapshot["records"]
        .as_array()
        .unwrap()
        .iter()
        .map(|record| {
            (
                record["sku"].as_str().unwrap().to_string(),
                record["priceMinor"].as_i64().unwrap(),
            )
        })
        .collect();
    let mut signatures = std::collections::BTreeSet::new();
    for candidate in candidates {
        assert_eq!(candidate["completeness"], "complete");
        assert_eq!(candidate["overallStatus"], "compatible");
        assert_eq!(candidate["check"]["overallStatus"], "compatible");
        assert_eq!(candidate["check"]["completeness"]["status"], "complete");
        assert!(candidate["totalMinor"].as_i64().unwrap() <= 200_000);
        assert_eq!(
            candidate["budgetDeltaMinor"].as_i64().unwrap(),
            200_000 - candidate["totalMinor"].as_i64().unwrap()
        );
        assert!(!candidate["reasons"].as_array().unwrap().is_empty());
        assert!(!candidate["check"]["rules"].as_array().unwrap().is_empty());
        let mut expected_total = 0;
        for part in candidate["parts"].as_array().unwrap() {
            let sku = part["sku"].as_str().unwrap();
            let quantity = part["quantity"].as_i64().unwrap();
            assert!(quantity >= 1);
            assert!(part["available"].as_i64().unwrap() >= quantity);
            if part["owned"].as_bool().unwrap() {
                assert_eq!(part["lineTotalMinor"], 0);
            } else {
                assert_eq!(
                    part["lineTotalMinor"].as_i64().unwrap(),
                    snapshot_prices[sku] * quantity
                );
            }
            expected_total += part["lineTotalMinor"].as_i64().unwrap();
        }
        assert_eq!(candidate["totalMinor"].as_i64().unwrap(), expected_total);
        assert!(signatures.insert(part_skus(candidate).join("|")));
        let dimension = candidate["scores"]["dimensions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|dimension| dimension["key"] == "performance")
            .unwrap();
        assert_eq!(dimension["available"], true);
        assert!(dimension["value"].as_f64().unwrap() > 0.0);
    }
    assert_eq!(candidates[0]["rank"], 1);
    assert_eq!(candidates[0]["objective"], "best_overall");
    assert_eq!(candidates[0]["label"], "Best overall");
    // Every part came from this snapshot and no candidate used a SKU twice.
    for candidate in candidates {
        for part in candidate["parts"].as_array().unwrap() {
            assert!(snapshot_prices.contains_key(part["sku"].as_str().unwrap()));
        }
        let mut per_category = std::collections::BTreeMap::new();
        for part in candidate["parts"].as_array().unwrap() {
            let count = per_category
                .entry(part["category"].as_str().unwrap().to_string())
                .or_insert(0);
            *count += 1;
            assert!(*count <= 2, "single categories must not repeat");
        }
    }
}

#[tokio::test]
async fn identical_requests_produce_the_same_order_parts_and_totals() {
    let harness = Harness::new();
    let snapshot = harness.fixture("feasible.csv").await;
    let snapshot_id = snapshot["id"].as_str().unwrap();
    let (_, first) = harness.recommend(snapshot_id, 200_000).await;
    let (_, second) = harness.recommend(snapshot_id, 200_000).await;
    assert_eq!(projection(&first), projection(&second));
    assert_ne!(first["id"], second["id"]);

    let (status, stored) = harness
        .get(&format!(
            "/api/recommendations/{}",
            first["id"].as_str().unwrap()
        ))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(projection(&stored), projection(&first));
    assert_eq!(stored["stale"], false);
}

#[tokio::test]
async fn a_budget_below_the_cheapest_build_explains_the_binding_constraint() {
    let harness = Harness::new();
    let snapshot = harness.fixture("tight-budget.csv").await;
    let (status, report) = harness
        .recommend(snapshot["id"].as_str().unwrap(), 120_000)
        .await;
    assert_eq!(status, StatusCode::OK, "{report}");
    assert_eq!(report["status"], "no_solution");
    assert_eq!(report["candidates"].as_array().unwrap().len(), 0);
    let no_solution = &report["noSolution"];
    assert_eq!(no_solution["binding"], "budget");
    assert_eq!(no_solution["minimumBuildMinor"], 245_000);
    assert_eq!(no_solution["budgetShortfallMinor"], 125_000);
    assert!(no_solution["explanation"]
        .as_str()
        .unwrap()
        .contains("2,450.00 USD"));
    assert!(!no_solution["adjustable"].as_array().unwrap().is_empty());
    assert!(no_solution["blockers"]
        .as_array()
        .unwrap()
        .iter()
        .any(|blocker| blocker["code"] == "cheapest_part"));
}

#[tokio::test]
async fn missing_stock_is_reported_as_the_stock_constraint() {
    let harness = Harness::new();
    let snapshot = harness.fixture("missing-storage.csv").await;
    let (status, report) = harness
        .recommend(snapshot["id"].as_str().unwrap(), 200_000)
        .await;
    assert_eq!(status, StatusCode::OK, "{report}");
    assert_eq!(report["status"], "no_solution");
    let no_solution = &report["noSolution"];
    assert_eq!(no_solution["binding"], "stock");
    assert!(no_solution["explanation"]
        .as_str()
        .unwrap()
        .contains("storage"));
    let blocker = no_solution["blockers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|blocker| blocker["category"] == "storage")
        .unwrap();
    assert!(blocker["detail"].as_str().unwrap().contains("No storage"));
}

#[tokio::test]
async fn an_all_conflicting_snapshot_is_not_recommended() {
    let harness = Harness::new();
    let snapshot = harness.fixture("conflict-only.csv").await;
    let (status, report) = harness
        .recommend(snapshot["id"].as_str().unwrap(), 200_000)
        .await;
    assert_eq!(status, StatusCode::OK, "{report}");
    assert_eq!(report["status"], "no_solution");
    assert_eq!(report["candidates"].as_array().unwrap().len(), 0);
    let no_solution = &report["noSolution"];
    assert_eq!(no_solution["binding"], "compatibility");
    assert!(no_solution["blockers"]
        .as_array()
        .unwrap()
        .iter()
        .any(|blocker| blocker["detail"].as_str().unwrap().contains("cpu-socket")));
    assert!(report["search"]["leavesEvaluated"].as_u64().unwrap() >= 1);
}

#[tokio::test]
async fn owned_and_excluded_parts_are_honoured() {
    let harness = Harness::new();
    let snapshot = harness.fixture("feasible.csv").await;
    let (status, report) = harness
        .post(
            "/api/recommendations",
            &json!({
                "snapshotId": snapshot["id"],
                "budgetMinor": 200_000,
                "currency": "USD",
                "useCase": "gaming",
                "inventoryOnly": true,
                "lockedSkus": ["FIX-R-CPU-A"],
                "excludedSkus": ["FIX-R-GPU-B"]
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{report}");
    let candidates = report["candidates"].as_array().unwrap();
    assert!(!candidates.is_empty());
    for candidate in candidates {
        let cpu = candidate["parts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|part| part["category"] == "cpu")
            .unwrap();
        assert_eq!(cpu["sku"], "FIX-R-CPU-A");
        assert_eq!(cpu["owned"], true);
        assert_eq!(cpu["lineTotalMinor"], 0);
        let stock = candidate["stock"]
            .as_array()
            .unwrap()
            .iter()
            .find(|line| line["sku"] == "FIX-R-CPU-A")
            .unwrap();
        assert_eq!(stock["owned"], true);
        assert_eq!(stock["used"], 0);
        assert!(!candidate["parts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|part| part["sku"] == "FIX-R-GPU-B"));
        assert!(candidate["ownedUnits"].as_u64().unwrap() >= 1);
    }
    let exclusions = report["exclusions"].as_array().unwrap();
    assert!(exclusions
        .iter()
        .any(|exclusion| exclusion["sku"] == "FIX-R-GPU-B"
            && exclusion["reason"] == "requested_excluded"));
    assert!(exclusions
        .iter()
        .any(|exclusion| exclusion["reason"] == "locked_category"));
}

#[tokio::test]
async fn a_missing_performance_proxy_is_not_scored_as_zero() {
    let harness = Harness::new();
    let snapshot = harness.fixture("no-proxy.csv").await;
    let (status, report) = harness
        .recommend(snapshot["id"].as_str().unwrap(), 200_000)
        .await;
    assert_eq!(status, StatusCode::OK, "{report}");
    assert_eq!(report["status"], "ok");
    let candidate = &report["candidates"][0];
    let dimension = candidate["scores"]["dimensions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|dimension| dimension["key"] == "performance")
        .unwrap();
    assert_eq!(dimension["available"], false);
    assert!(dimension["value"].is_null());
    assert_eq!(dimension["weight"], 0.0);
    assert_eq!(candidate["scores"]["confidence"], "partial");
    assert!(candidate["scores"]["total"].as_f64().unwrap() > 0.0);
    assert!(candidate["reasons"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reason| reason.as_str().unwrap().contains("perf_score")));
}

#[tokio::test]
async fn exports_round_trip_the_report_and_name_the_check_reports() {
    let harness = Harness::new();
    let snapshot = harness.fixture("feasible.csv").await;
    let (_, report) = harness
        .recommend(snapshot["id"].as_str().unwrap(), 200_000)
        .await;
    let id = report["id"].as_str().unwrap();

    let (status, content_type, body) = harness
        .raw(
            Request::builder()
                .method("GET")
                .uri(format!("/api/recommendations/{id}/export?format=json"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(content_type, "application/json");
    assert!(body.starts_with(&format!(
        "attachment; filename=\"rigcraft-recommendation-{}.json\"",
        id.chars().take(8).collect::<String>()
    )));
    let exported: Value = serde_json::from_str(body.split_once('\n').unwrap().1).unwrap();
    assert_eq!(exported["exportVersion"], "recommend-export-v1");
    assert_eq!(exported["snapshot"]["revision"], 1);
    assert_eq!(exported["snapshot"]["sourceFilename"], "feasible.csv");
    assert_eq!(exported["report"], report);
    assert_eq!(
        exported["checks"].as_array().unwrap().len(),
        report["candidates"].as_array().unwrap().len()
    );
    for (index, check) in exported["checks"].as_array().unwrap().iter().enumerate() {
        assert_eq!(
            check["checkReportId"],
            report["candidates"][index]["checkReportId"]
        );
        assert_eq!(check["ruleSetVersion"], "compat-rules-v1");
    }

    let (status, content_type, body) = harness
        .raw(
            Request::builder()
                .method("GET")
                .uri(format!("/api/recommendations/{id}/export?format=markdown"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(content_type, "text/markdown; charset=utf-8");
    assert!(body.contains("Manual confirmation required"));
    assert!(body.contains("## Ranked options"));
    assert!(body.contains("## Comparison"));
    assert!(body.contains(report["candidates"][0]["checkReportId"].as_str().unwrap()));
    assert!(body.contains(report["candidates"][0]["label"].as_str().unwrap()));
    assert!(body.contains("Evidence sources"));

    let (status, error) = harness
        .get(&format!("/api/recommendations/{id}/export?format=xml"))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["error"]["code"], "export_format_unsupported");
}

#[tokio::test]
async fn a_correction_marks_the_stored_report_stale() {
    let harness = Harness::new();
    let snapshot = harness.fixture("feasible.csv").await;
    let snapshot_id = snapshot["id"].as_str().unwrap().to_string();
    let (_, report) = harness.recommend(&snapshot_id, 200_000).await;
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
    assert_eq!(status, StatusCode::OK, "{corrected}");
    assert_eq!(corrected["revision"], 2);

    let (status, stored) = harness
        .get(&format!(
            "/api/recommendations/{}",
            report["id"].as_str().unwrap()
        ))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(stored["stale"], true);
    assert_eq!(stored["currentSnapshotRevision"], 2);
    assert!(stored["staleReason"]
        .as_str()
        .unwrap()
        .contains("revision 2"));

    let (_, _, body) = harness
        .raw(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/recommendations/{}/export?format=json",
                    report["id"].as_str().unwrap()
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    let exported: Value = serde_json::from_str(body.split_once('\n').unwrap().1).unwrap();
    assert_eq!(exported["report"]["stale"], true);
}

#[tokio::test]
async fn malformed_requests_have_stable_codes() {
    let harness = Harness::new();
    let snapshot = harness.fixture("feasible.csv").await;
    let snapshot_id = snapshot["id"].as_str().unwrap();

    let mut body = json!({
        "snapshotId": snapshot_id,
        "currency": "USD",
        "useCase": "gaming"
    });
    let (status, error) = harness.post("/api/recommendations", &body).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["error"]["code"], "budget_required");

    body["budgetMinor"] = json!(0);
    let (status, error) = harness.post("/api/recommendations", &body).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["error"]["code"], "budget_invalid");

    body["budgetMinor"] = json!(200_000);
    body["currency"] = json!("EUR");
    let (status, error) = harness.post("/api/recommendations", &body).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["error"]["code"], "currency_mismatch");

    body["currency"] = json!("USD");
    body["useCase"] = json!("mining");
    let (status, error) = harness.post("/api/recommendations", &body).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["error"]["code"], "use_case_unsupported");

    body["useCase"] = json!("gaming");
    body["inventoryOnly"] = json!(false);
    let (status, error) = harness.post("/api/recommendations", &body).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["error"]["code"], "catalog_unavailable");

    body["inventoryOnly"] = json!(true);
    body["lockedSkus"] = json!(["NOPE"]);
    let (status, error) = harness.post("/api/recommendations", &body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(error["error"]["code"], "locked_sku_not_found");

    body["lockedSkus"] = json!([]);
    body["excludedSkus"] = json!(["NOPE"]);
    let (status, error) = harness.post("/api/recommendations", &body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(error["error"]["code"], "excluded_sku_not_found");

    let (status, error) = harness.post("/api/recommendations", &json!({})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["error"]["code"], "snapshot_required");

    let (status, error) = harness
        .post(
            "/api/recommendations",
            &json!({
                "snapshotId": "missing-snapshot",
                "budgetMinor": 100,
                "currency": "USD",
                "useCase": "gaming"
            }),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(error["error"]["code"], "snapshot_not_found");

    let (status, error) = harness.get("/api/recommendations/missing").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(error["error"]["code"], "recommendation_not_found");
}

fn scale_csv(per_category: usize) -> String {
    let models: Vec<String> = (0..per_category)
        .map(|index| format!("Fixture Scale CPU {index}"))
        .collect();
    let bios = models.join(" / ");
    let mut csv = String::from("sku,category,model,quantity,price,currency,specs\n");
    for index in 0..per_category {
        csv.push_str(&format!(
            "SCALE-CPU-{index},cpu,Fixture Scale CPU {index},1,0.00,USD,socket=AM5;tdp=65W;peak_power=88W;igpu=no;perf_score=50pt\n"
        ));
        csv.push_str(&format!(
            "SCALE-MB-{index},motherboard,Fixture Scale Board {index},1,0.00,USD,socket=AM5;chipset=B650;memory_type=DDR5;form_factor=ATX;ram_slots=4;max_memory=192GB;bios_supports={bios};storage_interface=NVMe;pcie_version=PCIe5\n"
        ));
        csv.push_str(&format!(
            "SCALE-RAM-{index},ram,Fixture Scale Memory {index},1,0.00,USD,memory_type=DDR5;capacity=16GB;modules=2\n"
        ));
        csv.push_str(&format!(
            "SCALE-GPU-{index},gpu,Fixture Scale Graphics {index},1,0.00,USD,interface=PCIe4;length=240mm;tdp=115W;peak_power=140W;connector=1x8pin;perf_score=50pt\n"
        ));
        csv.push_str(&format!(
            "SCALE-PSU-{index},psu,Fixture Scale Power {index},1,0.00,USD,wattage=650W;psu_connectors=2x8pin;form_factor=ATX;length=160mm\n"
        ));
        csv.push_str(&format!(
            "SCALE-CASE-{index},case,Fixture Scale Case {index},1,0.00,USD,supported_form_factors=ATX;max_gpu_length=360mm;max_cooler_height=170mm;psu_form_factor=ATX;max_psu_length=180mm\n"
        ));
        csv.push_str(&format!(
            "SCALE-COOLER-{index},cooler,Fixture Scale Cooler {index},1,0.00,USD,cooler_socket=AM5;height=155mm;type=air\n"
        ));
        csv.push_str(&format!(
            "SCALE-SSD-{index},storage,Fixture Scale Storage {index},1,0.00,USD,interface=NVMe;capacity=1000GB\n"
        ));
    }
    csv
}

#[tokio::test]
async fn a_large_inventory_stops_at_the_stated_work_limit() {
    let harness = Harness::new();
    let csv = scale_csv(7);
    let snapshot = harness.import("scale.csv", csv.as_bytes()).await;
    let (status, report) = harness
        .recommend(snapshot["id"].as_str().unwrap(), 1_000_000)
        .await;
    assert_eq!(status, StatusCode::OK, "{report}");
    assert_eq!(report["status"], "search_limited");
    assert_eq!(report["search"]["limited"], true);
    assert_eq!(report["search"]["limit"], 100_000);
    assert!(report["search"]["nodesExamined"].as_u64().unwrap() >= 100_000);
}

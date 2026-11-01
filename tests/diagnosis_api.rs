//! End-to-end HTTP contract tests for existing-build diagnosis.

use std::path::{Path, PathBuf};
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
        let boundary = "rigcraft-diagnosis-boundary";
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
        let path = Path::new("tests/fixtures/diagnosis").join(name);
        let bytes = std::fs::read(path).unwrap();
        self.import(name, &bytes).await
    }

    fn count(&self, table: &str) -> i64 {
        let connection = rusqlite::Connection::open(&self.db_path).unwrap();
        connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap()
    }
}

fn owned(category: &str, model: &str, specs: &str) -> Value {
    json!({
        "source": "owned",
        "category": category,
        "model": model,
        "quantity": 1,
        "specs": specs,
    })
}

fn from_snapshot(sku: &str) -> Value {
    json!({ "source": "snapshot", "sku": sku })
}

fn sku_of(snapshot: &Value, category: &str) -> String {
    snapshot["records"]
        .as_array()
        .unwrap()
        .iter()
        .find(|record| record["category"] == category)
        .map(|record| record["sku"].as_str().unwrap().to_string())
        .unwrap_or_else(|| panic!("no {category} record in the fixture"))
}

fn build_for(snapshot: &Value) -> Vec<Value> {
    vec![
        owned(
            "cpu",
            "Fixture AM4 CPU 5600",
            "socket=AM4;tdp=65W;peak_power=88W;igpu=no",
        ),
        from_snapshot(&sku_of(snapshot, "motherboard")),
        from_snapshot(&sku_of(snapshot, "ram")),
        json!({ "source": "snapshot", "sku": "FIX-GPU-4070-TWIN" }),
        from_snapshot(&sku_of(snapshot, "psu")),
        from_snapshot(&sku_of(snapshot, "case")),
        from_snapshot(&sku_of(snapshot, "cooler")),
        from_snapshot(&sku_of(snapshot, "storage")),
    ]
}

async fn diagnose(harness: &Harness, snapshot: &Value, build: Value) -> (StatusCode, Value) {
    harness
        .post(
            "/api/diagnoses",
            &json!({
                "snapshotId": snapshot["id"],
                "currency": "USD",
                "build": build,
            }),
        )
        .await
}

#[tokio::test]
async fn pasted_list_reports_matches_ambiguity_misses_and_duplicates() {
    let harness = Harness::new();
    let snapshot = harness.fixture("upgrade.csv").await;
    let text = std::fs::read_to_string("tests/fixtures/diagnosis/paste-list.txt").unwrap();
    let (status, parsed) = harness
        .post(
            "/api/diagnoses/parse",
            &json!({
                "snapshotId": snapshot["id"],
                "currency": "USD",
                "text": text,
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{parsed}");
    assert_eq!(parsed["matchVersion"], "diagnosis-match-v1");
    assert_eq!(parsed["lineCount"], 5);
    assert_eq!(parsed["limits"]["maxLines"], 40);
    let lines = parsed["lines"].as_array().unwrap();

    assert_eq!(lines[0]["status"], "ambiguous", "{}", lines[0]);
    assert_eq!(lines[0]["candidates"].as_array().unwrap().len(), 2);
    let extras: Vec<String> = lines[0]["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|candidate| candidate["extraTokens"].as_array().unwrap().clone())
        .map(|token| token.as_str().unwrap().to_string())
        .collect();
    assert!(extras.contains(&"twin".to_string()));
    assert!(extras.contains(&"wind".to_string()));
    let disambiguation = lines[0]["disambiguation"].as_str().unwrap();
    assert!(disambiguation.contains("twin"), "{disambiguation}");
    assert!(disambiguation.contains("wind"), "{disambiguation}");

    assert_eq!(lines[1]["status"], "unmatched", "{}", lines[1]);
    assert!(lines[1]["candidates"].as_array().unwrap().is_empty());
    assert!(lines[1]["disambiguation"]
        .as_str()
        .unwrap()
        .contains("full model"));

    assert_eq!(lines[2]["status"], "matched", "{}", lines[2]);
    assert_eq!(lines[2]["candidates"][0]["sku"], "FIX-AM5-CPU-7600");
    assert_eq!(lines[2]["candidates"][0]["exact"], true);
    assert_eq!(lines[3]["status"], "matched", "{}", lines[3]);
    assert_eq!(lines[4]["duplicateOf"], 4, "{}", lines[4]);
    assert_eq!(lines[4]["issues"][0]["code"], "duplicate_row");

    // Parsing never persists a diagnosis.
    assert_eq!(harness.count("diagnoses"), 0);
}

#[tokio::test]
async fn pasted_input_is_bounded_and_validated() {
    let harness = Harness::new();
    let snapshot = harness.fixture("upgrade.csv").await;

    let (status, error) = harness
        .post(
            "/api/diagnoses/parse",
            &json!({ "snapshotId": snapshot["id"], "currency": "USD", "text": "  \n " }),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["error"]["code"], "text_required");
    assert_eq!(error["error"]["field"], "text");

    let forty_one = (1..=41)
        .map(|index| format!("gpu: Fixture RTX {index}"))
        .collect::<Vec<_>>()
        .join("\n");
    let (status, error) = harness
        .post(
            "/api/diagnoses/parse",
            &json!({ "snapshotId": snapshot["id"], "currency": "USD", "text": forty_one }),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["error"]["code"], "line_limit_exceeded");
    assert_eq!(error["error"]["details"]["limit"], 40);

    let huge = format!("gpu: {}", "x".repeat(9000));
    let (status, error) = harness
        .post(
            "/api/diagnoses/parse",
            &json!({ "snapshotId": snapshot["id"], "currency": "USD", "text": huge }),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["error"]["code"], "text_too_large");
    assert_eq!(error["error"]["field"], "text");

    let (status, error) = harness
        .post(
            "/api/diagnoses/parse",
            &json!({ "snapshotId": snapshot["id"], "text": "gpu: Fixture RTX 4070" }),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["error"]["code"], "currency_required");

    let (status, parsed) = harness
        .post(
            "/api/diagnoses/parse",
            &json!({ "snapshotId": snapshot["id"], "currency": "usd", "text": "gpu: Fixture RTX 4070" }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{parsed}");
    assert_eq!(parsed["currency"], "USD");

    let (status, error) = harness
        .post(
            "/api/diagnoses/parse",
            &json!({ "snapshotId": snapshot["id"], "currency": "us", "text": "gpu: Fixture RTX 4070" }),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["error"]["code"], "currency_format");

    let (status, error) = harness
        .post(
            "/api/diagnoses/parse",
            &json!({ "snapshotId": snapshot["id"], "currency": "EUR", "text": "gpu: Fixture RTX 4070" }),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["error"]["code"], "currency_mismatch");

    let (status, error) = harness
        .post(
            "/api/diagnoses/parse",
            &json!({ "snapshotId": "missing", "currency": "USD", "text": "gpu: Fixture RTX 4070" }),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(error["error"]["code"], "snapshot_not_found");

    assert_eq!(harness.count("diagnoses"), 0);
}

#[tokio::test]
async fn validated_upgrade_plan_replaces_the_conflicting_part() {
    let harness = Harness::new();
    let snapshot = harness.fixture("upgrade.csv").await;
    let (status, report) = diagnose(&harness, &snapshot, json!(build_for(&snapshot))).await;
    assert_eq!(status, StatusCode::OK, "{report}");

    assert_eq!(report["check"]["overallStatus"], "incompatible");
    assert_eq!(report["check"]["ruleSetVersion"], "compat-rules-v1");
    assert_eq!(report["adviceVersion"], "diagnosis-advice-v1");
    let conflict = report["check"]["rules"]
        .as_array()
        .unwrap()
        .iter()
        .find(|rule| rule["ruleId"] == "cpu-socket")
        .unwrap();
    assert_eq!(conflict["status"], "incompatible");
    assert_eq!(conflict["parts"].as_array().unwrap().len(), 2);

    assert_eq!(report["groups"]["mustReplace"], json!([1]));
    assert_eq!(report["groups"]["shouldUpgrade"], json!([]));
    assert_eq!(
        report["groups"]["keep"],
        json!([2, 3, 4, 5, 6, 7, 8]),
        "{report}"
    );

    let cpu_advice = &report["advice"][0];
    assert_eq!(cpu_advice["action"], "must_replace");
    assert_eq!(cpu_advice["conditional"], false);
    assert!(cpu_advice["ruleIds"]
        .as_array()
        .unwrap()
        .iter()
        .any(|rule| rule == "cpu-socket"));
    assert!(!cpu_advice["evidence"].as_array().unwrap().is_empty());
    assert_eq!(cpu_advice["cost"]["keepMinor"], 0);
    assert_eq!(cpu_advice["cost"]["replacementSku"], "FIX-AM5-CPU-7600");
    assert_eq!(cpu_advice["cost"]["additionalMinor"], 24900);

    let kept_board = &report["advice"][1];
    assert_eq!(kept_board["action"], "keep");
    assert!(kept_board["reason"]
        .as_str()
        .unwrap()
        .contains("FIX-AM5-CPU-7600"));

    assert_eq!(report["plan"]["status"], "validated", "{}", report["plan"]);
    assert_eq!(report["plan"]["steps"].as_array().unwrap().len(), 1);
    assert!(
        report["plan"]["summary"]
            .as_str()
            .unwrap()
            .contains("1 step adds"),
        "{}",
        report["plan"]["summary"]
    );
    assert_eq!(report["plan"]["steps"][0]["toSku"], "FIX-AM5-CPU-7600");
    assert_eq!(report["plan"]["steps"][0]["ruleIds"], json!(["cpu-socket"]));
    assert_eq!(report["plan"]["additionalCostMinor"], 24900);
    assert_eq!(report["plan"]["validation"]["overallStatus"], "compatible");
    assert!(report["plan"]["evaluated"].as_u64().unwrap() > 0);

    let id = report["id"].as_str().unwrap();
    let (status, stored) = harness.get(&format!("/api/diagnoses/{id}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(stored["stale"], false);
    assert_eq!(stored["plan"]["status"], "validated");
    assert_eq!(harness.count("diagnoses"), 1);
    assert_eq!(
        harness.count("check_reports"),
        0,
        "a diagnosis must not write a check report against a real snapshot"
    );
}

#[tokio::test]
async fn a_composed_plan_that_fails_the_checker_is_never_presented_as_valid() {
    let harness = Harness::new();
    let snapshot = harness.fixture("rejected.csv").await;
    let build = vec![
        owned(
            "cpu",
            "Fixture AM4 CPU 5600",
            "socket=AM4;tdp=65W;peak_power=88W;igpu=no",
        ),
        from_snapshot("FIX-REJ-MB-AM5-DDR5"),
        from_snapshot("FIX-REJ-RAM-DDR4"),
        from_snapshot("FIX-REJ-GPU"),
        from_snapshot("FIX-REJ-PSU"),
        from_snapshot("FIX-REJ-CASE"),
        from_snapshot("FIX-REJ-COOLER-AM5"),
        from_snapshot("FIX-REJ-SSD"),
    ];
    let (status, report) = diagnose(&harness, &snapshot, json!(build)).await;
    assert_eq!(status, StatusCode::OK, "{report}");
    assert_eq!(
        report["plan"]["status"], "no_valid_plan",
        "{}",
        report["plan"]
    );
    assert!(report["plan"]["steps"].as_array().unwrap().is_empty());
    assert_eq!(report["plan"]["validation"], Value::Null);
    assert!(!report["plan"]["rejected"].as_array().unwrap().is_empty());
    assert!(report["plan"]["summary"]
        .as_str()
        .unwrap()
        .contains("No plan is presented as valid"));

    // With no valid plan, every part in a conflict is still marked to replace.
    let must_replace = report["groups"]["mustReplace"].as_array().unwrap();
    assert!(must_replace.iter().any(|row| row == 1));
    assert!(must_replace.iter().any(|row| row == 2));
    assert!(must_replace.iter().any(|row| row == 3));
    for advice in report["advice"].as_array().unwrap() {
        if advice["action"] == "must_replace" {
            assert!(advice["reason"].as_str().unwrap().contains("conflict"));
        }
    }
}

#[tokio::test]
async fn a_snapshot_without_a_replacement_says_so() {
    let harness = Harness::new();
    let snapshot = harness.fixture("no-replacement.csv").await;
    let build = vec![
        owned(
            "cpu",
            "Fixture AM4 CPU 5600",
            "socket=AM4;tdp=65W;peak_power=88W;igpu=no",
        ),
        from_snapshot("FIX-NR-MB-AM5"),
        from_snapshot("FIX-NR-RAM-DDR5"),
        from_snapshot("FIX-NR-GPU"),
        from_snapshot("FIX-NR-PSU"),
        from_snapshot("FIX-NR-CASE"),
        from_snapshot("FIX-NR-COOLER-AM5"),
        from_snapshot("FIX-NR-SSD"),
    ];
    let (status, report) = diagnose(&harness, &snapshot, json!(build)).await;
    assert_eq!(status, StatusCode::OK, "{report}");
    assert_eq!(report["plan"]["status"], "no_option", "{}", report["plan"]);
    assert!(report["plan"]["steps"].as_array().unwrap().is_empty());
    assert!(report["plan"]["summary"]
        .as_str()
        .unwrap()
        .contains("cpu-socket"));
    let summary = report["plan"]["summary"].as_str().unwrap();
    assert!(!summary.contains("(s)"), "{summary}");
    assert!(summary.contains("rule"), "{summary}");
    let must_replace = report["groups"]["mustReplace"].as_array().unwrap();
    assert!(must_replace.iter().any(|row| row == 1));
    assert!(must_replace.iter().any(|row| row == 2));
}

#[tokio::test]
async fn a_missing_fact_is_a_conditional_upgrade_not_a_replacement() {
    let harness = Harness::new();
    let snapshot = harness.fixture("upgrade.csv").await;
    let build = vec![
        owned(
            "cpu",
            "Fixture AM5 CPU 7600",
            "socket=AM5;tdp=105W;peak_power=142W;igpu=no",
        ),
        owned(
            "motherboard",
            "Fixture B650 Board",
            "socket=AM5;memory_type=DDR5;form_factor=ATX;ram_slots=4;max_memory=128GB;storage_interface=NVMe;pcie_version=PCIe5",
        ),
        from_snapshot("FIX-RAM-DDR5"),
        from_snapshot("FIX-GPU-4070-TWIN"),
        from_snapshot("FIX-PSU-850"),
        from_snapshot("FIX-CASE-ATX"),
        from_snapshot("FIX-COOLER-AM5"),
        from_snapshot("FIX-SSD-NVME"),
    ];
    let (status, report) = diagnose(&harness, &snapshot, json!(build)).await;
    assert_eq!(status, StatusCode::OK, "{report}");
    assert_eq!(report["check"]["overallStatus"], "unknown");
    assert_eq!(report["plan"]["status"], "not_needed");
    assert_eq!(report["groups"]["mustReplace"], json!([]));
    assert_eq!(report["groups"]["shouldUpgrade"], json!([1, 2]));

    let cpu = report["advice"]
        .as_array()
        .unwrap()
        .iter()
        .find(|advice| advice["row"] == 1)
        .unwrap();
    assert_eq!(cpu["action"], "should_upgrade");
    assert_eq!(cpu["conditional"], true);
    assert!(cpu["reason"].as_str().unwrap().contains("BIOS"));
    assert!(cpu["verification"].as_str().unwrap().contains("BIOS"));
    assert_eq!(cpu["cost"]["keepMinor"], 0);
    // Nothing is inferred for the missing fact.
    let bios = report["check"]["rules"]
        .as_array()
        .unwrap()
        .iter()
        .find(|rule| rule["ruleId"] == "cpu-bios")
        .unwrap();
    assert_eq!(bios["status"], "unknown");
    assert!(bios["followUp"].as_str().unwrap().contains("BIOS"));
}

#[tokio::test]
async fn parts_with_no_evaluated_rule_are_not_presented_as_fine() {
    let harness = Harness::new();
    let snapshot = harness.fixture("upgrade.csv").await;
    let build = vec![
        owned(
            "cpu",
            "Fixture AM5 CPU 7600",
            "socket=AM5;tdp=105W;peak_power=142W;igpu=no",
        ),
        owned(
            "gpu",
            "Fixture PCIe4 GPU",
            "interface=PCIe4;length=300mm;tdp=200W;peak_power=240W;connector=2x8pin",
        ),
        owned(
            "ram",
            "Fixture DDR5 16GB Kit",
            "memory_type=DDR5;capacity=16GB;modules=2",
        ),
    ];
    let (status, report) = diagnose(&harness, &snapshot, json!(build)).await;
    assert_eq!(status, StatusCode::OK, "{report}");

    assert_eq!(report["check"]["rules"].as_array().unwrap().len(), 0);
    assert_eq!(
        report["check"]["summary"],
        json!({ "compatible": 0, "incompatible": 0, "unknown": 0 })
    );
    assert_eq!(report["groups"]["mustReplace"], json!([]));
    assert_eq!(report["groups"]["shouldUpgrade"], json!([]));
    assert_eq!(report["groups"]["keep"], json!([]));
    assert_eq!(report["groups"]["unverified"], json!([1, 2, 3]));
    for advice in report["advice"].as_array().unwrap() {
        assert_eq!(advice["action"], "unverified", "{advice}");
        assert!(advice["ruleIds"].as_array().unwrap().is_empty());
        assert!(advice["reason"].as_str().unwrap().contains("not verified"));
    }
}

#[tokio::test]
async fn invalid_builds_are_field_coded_and_store_nothing() {
    let harness = Harness::new();
    let snapshot = harness.fixture("upgrade.csv").await;

    let (status, error) = diagnose(&harness, &snapshot, json!([])).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["error"]["code"], "build_empty");
    assert_eq!(error["error"]["field"], "build");

    let (status, error) = diagnose(
        &harness,
        &snapshot,
        json!([
            owned("cpu", "Fixture AM5 CPU 7600", "socket=AM5"),
            owned("toaster", "Fixture Toaster", ""),
        ]),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["error"]["code"], "part_invalid");
    assert_eq!(error["error"]["row"], 2);

    let (status, error) = diagnose(
        &harness,
        &snapshot,
        json!([
            owned("ram", "Fixture DDR5 16GB Kit", "memory_type=DDR5"),
            owned("ram", "Fixture DDR5 16GB Kit", "memory_type=DDR5"),
        ]),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["error"]["code"], "duplicate_part");
    assert_eq!(error["error"]["row"], 2);

    let (status, error) = diagnose(
        &harness,
        &snapshot,
        json!([
            owned("cpu", "Fixture AM5 CPU 7600", "socket=AM5"),
            owned("cpu", "Fixture AM5 CPU 7800X", "socket=AM5"),
        ]),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(error["error"]["code"], "category_multiple");

    let (status, error) = diagnose(&harness, &snapshot, json!([from_snapshot("NOPE")])).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(error["error"]["code"], "sku_not_found");
    assert_eq!(error["error"]["row"], 1);

    let (status, error) = diagnose(
        &harness,
        &snapshot,
        json!([owned("cpu", "Fixture AM5 CPU 7600", "socket")]),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["error"]["code"], "part_invalid");
    assert_eq!(error["error"]["details"]["specCode"], "spec_format");

    let (status, error) = harness
        .post(
            "/api/diagnoses",
            &json!({ "snapshotId": snapshot["id"], "currency": "EUR", "build": build_for(&snapshot) }),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["error"]["code"], "currency_mismatch");

    let (status, error) = harness
        .post(
            "/api/diagnoses",
            &json!({ "snapshotId": "missing", "currency": "USD", "build": build_for(&snapshot) }),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(error["error"]["code"], "snapshot_not_found");

    assert_eq!(harness.count("diagnoses"), 0);
}

#[tokio::test]
async fn a_later_snapshot_revision_makes_the_diagnosis_stale() {
    let harness = Harness::new();
    let snapshot = harness.fixture("upgrade.csv").await;
    let (status, report) = diagnose(&harness, &snapshot, json!(build_for(&snapshot))).await;
    assert_eq!(status, StatusCode::OK, "{report}");
    let id = report["id"].as_str().unwrap().to_string();
    let snapshot_id = snapshot["id"].as_str().unwrap().to_string();
    let record_id = snapshot["records"][0]["id"].as_str().unwrap().to_string();

    let (status, corrected) = harness
        .post(
            &format!("/api/inventory/snapshots/{snapshot_id}/corrections"),
            &json!({
                "note": "Fixture recount",
                "changes": [{ "recordId": record_id, "quantity": 2 }],
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{corrected}");
    assert_eq!(corrected["revision"], 2);

    let (status, stored) = harness.get(&format!("/api/diagnoses/{id}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(stored["stale"], true);
    assert_eq!(stored["currentSnapshotRevision"], 2);
    let reason = stored["staleReason"].as_str().unwrap();
    assert!(reason.contains("revision 2"), "{reason}");
    assert!(reason.contains("revision 1"), "{reason}");

    let (status, error) = harness.get("/api/diagnoses/missing").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(error["error"]["code"], "diagnosis_not_found");
}

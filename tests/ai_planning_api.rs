//! End-to-end HTTP contract tests for the optional BYOK interpretation layer.
//!
//! Every test injects `MockProvider`; no test contacts a network provider and no
//! real credential is used. The one sentinel value below is a fixture string.

use std::path::Path;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use rigcraft::ai::{MockProvider, ProviderFailure};
use rigcraft::store::Store;
use rigcraft::{build_router, AppState};
use serde_json::{json, Value};
use tempfile::TempDir;
use tower::ServiceExt;

const SENTINEL_KEY: &str = "sk-rigcraft-test-sentinel-7f3a9c";
const BASE_URL: &str = "https://provider.example/v1";

struct Harness {
    dir: TempDir,
    router: Router,
    mock: Arc<MockProvider>,
}

impl Harness {
    fn with_completion(content: &str) -> Self {
        Self::with_mock(Arc::new(MockProvider::with_completion(content)))
    }

    fn with_failure(failure: ProviderFailure) -> Self {
        Self::with_mock(Arc::new(MockProvider::failing(failure)))
    }

    fn with_mock(mock: Arc<MockProvider>) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(Store::open(&dir.path().join("inventory.sqlite3")).unwrap());
        let state =
            AppState::new(store, dir.path().join("missing-web")).with_provider(mock.clone());
        let router = build_router(state);
        Self { dir, router, mock }
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
        let headers = format!("{:?}", response.headers());
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (
            status,
            format!("{content_type}\n{headers}"),
            String::from_utf8_lossy(&bytes).to_string(),
        )
    }

    async fn json(&self, request: Request<Body>) -> (StatusCode, Value) {
        let (status, _, body) = self.raw(request).await;
        (status, serde_json::from_str(&body).unwrap_or(Value::Null))
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
        let boundary = "rigcraft-ai-boundary";
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

    async fn parse(&self, body: Value) -> (StatusCode, Value) {
        self.post("/api/ai/parse", &body).await
    }

    fn parse_body(snapshot_id: &str, text: &str) -> Value {
        json!({
            "text": text,
            "snapshotId": snapshot_id,
            "apiKey": SENTINEL_KEY,
            "baseUrl": BASE_URL,
            "model": "mock-model",
            "includeInventorySummary": false,
        })
    }
}

fn grounded_reply() -> String {
    json!({
        "budgetMinor": { "value": 200000, "excerpt": "$2000" },
        "currency": { "value": "USD", "excerpt": "USD" },
        "useCase": { "value": "gaming", "excerpt": "gaming" },
        "inventoryOnly": { "value": true, "excerpt": "only my stock" },
        "lockedSkus": { "value": [], "excerpt": "" },
        "excludedSkus": { "value": [], "excerpt": "" }
    })
    .to_string()
}

const SENTENCE: &str = "Build a $2000 USD gaming PC from only my stock.";

#[tokio::test]
async fn a_grounded_reply_is_a_proposal_and_never_runs_a_recommendation() {
    let harness = Harness::with_completion(&grounded_reply());
    let snapshot = harness.fixture("feasible.csv").await;
    let snapshot_id = snapshot["id"].as_str().unwrap();
    let totals_before = snapshot["totals"].clone();

    let (status, proposal) = harness
        .parse(Harness::parse_body(snapshot_id, SENTENCE))
        .await;
    assert_eq!(status, StatusCode::OK, "{proposal}");
    assert_eq!(proposal["mode"], "provider");
    assert_eq!(proposal["promptVersion"], "ai-parse-v1");
    assert_eq!(proposal["confirmRequired"], true);
    assert_eq!(proposal["proposal"]["budgetMinor"]["value"], 200000);
    assert_eq!(proposal["proposal"]["budgetMinor"]["source"], "sentence");
    assert_eq!(proposal["proposal"]["budgetMinor"]["excerpt"], "$2000");
    assert_eq!(proposal["proposal"]["currency"]["value"], "USD");
    assert_eq!(proposal["proposal"]["inventoryOnly"]["source"], "sentence");
    assert_eq!(proposal["provider"]["baseUrl"], BASE_URL);
    assert_eq!(proposal["provider"]["model"], "mock-model");
    assert_eq!(proposal["provider"]["kind"], "mock");
    assert_eq!(harness.mock.calls(), 1);

    let request = proposal["request"].clone();
    assert_eq!(request["budgetMinor"], 200000);
    assert_eq!(request["snapshotId"], snapshot_id);

    // Parsing persisted nothing and mutated no snapshot.
    let (status, stored) = harness
        .get(&format!("/api/inventory/snapshots/{snapshot_id}"))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(stored["totals"], totals_before);
    let report_count = recommendation_rows(&harness.dir.path().join("inventory.sqlite3"));
    assert_eq!(report_count, 0, "a parse must not persist a recommendation");

    // The proposal is a real request: the deterministic engine accepts and
    // re-validates it.
    let (status, report) = harness.post("/api/recommendations", &request).await;
    assert_eq!(status, StatusCode::OK, "{report}");
    assert_eq!(report["status"], "ok");
    assert_eq!(report["request"]["budgetMinor"], 200000);
}

#[tokio::test]
async fn the_key_is_never_echoed_persisted_or_logged_and_goes_only_to_the_provider() {
    let harness = Harness::with_completion(&grounded_reply());
    let snapshot = harness.fixture("feasible.csv").await;
    let snapshot_id = snapshot["id"].as_str().unwrap();

    let request = Request::builder()
        .method("POST")
        .uri("/api/ai/parse")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            Harness::parse_body(snapshot_id, SENTENCE).to_string(),
        ))
        .unwrap();
    let (status, headers, body) = harness.raw(request).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(
        !body.contains(SENTINEL_KEY),
        "the response body echoed the key"
    );
    assert!(
        !headers.contains(SENTINEL_KEY),
        "a response header carried the key"
    );

    // Sent exactly once, to the operator's own provider URL.
    let call = harness.mock.last_call().expect("the provider was called");
    assert_eq!(harness.mock.calls(), 1);
    assert_eq!(call.endpoint, format!("{BASE_URL}/chat/completions"));
    assert_eq!(call.authorization, format!("Bearer {SENTINEL_KEY}"));

    // Nothing on disk carries the key or an AI table.
    let database = std::fs::read(harness.dir.path().join("inventory.sqlite3")).unwrap();
    assert!(!contains_bytes(&database, SENTINEL_KEY.as_bytes()));
    assert!(!has_ai_table(&harness.dir.path().join("inventory.sqlite3")));

    // A recommendation generated afterwards still carries no key, and its
    // exports do not either.
    let (status, proposal) = harness
        .parse(Harness::parse_body(snapshot_id, SENTENCE))
        .await;
    assert_eq!(status, StatusCode::OK, "{proposal}");
    let (status, report) = harness
        .post("/api/recommendations", &proposal["request"])
        .await;
    assert_eq!(status, StatusCode::OK, "{report}");
    let id = report["id"].as_str().unwrap();
    for format in ["json", "markdown"] {
        let (status, _, export) = harness
            .raw(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/recommendations/{id}/export?format={format}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
        assert_eq!(status, StatusCode::OK);
        assert!(
            !export.contains(SENTINEL_KEY),
            "the {format} export carried the key"
        );
    }
}

#[tokio::test]
async fn a_missing_key_is_refused_without_contacting_any_provider() {
    let harness = Harness::with_completion(&grounded_reply());
    let snapshot = harness.fixture("feasible.csv").await;
    let mut body = Harness::parse_body(snapshot["id"].as_str().unwrap(), SENTENCE);
    body["apiKey"] = json!("");
    let (status, error) = harness.parse(body).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["error"]["code"], "ai_key_required");
    assert_eq!(error["error"]["field"], "apiKey");
    assert!(error["error"]["message"]
        .as_str()
        .unwrap()
        .contains("deterministic form"));
    assert_eq!(harness.mock.calls(), 0);
}

#[tokio::test]
async fn an_oversized_sentence_is_refused_before_a_prompt_exists() {
    let harness = Harness::with_completion(&grounded_reply());
    let snapshot = harness.fixture("feasible.csv").await;
    let long = "build ".repeat(2000);
    let (status, error) = harness
        .parse(Harness::parse_body(snapshot["id"].as_str().unwrap(), &long))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["error"]["code"], "ai_text_too_large");
    assert_eq!(error["error"]["field"], "text");
    assert_eq!(harness.mock.calls(), 0);
}

#[tokio::test]
async fn a_remote_plain_http_provider_is_refused() {
    let harness = Harness::with_completion(&grounded_reply());
    let snapshot = harness.fixture("feasible.csv").await;
    let mut body = Harness::parse_body(snapshot["id"].as_str().unwrap(), SENTENCE);
    body["baseUrl"] = json!("http://198.51.100.7/v1");
    let (status, error) = harness.parse(body).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["error"]["code"], "ai_provider_url_invalid");
    assert_eq!(error["error"]["field"], "baseUrl");
    assert_eq!(harness.mock.calls(), 0);
}

#[tokio::test]
async fn provider_failures_are_stable_codes_without_the_response_body() {
    let timeout = Harness::with_failure(ProviderFailure {
        code: "ai_provider_timeout",
        status: 504,
        field: None,
        message: "The provider did not answer within 20 seconds.".to_string(),
    });
    let snapshot = timeout.fixture("feasible.csv").await;
    let (status, error) = timeout
        .parse(Harness::parse_body(
            snapshot["id"].as_str().unwrap(),
            SENTENCE,
        ))
        .await;
    assert_eq!(status, StatusCode::GATEWAY_TIMEOUT);
    assert_eq!(error["error"]["code"], "ai_provider_timeout");
    assert_eq!(timeout.mock.calls(), 1);

    let failing = Harness::with_failure(ProviderFailure {
        code: "ai_provider_error",
        status: 502,
        field: None,
        message: "The provider returned HTTP 401.".to_string(),
    });
    let snapshot = failing.fixture("feasible.csv").await;
    let (status, error) = failing
        .parse(Harness::parse_body(
            snapshot["id"].as_str().unwrap(),
            SENTENCE,
        ))
        .await;
    assert_eq!(status, StatusCode::BAD_GATEWAY);
    assert_eq!(error["error"]["code"], "ai_provider_error");
    assert!(!error.to_string().contains(SENTINEL_KEY));
}

#[tokio::test]
async fn an_unusable_reply_is_422_and_keeps_the_request_intact() {
    let harness = Harness::with_mock(Arc::new(MockProvider::with_raw_body("not json at all")));
    let snapshot = harness.fixture("feasible.csv").await;
    let (status, error) = harness
        .parse(Harness::parse_body(
            snapshot["id"].as_str().unwrap(),
            SENTENCE,
        ))
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(error["error"]["code"], "ai_response_invalid");

    // The deterministic flow still works after the failure.
    let (status, report) = harness
        .post(
            "/api/recommendations",
            &json!({
                "snapshotId": snapshot["id"],
                "budgetMinor": 200000,
                "currency": "USD",
                "useCase": "gaming",
                "inventoryOnly": true,
                "lockedSkus": [],
                "excludedSkus": []
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{report}");
    assert_eq!(report["status"], "ok");
}

#[tokio::test]
async fn an_untraceable_value_is_reported_instead_of_used() {
    let reply = json!({
        "budgetMinor": { "value": 900000, "excerpt": "$9000" },
        "useCase": { "value": "gaming", "excerpt": "gaming" }
    })
    .to_string();
    let harness = Harness::with_completion(&reply);
    let snapshot = harness.fixture("feasible.csv").await;
    let (status, proposal) = harness
        .parse(Harness::parse_body(
            snapshot["id"].as_str().unwrap(),
            SENTENCE,
        ))
        .await;
    assert_eq!(status, StatusCode::OK, "{proposal}");
    assert_eq!(proposal["proposal"]["budgetMinor"]["source"], "unresolved");
    assert_eq!(proposal["proposal"]["budgetMinor"]["value"], Value::Null);
    assert!(proposal["request"].is_null());
    assert!(proposal["ambiguities"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry["field"] == "budgetMinor" && entry["code"] == "ungrounded_value"));
}

#[tokio::test]
async fn an_invented_part_never_reaches_the_request() {
    let reply = json!({
        "budgetMinor": { "value": 200000, "excerpt": "$2000" },
        "useCase": { "value": "gaming", "excerpt": "gaming" },
        "lockedSkus": { "value": ["FIX-R-CPU-A", "FIX-INVENTED-PART"], "excerpt": "FIX-R-CPU-A" }
    })
    .to_string();
    let harness = Harness::with_completion(&reply);
    let snapshot = harness.fixture("feasible.csv").await;
    let sentence = "Build a $2000 gaming PC and keep FIX-R-CPU-A.";
    let (status, proposal) = harness
        .parse(Harness::parse_body(
            snapshot["id"].as_str().unwrap(),
            sentence,
        ))
        .await;
    assert_eq!(status, StatusCode::OK, "{proposal}");
    assert_eq!(
        proposal["proposal"]["lockedSkus"]["value"],
        json!(["FIX-R-CPU-A"])
    );
    assert_eq!(proposal["request"]["lockedSkus"], json!(["FIX-R-CPU-A"]));
    assert!(proposal["ambiguities"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry["code"] == "unknown_sku"));
}

#[tokio::test]
async fn the_inventory_is_absent_from_the_prompt_unless_opted_in() {
    let harness = Harness::with_completion(&grounded_reply());
    let snapshot = harness.fixture("feasible.csv").await;
    let snapshot_id = snapshot["id"].as_str().unwrap();
    let (status, first) = harness
        .parse(Harness::parse_body(snapshot_id, SENTENCE))
        .await;
    assert_eq!(status, StatusCode::OK, "{first}");
    assert_eq!(first["sent"]["inventoryRecords"], 0);
    let prompt = harness.mock.last_call().unwrap().body;
    assert!(!prompt.contains("FIX-R-CPU-A"));
    assert!(!prompt.contains("199.00"));

    let mut opted_in = Harness::parse_body(snapshot_id, SENTENCE);
    opted_in["includeInventorySummary"] = json!(true);
    let (status, second) = harness.parse(opted_in).await;
    assert_eq!(status, StatusCode::OK, "{second}");
    assert!(second["sent"]["inventoryRecords"].as_u64().unwrap() > 0);
    assert_eq!(second["sent"]["inventoryFields"][0], "sku");
    let prompt = harness.mock.last_call().unwrap().body;
    assert!(prompt.contains("FIX-R-CPU-A"));
    assert!(prompt.contains("priceMinor"));
}

#[tokio::test]
async fn a_revision_changes_only_the_field_in_its_diff() {
    let reply = json!({ "budgetMinor": { "value": 250000, "excerpt": "$2500" } }).to_string();
    let harness = Harness::with_completion(&reply);
    let snapshot = harness.fixture("feasible.csv").await;
    let snapshot_id = snapshot["id"].as_str().unwrap();
    let (status, proposal) = harness
        .post(
            "/api/ai/revise",
            &json!({
                "instruction": "raise the budget to $2500",
                "current": {
                    "budgetMinor": 200000,
                    "currency": "USD",
                    "useCase": "gaming",
                    "lockedSkus": ["FIX-R-CPU-A"],
                    "excludedSkus": []
                },
                "snapshotId": snapshot_id,
                "apiKey": SENTINEL_KEY,
                "baseUrl": BASE_URL,
                "model": "mock-model",
                "includeInventorySummary": false
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{proposal}");
    let diff = proposal["diff"].as_array().unwrap();
    assert_eq!(diff.len(), 1, "{diff:?}");
    assert_eq!(diff[0]["field"], "budgetMinor");
    assert_eq!(diff[0]["to"], 250000);
    assert_eq!(proposal["request"]["budgetMinor"], 250000);
    assert_eq!(proposal["request"]["useCase"], "gaming");
    assert_eq!(proposal["request"]["lockedSkus"], json!(["FIX-R-CPU-A"]));
}

fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

fn open_database(path: &Path) -> rusqlite::Connection {
    rusqlite::Connection::open(path).unwrap()
}

fn recommendation_rows(path: &Path) -> i64 {
    open_database(path)
        .query_row("SELECT COUNT(*) FROM recommendations", [], |row| row.get(0))
        .unwrap()
}

fn has_ai_table(path: &Path) -> bool {
    let connection = open_database(path);
    let mut statement = connection
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table'")
        .unwrap();
    let names: Vec<String> = statement
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(|name| name.unwrap())
        .collect();
    names.iter().any(|name| name.starts_with("ai_"))
}

//! Transport tests for the real OpenAI-compatible adapter.
//!
//! A loopback HTTP server stands in for a provider, so the production adapter,
//! its URL policy, its request shape and its failure mapping are exercised with
//! no external network and no real credential.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use http_body_util::BodyExt;
use rigcraft::ai::{CompletionRequest, OpenAiCompatibleProvider, ProviderAdapter, SecretKey};
use rigcraft::store::Store;
use rigcraft::{build_router, AppState};
use serde_json::{json, Value};
use tempfile::TempDir;
use tower::ServiceExt;

const DUMMY_KEY: &str = "loopback-test-key-not-real";

/// A one-shot HTTP server on the loopback interface.
struct LoopbackProvider {
    address: String,
    received: Arc<Mutex<Option<String>>>,
    handle: Option<thread::JoinHandle<()>>,
}

impl LoopbackProvider {
    fn start(status_line: &str, body: &str) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        let received = Arc::new(Mutex::new(None));
        let captured = received.clone();
        let status_line = status_line.to_string();
        let body = body.to_string();
        let handle = thread::spawn(move || {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            let accepted = loop {
                match listener.accept() {
                    Ok(connection) => break Some(connection),
                    Err(error)
                        if error.kind() == std::io::ErrorKind::WouldBlock
                            && std::time::Instant::now() < deadline =>
                    {
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                    Err(_) => break None,
                }
            };
            if let Some((mut stream, _)) = accepted {
                let mut buffer = [0u8; 8192];
                let mut request = Vec::new();
                loop {
                    match stream.read(&mut buffer) {
                        Ok(0) | Err(_) => break,
                        Ok(read) => {
                            request.extend_from_slice(&buffer[..read]);
                            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                                break;
                            }
                        }
                    }
                }
                *captured.lock().unwrap() = Some(String::from_utf8_lossy(&request).to_string());
                let response = format!(
                    "{status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.flush();
            }
        });
        Self {
            address,
            received,
            handle: Some(handle),
        }
    }

    fn received(&self) -> String {
        self.received
            .lock()
            .unwrap()
            .clone()
            .expect("the loopback provider received a request")
    }
}

impl Drop for LoopbackProvider {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn completion(content: &str) -> String {
    json!({ "choices": [{ "message": { "role": "assistant", "content": content } }] }).to_string()
}

fn request_to(endpoint: String) -> CompletionRequest {
    CompletionRequest {
        endpoint,
        model: "loopback-model".to_string(),
        api_key: SecretKey::new(DUMMY_KEY),
        body: json!({ "model": "loopback-model", "messages": [] }).to_string(),
    }
}

#[test]
fn the_real_adapter_posts_once_with_a_bearer_key_and_returns_the_body() {
    let provider = LoopbackProvider::start("HTTP/1.1 200 OK", &completion("{\"budgetMinor\":1}"));
    let endpoint = format!("{}/v1/chat/completions", provider.address);
    let body = OpenAiCompatibleProvider
        .complete(&request_to(endpoint))
        .unwrap();
    assert!(body.contains("budgetMinor"));

    let received = provider.received();
    assert!(
        received.starts_with("POST /v1/chat/completions "),
        "{received}"
    );
    assert!(
        received
            .to_ascii_lowercase()
            .contains(&format!("authorization: bearer {DUMMY_KEY}").to_ascii_lowercase()),
        "{received}"
    );
}

#[test]
fn a_redirect_is_refused_rather_than_followed() {
    let provider = LoopbackProvider::start(
        "HTTP/1.1 307 Temporary Redirect",
        &json!({ "location": "http://127.0.0.1:1/elsewhere" }).to_string(),
    );
    let failure = OpenAiCompatibleProvider
        .complete(&request_to(format!(
            "{}/v1/chat/completions",
            provider.address
        )))
        .unwrap_err();
    assert_eq!(failure.code, "ai_provider_error");
    assert!(failure.message.contains("redirect"));
}

#[test]
fn a_provider_status_is_reported_without_its_body() {
    let provider = LoopbackProvider::start(
        "HTTP/1.1 500 Internal Server Error",
        "{\"error\":\"internal detail that must not be echoed\"}",
    );
    let failure = OpenAiCompatibleProvider
        .complete(&request_to(format!(
            "{}/v1/chat/completions",
            provider.address
        )))
        .unwrap_err();
    assert_eq!(failure.code, "ai_provider_error");
    assert_eq!(failure.status, 502);
    assert!(!failure.message.contains("internal detail"));
}

#[test]
fn a_closed_port_is_unreachable() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);
    let failure = OpenAiCompatibleProvider
        .complete(&request_to(format!("http://{address}/v1/chat/completions")))
        .unwrap_err();
    assert_eq!(failure.code, "ai_provider_unreachable");
    assert_eq!(failure.status, 502);
}

struct Harness {
    _dir: TempDir,
    router: axum::Router,
    snapshot_id: String,
}

impl Harness {
    async fn new() -> Self {
        let dir = TempDir::new().unwrap();
        let store = Arc::new(Store::open(&dir.path().join("inventory.sqlite3")).unwrap());
        let router = build_router(AppState::new(store, dir.path().join("missing-web")));
        let boundary = "rigcraft-transport-boundary";
        let csv = std::fs::read(Path::new("tests/fixtures/recommendation/feasible.csv")).unwrap();
        let mut form = Vec::new();
        form.extend_from_slice(
            format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"feasible.csv\"\r\nContent-Type: text/csv\r\n\r\n"
            )
            .as_bytes(),
        );
        form.extend_from_slice(&csv);
        form.extend_from_slice(
            format!(
                "\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"currency\"\r\n\r\nUSD\r\n--{boundary}--\r\n"
            )
            .as_bytes(),
        );
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/imports")
                    .header(
                        header::CONTENT_TYPE,
                        format!("multipart/form-data; boundary={boundary}"),
                    )
                    .body(Body::from(form))
                    .unwrap(),
            )
            .await
            .unwrap();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let preview: Value = serde_json::from_slice(&bytes).unwrap();
        let preview_id = preview["previewId"].as_str().unwrap().to_string();
        let decisions = preview["rows"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| json!({ "row": row["row"], "action": "accept" }))
            .collect::<Vec<_>>();
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/imports/{preview_id}/commit"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({ "decisions": decisions, "duplicateResolutions": [] }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let committed: Value = serde_json::from_slice(&bytes).unwrap();
        Self {
            _dir: dir,
            router,
            snapshot_id: committed["snapshotId"].as_str().unwrap().to_string(),
        }
    }

    async fn parse(&self, provider: &LoopbackProvider) -> (StatusCode, Value) {
        let body = json!({
            "text": "Build a $2000 USD gaming PC from only my stock.",
            "snapshotId": self.snapshot_id,
            "apiKey": DUMMY_KEY,
            "baseUrl": format!("{}/v1", provider.address),
            "model": "loopback-model",
            "includeInventorySummary": false,
        });
        let response = self
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/ai/parse")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (
            status,
            serde_json::from_str(&String::from_utf8_lossy(&bytes)).unwrap_or(Value::Null),
        )
    }
}

#[tokio::test]
async fn the_api_uses_the_real_adapter_against_a_loopback_provider() {
    let harness = Harness::new().await;
    let content = json!({
        "budgetMinor": { "value": 200000, "excerpt": "$2000" },
        "currency": { "value": "USD", "excerpt": "USD" },
        "useCase": { "value": "gaming", "excerpt": "gaming" }
    })
    .to_string();
    let provider = LoopbackProvider::start("HTTP/1.1 200 OK", &completion(&content));
    let (status, proposal) = harness.parse(&provider).await;
    assert_eq!(status, StatusCode::OK, "{proposal}");
    assert_eq!(proposal["mode"], "provider");
    assert_eq!(proposal["provider"]["kind"], "openai-compatible");
    assert_eq!(proposal["proposal"]["budgetMinor"]["value"], 200000);
    assert_eq!(proposal["request"]["snapshotId"], harness.snapshot_id);
    let received = provider.received();
    assert!(received.contains("POST /v1/chat/completions"));
    assert!(received
        .to_ascii_lowercase()
        .contains(&format!("authorization: bearer {DUMMY_KEY}").to_ascii_lowercase()));
}

#[tokio::test]
async fn the_api_maps_an_unreadable_reply_to_422() {
    let harness = Harness::new().await;
    let provider = LoopbackProvider::start("HTTP/1.1 200 OK", "{\"unexpected\":true}");
    let (status, error) = harness.parse(&provider).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(error["error"]["code"], "ai_response_invalid");
}

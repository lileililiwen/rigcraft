//! Every rejected request uses the documented JSON error envelope, including
//! failures raised by the body extractors before a handler runs.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use rigcraft::store::Store;
use rigcraft::{build_router, AppState};
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

    async fn post(
        &self,
        uri: &str,
        content_type: Option<&str>,
        body: impl Into<Body>,
    ) -> (StatusCode, Option<String>, serde_json::Value) {
        let mut builder = Request::builder().method("POST").uri(uri);
        if let Some(content_type) = content_type {
            builder = builder.header(header::CONTENT_TYPE, content_type);
        }
        let response = self
            .router
            .clone()
            .oneshot(builder.body(body.into()).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .map(|value| value.to_str().unwrap().to_string());
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let value = serde_json::from_slice(&bytes)
            .unwrap_or_else(|_| serde_json::json!({ "raw": String::from_utf8_lossy(&bytes) }));
        (status, content_type, value)
    }
}

#[tokio::test]
async fn wrongly_typed_json_is_a_json_error_envelope() {
    let harness = Harness::new();
    for (uri, payload) in [
        ("/api/builds/check", r#"{"selection":[{"quantity":1.5}]}"#),
        ("/api/recommendations", r#"{"budgetMinor":"many"}"#),
        ("/api/diagnoses", r#"{"build":"not-a-list"}"#),
        (
            "/api/inventory/snapshots/anything/corrections",
            r#"{"changes":"not-a-list"}"#,
        ),
    ] {
        let (status, content_type, body) =
            harness.post(uri, Some("application/json"), payload).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{uri}");
        assert_eq!(content_type.as_deref(), Some("application/json"), "{uri}");
        assert_eq!(body["error"]["code"], "invalid_json", "{uri}: {body}");
    }
}

#[tokio::test]
async fn malformed_json_is_a_json_error_envelope() {
    let harness = Harness::new();
    let (status, content_type, body) = harness
        .post(
            "/api/builds/check",
            Some("application/json"),
            Body::from("{bad"),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(content_type.as_deref(), Some("application/json"));
    assert_eq!(body["error"]["code"], "invalid_json");
    assert!(body["error"]["message"].is_string());
}

#[tokio::test]
async fn a_missing_json_content_type_is_an_envelope() {
    let harness = Harness::new();
    let (status, content_type, body) = harness
        .post(
            "/api/builds/check",
            None,
            Body::from(r#"{"snapshotId":"x","selection":[]}"#),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(content_type.as_deref(), Some("application/json"));
    assert_eq!(body["error"]["code"], "invalid_body");
}

#[tokio::test]
async fn a_malformed_multipart_boundary_is_an_envelope() {
    let harness = Harness::new();
    let (status, content_type, body) = harness
        .post(
            "/api/imports",
            Some("multipart/form-data"),
            Body::from("not a multipart body"),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(content_type.as_deref(), Some("application/json"));
    assert_eq!(body["error"]["code"], "invalid_body");
}

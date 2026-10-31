//! Static and SPA fallback routing: a client route gets the shell, a missing
//! file or metadata path gets a real 404.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use rigcraft::store::Store;
use rigcraft::{build_router, AppState};
use tempfile::TempDir;
use tower::ServiceExt;

const MARKER: &str = "<title>rigcraft-spa-shell</title>";

struct Harness {
    _dir: TempDir,
    router: Router,
}

impl Harness {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let web_dir = dir.path().join("web");
        std::fs::create_dir_all(&web_dir).unwrap();
        std::fs::write(
            web_dir.join("index.html"),
            format!("<!doctype html><html><head>{MARKER}</head><body></body></html>"),
        )
        .unwrap();
        let store = Arc::new(Store::open(&dir.path().join("inventory.sqlite3")).unwrap());
        let router = build_router(AppState::new(store, web_dir));
        Self { _dir: dir, router }
    }

    async fn get(&self, uri: &str) -> (StatusCode, Option<String>, String) {
        let response = self
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(uri)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .map(|value| value.to_str().unwrap().to_string());
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (
            status,
            content_type,
            String::from_utf8_lossy(&bytes).to_string(),
        )
    }
}

#[tokio::test]
async fn extensionless_client_routes_return_the_spa_shell() {
    let harness = Harness::new();
    for uri in [
        "/",
        "/inventory",
        "/builds/check",
        "/recommend",
        "/diagnose",
        "/some/history/route",
    ] {
        let (status, content_type, body) = harness.get(uri).await;
        assert_eq!(status, StatusCode::OK, "{uri}");
        assert!(
            content_type.unwrap_or_default().starts_with("text/html"),
            "{uri} was not HTML"
        );
        assert!(body.contains(MARKER), "{uri} did not return the shell");
    }
}

#[tokio::test]
async fn unknown_static_and_metadata_paths_are_not_html() {
    let harness = Harness::new();
    for uri in [
        "/assets/nope.js",
        "/assets/nope",
        "/robots.txt",
        "/llms.txt",
        "/ai-catalog.json",
        "/missing.html",
        "/favicon.ico",
    ] {
        let (status, _content_type, body) = harness.get(uri).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{uri}");
        assert!(
            !body.contains(MARKER),
            "{uri} returned the SPA shell instead of 404"
        );
    }
}

#[tokio::test]
async fn unknown_api_routes_stay_json_not_found() {
    let harness = Harness::new();
    let (status, content_type, body) = harness.get("/api/nope").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(content_type.as_deref(), Some("application/json"));
    let value: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(value["error"]["code"], "route_not_found");
}

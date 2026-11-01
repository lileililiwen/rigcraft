//! HTTP surface routing: the six client routes get the shell, the API root and
//! every unknown API route are JSON 404s, and any other path is a real 404.

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
        self.send("GET", uri).await
    }

    async fn send(&self, method: &str, uri: &str) -> (StatusCode, Option<String>, String) {
        let response = self
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
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
async fn the_six_client_routes_return_the_spa_shell() {
    let harness = Harness::new();
    for uri in [
        "/",
        "/inventory",
        "/catalog",
        "/builds/check",
        "/recommend",
        "/diagnose",
        "/inventory/",
        "/diagnose/",
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
async fn an_unknown_extensionless_path_is_a_real_404_document() {
    let harness = Harness::new();
    for uri in [
        "/nope",
        "/recomend",
        "/some/history/route",
        "/API/health",
        "/inventory//",
    ] {
        let (status, content_type, body) = harness.get(uri).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{uri}");
        assert!(
            !body.contains(MARKER),
            "{uri} returned the SPA shell instead of 404"
        );
        assert!(
            body.contains("404 Not found"),
            "{uri} did not return the not-found document"
        );
        assert!(
            !content_type.unwrap_or_default().starts_with("text/html"),
            "{uri} returned an HTML document"
        );
    }
}

#[tokio::test]
async fn the_api_root_and_unknown_api_routes_are_json_404s() {
    let harness = Harness::new();
    for uri in ["/api", "/api/", "/api/nope", "/api/inventory/nope/deeper"] {
        let (status, content_type, body) = harness.get(uri).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{uri}");
        assert_eq!(content_type.as_deref(), Some("application/json"), "{uri}");
        let value: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(value["error"]["code"], "route_not_found", "{uri}");
    }
}

#[tokio::test]
async fn a_wrong_method_on_a_real_api_route_keeps_its_405() {
    let harness = Harness::new();
    let (status, _content_type, body) = harness.send("POST", "/api/health").await;
    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
    assert!(!body.contains(MARKER));
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

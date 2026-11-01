//! Delivery headers: one policy covers pages, API responses, static assets and
//! the 404 document, and compression is negotiated rather than always on.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, HeaderMap, Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use rigcraft::store::Store;
use rigcraft::{build_router, AppState};
use tempfile::TempDir;
use tower::ServiceExt;

const SHELL: &str = "<!doctype html><html><head><title>rigcraft delivery shell</title></head>\
<body><div id=\"root\"></div>\
<p>A shell long enough that compression beats its framing.</p>\
<p>A shell long enough that compression beats its framing.</p>\
<p>A shell long enough that compression beats its framing.</p>\
<p>A shell long enough that compression beats its framing.</p>\
<p>A shell long enough that compression beats its framing.</p>\
<p>A shell long enough that compression beats its framing.</p>\
</body></html>";
const SCRIPT: &str = "console.log('delivery hardening asset with enough bytes to compress');\n\
console.log('delivery hardening asset with enough bytes to compress');\n\
console.log('delivery hardening asset with enough bytes to compress');\n\
console.log('delivery hardening asset with enough bytes to compress');\n\
console.log('delivery hardening asset with enough bytes to compress');\n";
const ICON: &[u8] = &[0x00, 0x00, 0x01, 0x00, 0x20, 0x20];

struct Harness {
    _dir: TempDir,
    router: Router,
}

impl Harness {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let web_dir = dir.path().join("web");
        std::fs::create_dir_all(web_dir.join("assets")).unwrap();
        std::fs::write(web_dir.join("index.html"), SHELL).unwrap();
        std::fs::write(web_dir.join("assets/index-abc123.js"), SCRIPT).unwrap();
        std::fs::write(web_dir.join("favicon.ico"), ICON).unwrap();
        let store = Arc::new(Store::open(&dir.path().join("inventory.sqlite3")).unwrap());
        let router = build_router(AppState::new(store, web_dir));
        Self { _dir: dir, router }
    }

    async fn get(&self, uri: &str) -> (StatusCode, HeaderMap, Vec<u8>) {
        self.request(uri, HeaderMap::new()).await
    }

    async fn request(&self, uri: &str, headers: HeaderMap) -> (StatusCode, HeaderMap, Vec<u8>) {
        let mut request = Request::builder().method("GET").uri(uri);
        for (name, value) in &headers {
            request = request.header(name, value);
        }
        let response = self
            .router
            .clone()
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let headers = response.headers().clone();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (status, headers, bytes.to_vec())
    }
}

fn encoding(value: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(header::ACCEPT_ENCODING, value.parse().unwrap());
    headers
}

fn header(headers: &HeaderMap, name: header::HeaderName) -> &str {
    headers
        .get(&name)
        .unwrap_or_else(|| panic!("missing header {name}"))
        .to_str()
        .unwrap()
}

fn assert_security_headers(headers: &HeaderMap) {
    assert_eq!(header(headers, header::X_CONTENT_TYPE_OPTIONS), "nosniff");
    assert_eq!(header(headers, header::REFERRER_POLICY), "no-referrer");
    assert_eq!(header(headers, header::X_FRAME_OPTIONS), "DENY");
    let csp = header(headers, header::CONTENT_SECURITY_POLICY);
    for directive in [
        "default-src 'self'",
        "script-src 'self'",
        "style-src 'self' 'unsafe-inline'",
        "img-src 'self' data:",
        "font-src 'self'",
        "connect-src 'self' https: http://127.0.0.1:* http://localhost:*",
        "frame-ancestors 'none'",
        "base-uri 'none'",
        "object-src 'none'",
        "form-action 'self'",
    ] {
        assert!(csp.contains(directive), "CSP lacks {directive}: {csp}");
    }
    assert!(
        !csp.contains("'unsafe-eval'"),
        "the policy must not allow eval: {csp}"
    );
    assert!(
        !csp.contains("script-src 'self' 'unsafe-inline'"),
        "no inline script is allowed: {csp}"
    );
}

#[tokio::test]
async fn every_response_carries_the_baseline_security_policy() {
    let harness = Harness::new();
    for uri in [
        "/recommend",
        "/api/health",
        "/nope",
        "/assets/index-abc123.js",
    ] {
        let (status, headers, _body) = harness.get(uri).await;
        assert!(
            status == StatusCode::OK || status == StatusCode::NOT_FOUND,
            "{uri} returned {status}"
        );
        assert_security_headers(&headers);
    }
}

#[tokio::test]
async fn cache_policy_is_decided_once_for_assets_pages_api_and_favicon() {
    let harness = Harness::new();

    let (status, headers, _) = harness.get("/assets/index-abc123.js").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        header(&headers, header::CACHE_CONTROL),
        "public, max-age=31536000, immutable"
    );

    for page in ["/", "/inventory", "/recommend"] {
        let (status, headers, _) = harness.get(page).await;
        assert_eq!(status, StatusCode::OK, "{page}");
        assert_eq!(
            header(&headers, header::CACHE_CONTROL),
            "no-cache",
            "{page}"
        );
    }

    let (status, headers, _) = harness.get("/favicon.ico").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        header(&headers, header::CACHE_CONTROL),
        "public, max-age=86400"
    );

    let (status, headers, _) = harness.get("/api/health").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(header(&headers, header::CACHE_CONTROL), "no-store");

    let (status, headers, _) = harness.get("/nope").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(
        headers.get(header::CACHE_CONTROL).is_none(),
        "a 404 document is not a cacheable page"
    );

    let (status, headers, _) = harness.get("/assets/missing.js").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(
        headers.get(header::CACHE_CONTROL).is_none(),
        "a missing asset must not be marked immutable"
    );
}

#[tokio::test]
async fn gzip_and_brotli_are_negotiated_and_vary_is_declared() {
    let harness = Harness::new();

    for (accept, expected) in [("gzip", "gzip"), ("br", "br")] {
        for uri in ["/recommend", "/assets/index-abc123.js"] {
            let (status, headers, body) = harness.request(uri, encoding(accept)).await;
            assert_eq!(status, StatusCode::OK, "{uri} with {accept}");
            assert_eq!(
                header(&headers, header::CONTENT_ENCODING),
                expected,
                "{uri} with {accept}"
            );
            assert!(
                header(&headers, header::VARY).contains("accept-encoding"),
                "{uri} with {accept} lacks Vary"
            );
            assert!(
                body.len() < SHELL.len().max(SCRIPT.len()),
                "{uri} with {accept} was not smaller than the plain bytes"
            );
            assert!(
                headers.get(header::CONTENT_LENGTH).is_none(),
                "{uri} with {accept} kept a length that no longer describes the body"
            );
        }
    }

    // A small payload is not made larger by framing it.
    let (status, headers, body) = harness.request("/api/health", encoding("gzip")).await;
    assert_eq!(status, StatusCode::OK);
    assert!(headers.get(header::CONTENT_ENCODING).is_none());
    assert!(header(&headers, header::VARY).contains("accept-encoding"));
    assert_eq!(
        header(&headers, header::CONTENT_LENGTH)
            .parse::<usize>()
            .unwrap(),
        body.len()
    );
}

#[tokio::test]
async fn an_unrequested_encoding_is_identity_with_a_usable_length() {
    let harness = Harness::new();
    for uri in ["/recommend", "/assets/index-abc123.js"] {
        let (status, headers, body) = harness.get(uri).await;
        assert_eq!(status, StatusCode::OK);
        assert!(headers.get(header::CONTENT_ENCODING).is_none(), "{uri}");
        assert!(header(&headers, header::VARY).contains("accept-encoding"));
        assert_eq!(
            header(&headers, header::CONTENT_LENGTH)
                .parse::<usize>()
                .unwrap(),
            body.len(),
            "{uri} length disagrees with its body"
        );
    }
}

#[tokio::test]
async fn an_already_compressed_icon_is_never_reencoded() {
    let harness = Harness::new();
    let (status, headers, body) = harness.request("/favicon.ico", encoding("gzip, br")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, ICON);
    assert!(headers.get(header::CONTENT_ENCODING).is_none());
    assert_eq!(
        header(&headers, header::CONTENT_LENGTH)
            .parse::<usize>()
            .unwrap(),
        ICON.len()
    );
}

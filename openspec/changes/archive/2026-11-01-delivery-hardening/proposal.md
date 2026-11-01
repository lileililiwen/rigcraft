# Proposal: Send the same bytes efficiently under a defensible header policy

## Why

The service sends every response uncompressed, without a cache policy and
without security headers. The production bundle is 307,491 bytes of JavaScript
plus 27,698 bytes of CSS on every first visit, and no response tells a browser
whether it may keep them. `index.html` is served with no `Cache-Control`, so a
browser may reuse a stale shell after a deploy, and the hashed `/assets/*` files
are revalidated on every load because nothing marks them immutable.

The app holds private inventory and a provider key typed by the operator. A
response that omits `X-Content-Type-Options`, `Referrer-Policy`,
`X-Frame-Options` and a Content-Security-Policy leaves MIME sniffing, referrer
leakage and framing available to any page that can load or embed it. There is
also no measured performance number anywhere in `docs/evidence/`, so "it is
fast" is currently a claim rather than a fact.

## What Changes

- **Compression.** The router negotiates `gzip` and `br` through `tower-http`'s
  `CompressionLayer` for `text/*`, `application/json`,
  `application/javascript` and `image/svg+xml`, and declares
  `Vary: Accept-Encoding` on those responses. `image/x-icon` and other images
  stay uncompressed; a range response is never compressed and a compressed
  response drops its now-wrong `Content-Length`. Only the existing
  `tower-http` dependency gains the `compression-gzip` and `compression-br`
  features; zstd is not enabled.
- **Cache policy.** One response layer decides the policy: hashed `/assets/*`
  are `public, max-age=31536000, immutable`, `index.html` and every HTML route
  are `no-cache`, the favicon is `public, max-age=86400`, and `/api/*` JSON is
  `no-store`. No handler sets its own `Cache-Control`.
- **Security headers.** Every response, including an API error, a static asset
  and the 404 document, carries `X-Content-Type-Options: nosniff`,
  `Referrer-Policy: no-referrer`, `X-Frame-Options: DENY` and the CSP
  `default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'self' https: http://127.0.0.1:* http://localhost:*; frame-ancestors 'none'; base-uri 'none'; object-src 'none'; form-action 'self'`.
  The build has no inline script; each allowance is justified in `design.md`.
- **Measured performance.** `/recommend` is traced once against the production
  build on a scratch data directory, before and after compression, and LCP, CLS
  and transferred bytes are recorded in
  `docs/evidence/delivery-hardening-browser.md`. No score is claimed that was
  not measured.

## Impact

- Affected specs: `inventory-import`, which owns the service surface, gains
  three requirements: negotiated compression, a cache policy, and the baseline
  security header policy.
- Affected code: `Cargo.toml` (two `tower-http` features), `Cargo.lock`
  (transitive crates only), `src/api.rs` (the response layer and compression
  layer), `tests/delivery_headers.rs`, `e2e/delivery.spec.ts`, `docs/`.
- Affected behaviour: response headers and body encoding only. Every route,
  status code, content type, API field, error code, schema, engine value,
  copy string and render tree stays the same; an uncompressed request receives
  byte-identical bodies to today.
- Non-goals: no new direct dependency, no zstd, no CDN, no analytics, no
  external font or third-party script, no redirect, no change to the six-route
  allowlist or the real 404s.

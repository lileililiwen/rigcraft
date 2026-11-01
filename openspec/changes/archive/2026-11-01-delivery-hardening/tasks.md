# Tasks: Send the same bytes efficiently under a defensible header policy

## 1. BFS — Baseline, impact and measurement
- [x] 1.1 Read `AGENTS.md`, `.ai-rules/`, `HANDOFF.md`, `docs/design-program.md` (package 6), `Cargo.toml`, `src/api.rs`, `src/lib.rs`, `tests/spa_routes.rs`, the e2e suite and the browser evidence format; confirm no active change and set `current_spec`.
- [x] 1.2 Build the production bundle and the service, start them on `127.0.0.1:33210` with a scratch data directory, seed the shipped sample snapshot `fa559ef5-104b-49c1-a5d3-df299e58bbbf`, and record the baseline `curl -i` matrix: no `Content-Encoding`, no `Cache-Control`, no security headers.
- [x] 1.3 Trace `/recommend` once before the change with the Chrome DevTools MCP `performance_start_trace` tool (cold cache) and record LCP 69 ms, CLS 0.01, 364,831 transferred bytes.
- [x] 1.4 Map impact: `src/api.rs` router layers and both fallbacks, `Cargo.toml`/`Cargo.lock`, the six client routes, every API response, the static files, the 404 document, `tests/spa_routes.rs`, `e2e/`, the CSP-sensitive assistant transport and the existing Lighthouse/accessibility evidence.
- [x] 1.5 Assess shared infrastructure by name: `tower-http` (already a dependency, gains two features), the axum layer stack, `ServeDir`, the Playwright suite and the pinned token kit; no new direct dependency is added.

## 2. DFS — Compression
- [x] 2.1 Enable `tower-http` `compression-gzip` and `compression-br`; do not enable zstd.
- [x] 2.2 Apply `CompressionLayer` to the finished router so the API, static files and SPA fallback are all covered, with a 256-byte floor and the image/gRPC/SSE exclusions.
- [x] 2.3 Keep `Vary: Accept-Encoding` on every text response without duplicating the header, and leave `Content-Length`/range behaviour correct: compressed bodies drop the stale length, range responses are never compressed.

## 3. DFS — Cache and security policy
- [x] 3.1 Add one `delivery_policy` middleware outside the compression layer, so every response — page, API, asset, 404 — receives the policy.
- [x] 3.2 Decide the cache policy in one function: immutable `/assets/*`, `no-cache` HTML, one-day favicon, `no-store` `/api`, and nothing for a 404 or missing asset.
- [x] 3.3 Add `X-Content-Type-Options: nosniff`, `Referrer-Policy: no-referrer`, `X-Frame-Options: DENY` and the documented CSP to every response.
- [x] 3.4 Confirm `apps/web/index.html` has no inline `<script>` and that the CSP allowances match the bundle, the icon/font positions and the BYOK provider URL rule.

## 4. BFS — Tests
- [x] 4.1 Add `tests/delivery_headers.rs`: the header matrix on a page, an API route, a 404, the API 404 and an asset; the cache matrix; `gzip`/`br` negotiation with `Vary`; identity when no encoding is requested; the icon never re-encoded; a sub-floor response never inflated.
- [x] 4.2 Add `e2e/delivery.spec.ts`: every route renders with no CSP console violation, a typo still returns a real 404, and the assistant still reaches a loopback provider through the service.
- [x] 4.3 Keep the six-route allowlist, the real 404s and every existing journey assertion unchanged.

## 5. BFS — Documentation and evidence
- [x] 5.1 Write `docs/delivery-headers.md` (the policy, each CSP allowance, the cache table and the measured byte matrix) and cross-reference it from `docs/inventory-import.md`.
- [x] 5.2 Record the real `curl -i` headers with and without compression, the identity/gzip/br sizes, the decompressed-body hash and the CSP/browser verdict in `docs/evidence/delivery-hardening-browser.md`.
- [x] 5.3 Record the performance trace: LCP and CLS before and after, cold and warm transferred bytes, and the honest statement that the byte reduction is measured while the LCP difference is single-run lab variance.
- [x] 5.4 Update `docs/design-program.md` (package 6 delivered), `docs/evidence/browser-suite.md` (28 tests), `docs/assets/capture-plan.md` (headers are invisible, no new capture) and README if a claim changed.

## 6. Verification and close-out
- [x] 6.1 `cargo fmt --all -- --check`; `cargo clippy --all-targets --all-features -- -D warnings`; `cargo test`; `npm run check`; `npm test`; `npm run build`; `npm run test:e2e`; `./verify-local.sh`.
- [x] 6.2 `node scripts/check-openspec-change-names.mjs`; `openspec validate delivery-hardening --strict`; `openspec validate --all --strict`.
- [x] 6.3 Run the Forge full Gate and record its aggregate verdict and labelled revision.
- [x] 6.4 Archive with `openspec archive delivery-hardening --yes` (no `--skip-specs`), commit everything except `HANDOFF.md`, then commit `HANDOFF.md` only; do not push.

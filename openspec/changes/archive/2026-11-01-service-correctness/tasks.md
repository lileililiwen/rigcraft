# Tasks: Make the HTTP surface and the CSV export honest

## 1. BFS — Baseline, impact and reproduction
- [x] 1.1 Read `AGENTS.md`, `.ai-rules/`, `HANDOFF.md`, `docs/design-program.md` (package 5), `src/api.rs`, `src/portability.rs`, `src/csv_import.rs`, `src/error.rs`, the HTTP test suites, `apps/web/src/App.tsx`, `docs/data-portability.md` and `docs/inventory-import.md`; confirm no active change and set `current_spec`.
- [x] 1.2 Reproduce all four defects against the built app on a free port with a scratch data directory and record the `curl -i` matrix: `/api` 200 HTML, `/api/` 200 HTML, `/API/health` 200 HTML, `/nope` 200 HTML (Inventory), `/recomend` 200 HTML, a `=1+1` model exporting raw, and a semicolon CSV reporting only the missing columns.
- [x] 1.3 Map impact: `tests/spa_routes.rs` (it asserts `/some/history/route` returns the shell), `e2e/regression.spec.ts` and `e2e/shell.spec.ts`, the six `App.tsx` routes, `x-rigcraft-export-*` headers, the CSV round-trip journey, and every assertion that names `mapping_missing`.
- [x] 1.4 Assess shared infrastructure by name (the axum fallback/ServeDir pair, `csv` crate, `ApiError` envelope, the Playwright suite, the pinned token kit) and record that all are consumed unchanged, with no new dependency.

## 2. DFS — API surface and client routes
- [x] 2.1 Mount `/api` and `/api/` with the existing `api_not_found` so the API root and its trailing-slash form return the JSON `route_not_found` 404.
- [x] 2.2 Replace the inferred `is_client_route` with the exact six-route allowlist plus optional trailing slash, keeping `/assets/*` and dotted paths non-client.
- [x] 2.3 Return a static plain-text 404 document for every other extensionless path, without reflecting the request path and without a new dependency or colour literal.
- [x] 2.4 Keep `GET /api/health` 200 and the 405 for a wrong method on a real API route.

## 3. DFS — CSV neutralisation and delimiter hint
- [x] 3.1 Add cell neutralisation to `snapshot_csv` for a first character of `=`, `+`, `-`, `@`, TAB or CR, and for a leading single or double quote followed by one of those.
- [x] 3.2 Keep `snapshot_json` raw and document the rule and its lossiness in `docs/data-portability.md`.
- [x] 3.3 Name the likely semicolon delimiter in the `mapping_missing` refusal when the parsed header is a single column containing `;`, keeping the code, field and missing list.

## 4. BFS — Tests
- [x] 4.1 Add the Rust route matrix: `/api`, `/api/`, `/api/nope` JSON 404 `route_not_found`; `/nope`, `/recomend`, `/API/health` 404 non-shell; each of the six routes 200 HTML; `/inventory/` 200; `/assets/missing.js` 404; `POST /api/health` 405.
- [x] 4.2 Prove CSV neutralisation end to end: the CSV cell is `'=1+1`, the JSON value is `=1+1`, and the apostrophe is the only difference.
- [x] 4.3 Prove the delimiter hint keeps `mapping_missing` and its field while naming the semicolon, and that a comma CSV with a genuine missing column gets no hint.
- [x] 4.4 Add a Playwright journey: a direct visit to a typo path renders the real 404 rather than the Inventory page, and the existing journeys stay green.

## 5. BFS — Documentation and evidence
- [x] 5.1 Update `docs/data-portability.md` (neutralisation rule and lossiness), `docs/inventory-import.md` (delimiter hint and the route table) and `docs/design-program.md` (package 5 delivered).
- [x] 5.2 Build and run the app on a free port with a scratch data directory; capture the 404 document and the downloaded CSV at 1440 and 390 into `docs/assets/service-correctness-*.png`.
- [x] 5.3 Record the real `curl -i` status/content-type matrix and the exported cell bytes in `docs/evidence/service-correctness-browser.md`, and state why no Lighthouse audit applies to a plain-text 404.

## 6. Verification and close-out
- [x] 6.1 `cargo fmt --all -- --check`; `cargo clippy --all-targets --all-features -- -D warnings`; `cargo test`; `npm run check`; `npm test`; `npm run build`; `npm run test:e2e`; `./verify-local.sh`.
- [x] 6.2 `node scripts/check-openspec-change-names.mjs`; `openspec validate service-correctness --strict`; `openspec validate --all --strict`.
- [x] 6.3 Run the Forge full Gate and record its aggregate verdict and labelled revision.
- [x] 6.4 Archive with `openspec archive service-correctness --yes` (no `--skip-specs`), commit everything except `HANDOFF.md`, then commit `HANDOFF.md` only; do not push.

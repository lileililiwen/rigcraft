# Tasks: A checked-in browser regression suite

## 1. BFS — Baseline and impact coverage
- [x] 1.1 Read `AGENTS.md`, `.ai-rules/workflow.md`, `.ai-rules/completion.md`, `HANDOFF.md`, `ROADMAP.md`, `docs/data-sources.md` and every `docs/evidence/*-browser.md`; confirm no other active change and set `current_spec`.
- [x] 1.2 Map the claimed browser behaviour to code: the four routes in `apps/web/src/App.tsx`, the page components and their headings/ids, the API routes in `src/api.rs`, the served sample in `src/csv_import.rs`, and the 404/JSON-envelope/ISO-8601 fixes in `src/api.rs`, `src/error.rs` and `apps/web/src/inventory.ts`.
- [x] 1.3 Assess shared infrastructure by name: Playwright for the driver, Vitest for component state, Chrome DevTools MCP captures, Forge Gate; record that only Playwright is added.

## 2. DFS — Harness
- [x] 2.1 Add `@playwright/test` as a root devDependency via `npm install --save-dev @playwright/test`.
- [x] 2.2 Add `playwright.config.ts`: one chromium project on `channel: 'chrome'`, headless, `baseURL`, no retries, one worker, JSON + list reporters, trace on first retry, `webServer` polling `/api/health`.
- [x] 2.3 Add `e2e/start-server.mjs` (fresh temp data dir, suite port, signal forwarding, cleanup) and `e2e/run.mjs` (free-port selection, `npm run test:e2e` entry point).
- [x] 2.4 Add `e2e/fixtures.ts`: verified sample seed, explicit snapshot selection, and the 390 px overflow assertion.
- [x] 2.5 Add `/test-results/` to `.gitignore`; confirm no artifact or database can be committed.

## 3. DFS — Journeys, one at a time, each run as written
- [x] 3.1 `e2e/inventory.spec.ts`: download the sample, load it, preview, commit revision 1 with 10 records, assert the catalog-versus-operator provenance split and an ISO 8601 UTC `<time>`.
- [x] 3.2 `e2e/compatibility.spec.ts`: select the full seeded build, assert `12 compatible · 0 conflicts · 2 unknown`, the `Unknown` verdict badge, and the `cpu-bios` / `psu-capacity` follow-ups.
- [x] 3.3 `e2e/recommendation.spec.ts`: a budget that fits yields a ranked option under budget labelled `Unknown`; a budget below the minimum yields `Budget constraint` with its minimum.
- [x] 3.4 `e2e/diagnosis.spec.ts`: exact/ambiguous/unmatched lines, no coercion, the blocked review, then the unverified group with `Fine as is (0)` and no check mark.
- [x] 3.5 `e2e/regression.spec.ts`: static/metadata 404s, the JSON `invalid_json` envelope, `/diagnose` sections 1–5, and the ISO 8601 timestamp under `de-DE`.
- [x] 3.6 `e2e/keyboard-responsive.spec.ts`: a keyboard-only recommendation and `scrollWidth === clientWidth` at 390 px on all four routes.

## 4. Wiring and docs
- [x] 4.1 Add the `test:e2e` npm script and run it from `./verify-local.sh` with a log that is printed on failure.
- [x] 4.2 Update `AGENTS.md` Project facts and the README Runtime section with the exact command and its prerequisites.
- [x] 4.3 Replace the "Not done"/"no checked-in Playwright suite" limitation in the seven browser-evidence documents with what the suite now covers.
- [x] 4.4 Confirm `.project.json` still states a truthful verification claim and that README/AGENTS claims match the tree.

## 5. BFS — Cross-surface regression
- [x] 5.1 Run the full suite twice and confirm the runtime is stable and under two minutes.
- [x] 5.2 Recheck that no product source, contract, schema, token, deployment flag or sibling repository changed; no secret, generated directory or lockfile hand-edit is in the diff.

## 6. Verification and evidence
- [x] 6.1 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test`, `npm run check`, `npm test`, `npm run build`, `npm run test:e2e`, `./verify-local.sh`.
- [x] 6.2 Run `node scripts/check-openspec-change-names.mjs`, `openspec validate browser-regression-suite --strict` and `openspec validate --all --strict`.
- [x] 6.3 Run the Forge full Gate and record its aggregate verdict and revision.
- [x] 6.4 Archive with `openspec archive browser-regression-suite --yes` (no `--skip-specs`): commit everything except `HANDOFF.md`, then commit `HANDOFF.md` only; do not push.

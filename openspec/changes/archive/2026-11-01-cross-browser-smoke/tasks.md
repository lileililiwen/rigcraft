# Tasks: Firefox and WebKit smoke, and an assistive-technology review

## 1. BFS — Baseline and impact
- [x] 1.1 Read `AGENTS.md`, `.ai-rules/`, `HANDOFF.md`, `docs/gap-closure.md` (package 4), `playwright.config.ts`, `e2e/run.mjs`, `e2e/fixtures.ts`, `e2e/visual.spec.ts`, `e2e/keyboard-responsive.spec.ts`, `e2e/regression.spec.ts` and `e2e/delivery.spec.ts`; confirm `openspec list` reports no active change.
- [x] 1.2 Record the suite's shape (one worker, no retries, Chrome channel, one service and database) and the routes' titles, skip-link names, landmarks and live regions.
- [x] 1.3 Decide the owners: `browser-regression` for the smoke projects, `ui-foundation` for the assistive-technology review; no existing requirement modified.
- [x] 1.4 Decide the smoke subset: six routes, skip link, 390 px overflow, 404, seeded recommendation, inventory export.

## 2. DFS — Cross-browser smoke
- [x] 2.1 Add `e2e/smoke.spec.ts` with the six tests, reusing `seedSample`, `chooseSnapshot` and `expectNoHorizontalOverflow`; assert no pixels.
- [x] 2.2 Add `firefox` and `webkit` projects matching only `smoke.spec.ts`, and move `channel: 'chrome'` into the chromium project so the other engines do not inherit it.
- [x] 2.3 Install Firefox and WebKit with `npx playwright install firefox webkit`; record the exact download versions.
- [x] 2.4 Diagnose the WebKit host-dependency failure, install the six packages user-locally without sudo, and prove WebKit launches without any skip-ramp environment variable.
- [x] 2.5 Run the Firefox project, the WebKit project and the full `npm run test:e2e`; record pass counts and times for each.

## 3. DFS — Assistive-technology review
- [x] 3.1 Capture `locator("body").ariaSnapshot()` for all six routes from the production build with the shipped sample committed.
- [x] 3.2 Record the landmarks, heading order and every live region per route, before and after the route's primary action.
- [x] 3.3 Record the Tab sequence from the document start on `/recommend`, the skip link's focus move to `main`, and the programmatically focused result element after each action.
- [x] 3.4 Write `docs/evidence/assistive-technology-review.md`, stating at the top that it is not a screen-reader pass, listing the role/name gaps found and the exact NVDA/VoiceOver/JAWS script.

## 4. BFS — Evidence and wiring
- [x] 4.1 Write `docs/evidence/cross-browser-smoke.md`: the browser matrix, the install output, the Firefox/WebKit/Chrome results, the WebKit error and workaround, and the smoke coverage list.
- [x] 4.2 Install the two engines in CI before `verify-local.sh` so the declared runner runs the declared projects.
- [x] 4.3 Update `README.md`, `docs/evidence/browser-suite.md` and `AGENTS.md` Project facts; re-check the README claims against the tree.

## 5. Verification and close-out
- [x] 5.1 `cargo fmt --all -- --check`; `cargo clippy --all-targets --all-features -- -D warnings`; `cargo test`; `npm run build`; `npm run check`; `npm test`; `npm run test:e2e`; `./verify-local.sh`.
- [x] 5.2 `node scripts/check-openspec-change-names.mjs`; `openspec validate cross-browser-smoke --strict`; `openspec validate --all --strict`.
- [x] 5.3 Run the Forge full Gate and record its aggregate verdict and labelled revision.
- [x] 5.4 Archive with `openspec archive cross-browser-smoke --yes` (no `--skip-specs`), commit everything except `HANDOFF.md`, then commit `HANDOFF.md` only; do not push.

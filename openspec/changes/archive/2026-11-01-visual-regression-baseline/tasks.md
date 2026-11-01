# Tasks: Pixel baselines that fail on an unintended visual change

## 1. BFS — Baseline and impact
- [x] 1.1 Read `AGENTS.md`, `.ai-rules/`, `HANDOFF.md`, `docs/gap-closure.md` (package 3), the two owning specs and every existing `e2e/*.spec.ts`; confirm `openspec list` reports no active change.
- [x] 1.2 Record the baseline build (asset names and sizes) and the suite's shape: one worker, no retries, real Chrome, alphabetical spec order, one shared server and database.
- [x] 1.3 Map the dynamic values per route: snapshot/report identifiers, committed/checked timestamps, sha256 fragment, the `<select>` option label and the catalog import time.
- [x] 1.4 Decide the owning specs: `ui-foundation` holds the interface evidence and mask policy; `browser-regression` holds the suite wiring and the failure artifact.

## 2. DFS — The visual spec
- [x] 2.1 Add `syntheticCatalog()` and `resetCatalog()` to `e2e/fixtures.ts`, reusing the synthetic document already driven by `e2e/catalog.spec.ts`; keep that spec's assertions unchanged.
- [x] 2.2 Add `e2e/visual.spec.ts` with one test per route per viewport (12 tests), `deviceScaleFactor: 1`, `locale: en-US`, `timezoneId: UTC`, `animations: 'disabled'`, `caret: 'hide'`, `threshold: 0.2`, `maxDiffPixelRatio: 0.01`.
- [x] 2.3 Seed state immediately before each capture: commit the shipped sample for `/inventory`, `/builds/check`, `/recommend` and `/diagnose`; reset then upload the synthetic catalog for `/catalog`; load `/nope` for the 404.
- [x] 2.4 Drive the real result where the route has one: a full AM5 check, a 1,800.00 gaming recommendation and an all-evaluated diagnosis; scroll each to its fixed anchor.
- [x] 2.5 Apply the mask list by block-level locator; confirm no dynamic value remains in the capture.
- [x] 2.6 Generate the twelve baselines with `npm run test:e2e -- e2e/visual.spec.ts --update-snapshots`.

## 3. DFS — Determinism and sensitivity proof
- [x] 3.1 Run the visual spec twice in a row against the same database and the same tree; both runs pass.
- [x] 3.2 Run the full `npm run test:e2e` on a fresh database; the visual captures still match and the existing journeys stay green.
- [x] 3.3 Mutate one brand surface token, rebuild, run the visual spec; record the failure and the diff artifact, then restore, rebuild and show it passes again.
- [x] 3.4 Restore the baselines to the unmodified bytes and confirm the tree is clean of the mutation.

## 4. BFS — Docs and evidence
- [x] 4.1 Write `docs/evidence/visual-regression.md`: the twelve baseline paths, the exact commands, the two clean runs, the mutation and its reported diff, the restored pass, the mask list, the generating platform and the update policy.
- [x] 4.2 Update `README.md` (suite count, the visual spec, the update command) and `docs/evidence/browser-suite.md` (the visual row and the new count); state that no Lighthouse run applies.
- [x] 4.3 Update `AGENTS.md` Project facts only if a directory, tool or command changed.

## 5. Verification and close-out
- [x] 5.1 `cargo fmt --all -- --check`; `cargo clippy --all-targets --all-features -- -D warnings`; `cargo test`; `npm run build`; `npm run check`; `npm test`; `npm run test:e2e`; `./verify-local.sh`.
- [x] 5.2 `node scripts/check-openspec-change-names.mjs`; `openspec validate visual-regression-baseline --strict`; `openspec validate --all --strict`.
- [x] 5.3 Run the Forge full Gate and record its aggregate verdict and labelled revision.
- [x] 5.4 Archive with `openspec archive visual-regression-baseline --yes` (no `--skip-specs`), commit everything except `HANDOFF.md`, then commit `HANDOFF.md` only; do not push.

# Tasks: Measure performance and enforce a byte budget

## 1. BFS — Baseline and impact
- [x] 1.1 Read `AGENTS.md`, `.ai-rules/`, `HANDOFF.md`, `docs/gap-closure.md` (package 2), the six capability specs and the delivery evidence; confirm no active change.
- [x] 1.2 Record the baseline build: `index-DO5RYnOJ.js` 307,491 bytes, `index-B-c8V-0n.css` 27,698 bytes, `index.html` 664 bytes, with their SHA-256 values.
- [x] 1.3 Map the surface: the six client routes, `scripts/`, `package.json`, `verify-local.sh`, `AGENTS.md`, `README.md` and `docs/evidence/`.
- [x] 1.4 Decide the owning spec: `ui-foundation` holds client-asset policy, so the performance record and the byte ceiling are recorded there and `inventory-import` is untouched.

## 2. DFS — Measured performance
- [x] 2.1 Build production assets, start `target/debug/rigcraft` on a scratch data directory, seed the shipped sample snapshot and the synthetic catalog dataset through the documented API.
- [x] 2.2 Run Lighthouse 13.6.0 performance-only for `/`, `/inventory`, `/catalog`, `/builds/check`, `/recommend` and `/diagnose`, mobile and desktop; commit every JSON report under `docs/evidence/perf-lighthouse/`.
- [x] 2.3 Read score, FCP, LCP, TBT, CLS, speed index and transferred bytes from the reports; record the asset hashes and the exact command.
- [x] 2.4 Audit every non-perfect finding; fix only an unambiguous, safe one; record the rest with the measured number and the reason.

## 3. DFS — Byte budget
- [x] 3.1 Add `scripts/check-bundle-size.mjs` with explicit per-type ceilings, a named failure carrying file, size, ceiling and delta, and a hard failure when the build is absent or empty.
- [x] 3.2 Add `scripts/check-bundle-size.test.mjs` with `node:test`: a `fs.mkdtemp` within-budget fixture passes and an over-budget fixture fails; no repository write.
- [x] 3.3 Wire the check into `npm run build` and `npm run check`; reorder `./verify-local.sh` so `npm run build` precedes `npm run check`.

## 4. BFS — Docs and evidence
- [x] 4.1 Write `docs/evidence/performance-evidence.md`: the full route/device table, the commands, the report paths, the budget ceilings, the findings and the lab-vs-field statement.
- [x] 4.2 Update `AGENTS.md`, `README.md` and the docs that state what `npm run check` covers.

## 5. Verification and close-out
- [x] 5.1 `cargo fmt --all -- --check`; `cargo clippy --all-targets --all-features -- -D warnings`; `cargo test`; `npm run check`; `npm test`; `npm run build`; `npm run test:e2e`; `./verify-local.sh`.
- [x] 5.2 `node scripts/check-openspec-change-names.mjs`; `openspec validate performance-evidence --strict`; `openspec validate --all --strict`.
- [x] 5.3 Run the Forge full Gate and record its aggregate verdict and labelled revision.
- [x] 5.4 Archive with `openspec archive performance-evidence --yes` (no `--skip-specs`), commit everything except `HANDOFF.md`, then commit `HANDOFF.md` only; do not push.

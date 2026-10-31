# Tasks: Correct audited correctness and contract defects

## 1. BFS — Baseline and impact coverage
- [x] 1.1 Read `AGENTS.md`, `.ai-rules/`, `HANDOFF.md`, `ROADMAP.md`, `requirement.md` and the five contracts; confirm `openspec list` shows no active change and set `current_spec`.
- [x] 1.2 Re-confirm each defect's mechanism in the code and against a fresh instance after the fix; map callers, persistence, routes, fixtures, tests and browser states.
- [x] 1.3 Assess shared infrastructure by name (Forge Rust libraries, pinned tokens) and record that no new dependency or sibling source is needed.

## 2. DFS — Requirement-by-requirement implementation
- [x] 2.1 Checked totals: make `Snapshot::totals_for` return a checked result; reject commit and correction overflow with 422 `price_overflow`; add tests.
- [x] 2.2 Diagnosis unverified state: add the `unverified` action/group, summary counts, group blurb alignment and tests.
- [x] 2.3 Markdown export escaping: escape pipe/backslash, neutralise HTML in table cells, add the injection fixture and column-count test.
- [x] 2.4 SPA fallback: 404 unknown static/metadata paths, keep extensionless client routes, add route tests.
- [x] 2.5 Error envelope: `ApiJson` and multipart rejection mapping with `invalid_json`/`invalid_body`, tests across POST endpoints.
- [x] 2.6 `/diagnose` skeleton: stable 1–5 numbering with a real review empty state and component tests.
- [x] 2.7 Currency: normalise case-insensitively, require three ASCII letters, lock with API tests, update all five docs.
- [x] 2.8 Timestamps: one ISO 8601 UTC helper with `<time datetime>` and locale-invariance tests; document `RIGCRAFT_ADDR` in README.

## 3. BFS — Cross-surface regression and completeness
- [x] 3.1 Recheck callers, stored data, error codes, privacy, stale state, responsive layout and accessibility; update contract docs and README claims in the same change.
- [x] 3.2 Update `AGENTS.md` Project facts only if a command or path changed; re-check the tree.

## 4. Verification and evidence
- [x] 4.1 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test`, `npm run check`, `npm test`, `npm run build`, `./verify-local.sh`.
- [x] 4.2 Run `node scripts/check-openspec-change-names.mjs` and `openspec validate --all --strict`; validate this change strictly.
- [x] 4.3 Capture after-fix browser evidence for #2, #6, #8 and the #4 static 404s (the before state is quoted from the reproduced defect, not fabricated); write `docs/evidence/correctness-audit-browser.md`; re-run Lighthouse for the changed pages.
- [x] 4.4 Run the Forge full Gate and record its aggregate verdict.
- [x] 4.5 Archive without `--skip-specs`; commit implementation (everything except `HANDOFF.md`), then commit `HANDOFF.md` only; do not push.

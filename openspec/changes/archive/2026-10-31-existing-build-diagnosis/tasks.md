# Tasks: Diagnose an existing or proposed PC build

## 1. BFS — Baseline and impact coverage
- [x] 1.1 Map P1 upgrade and list diagnosis to actors, route, contracts, persistence, errors, fixtures and browser states; reconcile proposal/design/spec. Mapped in `openspec/changes/existing-build-diagnosis/specs/build-diagnosis/spec.md` and `docs/build-diagnosis.md`: `/diagnose`, `POST /api/diagnoses/parse`, `POST /api/diagnoses`, `GET /api/diagnoses/{id}`, `diagnoses` persistence, `part_invalid`/`duplicate_part`/`category_multiple`/`currency_mismatch`/`sku_not_found`/`build_empty`/`build_too_large`/`line_limit_exceeded`/`text_too_large` errors, four synthetic fixtures, and the empty/review/ambiguity/diagnosing/results/plan/error states.
- [x] 1.2 Inspect Forge shared artifacts and pinned token receipt; confirm local ownership and the implementation-handoff gate. `.project.json` records the pinned `tokens.css` SHA-256 and no sibling PC-domain package exists; the proposal's reconnaissance table keeps the domain local. The page reuses the vendored token stylesheet and the shared component styles.

## 2. DFS — Requirement-by-requirement implementation
- [x] 2.1 Implement domain types, validation, persistence and contract for build-diagnosis; add failure fixtures. `src/diagnosis/{mod,matching,advice}.rs`, SQLite schema `4` with `diagnoses`, and `tests/diagnosis_api.rs` plus `tests/fixtures/diagnosis/` cover parsing limits, currency, duplicate rows, honest matching, advice groups, and the three plan-validation outcomes.
- [x] 2.2 Implement /diagnose with all stated states, keyboard/focus, mobile layout and accessible feedback. `apps/web/src/components/{DiagnosePage,DiagnosisReportPanel}.tsx` with `apps/web/src/diagnosis.ts`; the Vitest suite covers the ambiguity resolution, blocked review, manual entry, failure-retains-work and empty states.
- [x] 2.3 Connect the UI to the real API and verify each spec scenario end to end. The browser journeys in `docs/evidence/diagnosis-browser.md` prove each scenario against the running service; `tests/diagnosis_api.rs` asserts the same contract at the HTTP boundary.

## 3. BFS — Cross-surface regression and completeness
- [x] 3.1 Recheck callers, snapshots, error codes, privacy, stale data, responsive and accessibility behavior; remove placeholders. Staleness follows the snapshot correction chain; the described build is a synthetic snapshot so no real snapshot or check row is written; no catalog, AI or network call is added; 390 px overflow and Lighthouse accessibility were re-checked.
- [x] 3.2 Review adjacent package contracts without implementing their scope. The diagnosis reuses `POST /api/builds/check`'s engine and `compat-rules-v1` report shape, the recommendation money/stock conventions, and the AI layer's no-persistence boundary; none of their archived specs changed.

## 4. Verification
- [x] 4.1 Run formatter, native build/tests, Web checks, browser flows, accessibility checks, change-name check, strict OpenSpec validation and Forge full Gate. Recorded in `HANDOFF.md` with verdicts and the commit hash.
- [x] 4.2 Record revision-bound evidence and capture; archive without skip-specs only after implementation is complete. Archived to `openspec/changes/archive/2026-10-31-existing-build-diagnosis/` after strict validation and the full Gate; captures and the browser/accessibility evidence are in `docs/`.

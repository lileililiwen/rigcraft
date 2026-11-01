# Tasks: Name the forms, the main regions and the result heading

## 1. BFS — Baseline and impact
- [x] 1.1 Read `AGENTS.md`, `.ai-rules/`, `HANDOFF.md`, `docs/gap-closure.md`, `docs/evidence/assistive-technology-review.md`, `openspec/specs/ui-foundation/spec.md`, `playwright.config.ts`, `e2e/fixtures.ts` and `e2e/smoke.spec.ts`; confirm `openspec list` reports no active change and set `current_spec`.
- [x] 1.2 Enumerate every `<form>` in `apps/web/src` with the heading that introduces it, and the routes whose `main` has no name.
- [x] 1.3 Decide the owner: `ui-foundation` for all six requirements; no existing requirement modified, and the collapsed-assistant transport-form requirement left intact.
- [x] 1.4 Record the pixel constraint: twelve Chrome baselines exist and none may be regenerated, so every edit must be attribute-only or an element swap with identical box metrics.

## 2. DFS — Form names
- [x] 2.1 Make the inventory import panel a `<form className="panel" aria-labelledby="import-title">` with its existing class, children and handlers.
- [x] 2.2 Name the catalogue upload and search forms from `#upload-title` and `#search-title`.
- [x] 2.3 Name the budget form from `#request-form-title` and give the assistant transport form the aria-label `Assistant provider settings`.
- [x] 2.4 Name the compatibility-check parts form from `#parts-title`.
- [x] 2.5 Give the restore, restore-confirmation, row-correction and record-correction forms their headings' `id`s (or an aria-label where no heading names them) and point `aria-labelledby` at them.

## 3. DFS — Roles and names
- [x] 3.1 Give the `/inventory` export/backup line `role="note"` and add the visually hidden `role="status"` region that carries each real action result.
- [x] 3.2 Give the "Catalog accepted" heading `id="catalog-result-title"` and keep its focus move.
- [x] 3.3 Give every page `h1` a stable `id` and every route's single `main` an `aria-labelledby` to it.
- [x] 3.4 Leave the `/builds/check` `aria-live="off"` selection counter byte-identical.

## 4. DFS — Tests
- [x] 4.1 Vitest: every form's accessible name, the static hint as `note` and not `status`, the catalogue result heading's `id`, and each page `main`'s name.
- [x] 4.2 Playwright `e2e/landmarks.spec.ts`: `getByRole("main", { name })` on all six routes, `getByRole("form", { name })` after the interactions that render each form, no unnamed form, and the skip link still focusing `main`.
- [x] 4.3 Run the full browser suite with the committed baselines unregenerated and record the pass counts per project.

## 5. BFS — Evidence and wiring
- [x] 5.1 Capture `ariaSnapshot()` of `/inventory` and `/recommend` from the production build and write `docs/evidence/assistive-tech-gaps.md`.
- [x] 5.2 Update `docs/evidence/assistive-technology-review.md`: mark items 1, 2, 3 and 5 fixed with the new names, keep item 4 open as a recorded human decision, and keep the "not a screen-reader pass" statement and the human script.
- [x] 5.3 Re-check `AGENTS.md` Project facts and the README claims against the tree; record the new spec file in the browser-suite evidence.

## 6. Verification and close-out
- [x] 6.1 `cargo fmt --all -- --check`; `cargo clippy --all-targets --all-features -- -D warnings`; `cargo test`; `npm run build`; `npm run check`; `npm test`; `npm run test:e2e`; `./verify-local.sh`.
- [x] 6.2 `node scripts/check-openspec-change-names.mjs`; `openspec validate assistive-tech-gaps --strict`; `openspec validate --all --strict`.
- [x] 6.3 Run the Forge full Gate and record its aggregate verdict and labelled revision.
- [x] 6.4 Archive with `openspec archive assistive-tech-gaps --yes` (no `--skip-specs`), commit everything except `HANDOFF.md`, then commit `HANDOFF.md` only; do not push.

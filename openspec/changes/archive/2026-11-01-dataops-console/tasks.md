# Tasks: Present inventory and catalog as an operator console

## 1. BFS — Baseline, impact and reproduction
- [x] 1.1 Read `AGENTS.md`, `.ai-rules/`, `HANDOFF.md`, `docs/design-program.md` (package 4), `docs/branding.md`, `docs/inventory-import.md`, `docs/catalog-ingestion.md`, `docs/data-portability.md`, `InventoryPage.tsx`, `SnapshotPanel.tsx`, `PreviewReview.tsx`, `CatalogPage.tsx`, `DataPortability.tsx`, `DiagnosePage.tsx`, `inventory.ts`, `api.ts`, `app.css`, the Vitest page tests and the Playwright inventory/catalog/portability journeys; confirm no other active change and set `current_spec`.
- [x] 1.2 Reproduce against a committed snapshot: `/inventory` renders both export buttons disabled with only `Open or commit a snapshot first.`; the snapshot is reachable only through a `View` button in a prose list; `/catalog` buries licence, version and counts inside a name cell; the four actions error after the click; `/diagnose` shows `Fixture AM4 CPU 5600` and `Fixture RTX 4070`.
- [x] 1.3 Map impact: the positional `dl.summary-grid dd` assertions, `dl.record-meta`, `page.locator("table.responsive-table")` on `/inventory`, the `View {shortId}` button names, the catalog registry/search captions, the `Catalog accepted` heading and refusal alert, the `Review import` flow, the `/diagnose` 1–5 skeleton and every live region and focus target.
- [x] 1.4 Assess shared infrastructure by name (pinned token kit `7ca6b795…`, brand layer, `plural.ts`, the `.responsive-table`/`.badge` shell, the Playwright suite, the inventory/catalog/backup APIs) and record that all are consumed unchanged.

## 2. DFS — Inventory console
- [x] 2.1 Add the console summary bar for the open snapshot: revision, records, units in stock, stock value, latest/earlier chip, source file and commit time, without reusing `dl.summary-grid`.
- [x] 2.2 Turn snapshot history into a card grid with revision, timestamp, record count, stock value, note, `Viewing now` marker and the explicit `View` action.
- [x] 2.3 Auto-open the newest revision on first load through the existing `viewSnapshot` path, announce it politely, never move focus on load, and let `Start a new import` close it for good.
- [x] 2.4 Keep the import panel rendered when a snapshot is open, clear the committed file from it, and give the import, history and export panels the console header treatment.
- [x] 2.5 Replace the leftover inline style in `SnapshotPanel` with a class and keep every record field, provenance chip, correction and announcement.

## 3. DFS — Catalog registry console
- [x] 3.1 Present the registry as a dense table: dataset name with a catalog provenance chip, licence, version, parts, categories, source URL, active chip and actions, plus a dataset/parts/active count summary.
- [x] 3.2 Add rank and dataset version to the search results while keeping rank order, dataset attribution, licence, caption and the part detail view.
- [x] 3.3 Keep the accepted report, the refusal alert with its code, row and field wording, the empty-registry state and the empty-search state reachable and announced.

## 4. DFS — Guarded actions, examples and plurals
- [x] 4.1 Disable `Preview CSV` without a file or a three-letter currency, `Search catalog` with an empty query, `Match this list` with an empty list and `Diagnose this build` with an empty build, each with an `aria-describedby` reason; keep every handler and server check.
- [x] 4.2 Add `isValidCurrencyCode` to `inventory.ts` and use it in the form and the preview handler.
- [x] 4.3 Replace the diagnosis placeholders, the parts-list help text and the manual-entry specification example with the shipped sample's real parts and specifications.
- [x] 4.4 Route every count these pages compose through `plural`, including the catalog hit count and the portability record and snapshot counts, and fix `1 records`.

## 5. BFS — Tests and evidence
- [x] 5.1 Extend Vitest: inventory summary and auto-opened snapshot with enabled export, history cards, guarded buttons with their reasons, catalog registry counts/columns and guarded search, the new placeholder copy and plural forms.
- [x] 5.2 Extend Playwright: export reachable after a commit, the catalog upload/refusal/search journey unchanged and green, the disabled actions at 390 px, and no horizontal overflow.
- [x] 5.3 Run the app on a free port with a scratch data directory, import the shipped sample and upload the synthetic catalog fixture used by `e2e/catalog.spec.ts`, then capture 1440×900 and 390×844 into `docs/assets/dataops-*.png`.
- [x] 5.4 Run Chrome DevTools MCP `lighthouse_audit` desktop and mobile on `/inventory` and `/catalog`, measure focus rings, caption widths and `scrollWidth === clientWidth`, and write `docs/evidence/dataops-console-browser.md` with the real numbers.
- [x] 5.5 Update `docs/design-program.md`, `docs/inventory-import.md`, `docs/catalog-ingestion.md`, `docs/data-portability.md`, `docs/assets/capture-plan.md` and the README claims touched by the new presentation.

## 6. Verification and close-out
- [x] 6.1 `cargo fmt --all -- --check`; `cargo clippy --all-targets --all-features -- -D warnings`; `cargo test`; `npm run check`; `npm test`; `npm run build`; `npm run test:e2e`; `./verify-local.sh`.
- [x] 6.2 `node scripts/check-openspec-change-names.mjs`; `openspec validate dataops-console --strict`; `openspec validate --all --strict`.
- [x] 6.3 Run the Forge full Gate and record its aggregate verdict and labelled revision.
- [x] 6.4 Archive with `openspec archive dataops-console --yes` (no `--skip-specs`), commit everything except `HANDOFF.md`, then commit `HANDOFF.md` only; do not push.

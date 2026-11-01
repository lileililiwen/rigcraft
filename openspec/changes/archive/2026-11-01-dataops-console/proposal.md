# Proposal: Present inventory and catalog as an operator console

## Why

`/inventory` and `/catalog` are the two pages an operator lives in, and both open
with the things an operator does not need first. `/inventory` shows an empty file
picker above everything, keeps the snapshot it already has behind a `View` button
in a one-line list, and the export panel below it is disabled until that button is
pressed. A shop that has committed a snapshot, reloads the page and wants its CSV
sees two greyed-out download buttons and no explanation of the step that would
light them up — the export is reachable, but only through a control the copy never
names. `/catalog` lists a registry and a result set as loose prose rows, so the
licence, version and source URL that decide whether a dataset may be used are the
hardest things on the page to compare.

Four actions also raise an error after the click instead of preventing it. `Preview
CSV` with no file, `Search catalog` with an empty query, `Match this list` with an
empty list and `Diagnose this build` with an empty build each set an alert that the
button's own state could have told the operator in advance. The diagnosis page
still offers `Fixture AM4 CPU 5600` and `Fixture RTX 4070` as examples, though the
shipped sample is an AM5 machine, and its manual-entry example names an AM4 socket
the sample does not contain.

This change presents both pages as an operator console — state first, provenance in
every row, actions guarded — and fixes those copy defects. It changes no API shape,
no route, no schema, no rule, no engine value and no data source. The catalog stays
an operator upload: no crawler, fetcher, sync, scheduler or browser schema editor
is added or implied.

## What Changes

- **Inventory summary bar.** The open snapshot opens with one bar carrying its
  revision, record count, units in stock, stock value, whether it is the latest
  revision, and its source file and commit time. `SnapshotPanel` keeps every
  record field, provenance chip, correction entry point and announcement.
- **Snapshot history as cards.** Every revision is a card with its revision,
  timestamp, record count, stock value, note and an explicit `View` action, with
  the currently open revision marked. No revision, count or announcement is
  dropped.
- **Export is usable on arrival.** The newest revision opens automatically on
  first load, so the CSV and JSON buttons are enabled without an unexplained step,
  and `Start a new import` still closes the view. The disabled copy now names the
  control that opens a snapshot instead of leaving the buttons dead.
- **Catalog registry console.** The upload panel, the dataset registry and the
  search results become dense console tables: dataset name with a catalog
  provenance chip, licence, version, part and per-category counts, source URL,
  active state and actions, plus a registry count summary. Refusals keep their
  code, row and field wording, and search keeps rank order, dataset attribution,
  licence, version and the part detail view.
- **Guarded actions.** `Preview CSV` (no file, empty or malformed currency),
  `Search catalog` (empty query), `Match this list` (empty list) and `Diagnose this
  build` (empty build) are disabled with an accessible reason next to the control.
  Nothing that currently succeeds is disabled, and server validation stays the
  source of truth.
- **Real examples and plurals.** The diagnosis placeholders and the parts-list
  help text use the shipped sample's real parts (`AMD Ryzen 7 7800X3D`,
  `NVIDIA GeForce RTX 4070 Founders Edition`, `Corsair RM750e`), and page-composed
  counts on both pages use the shared plural helper instead of hand-rolled
  singular/plural concatenation.
- **Tests and evidence.** Vitest covers the inventory summary, the auto-opened
  snapshot and enabled export, the guarded buttons, the catalog registry and
  search density and the new placeholder copy; Playwright keeps every inventory,
  catalog and portability assertion and adds an export-after-commit journey, the
  catalogue upload/refusal/search journey and the disabled 390 px controls.
  Lighthouse, captures and a browser evidence document are added.

## Impact

- Affected specs: `ui-foundation` (four presentation requirements added),
  `inventory-import` (Web-flow requirement modified: auto-open and reset),
  `catalog-ingestion` (catalog-page requirement modified: dense registry, search
  and guarded query), `data-portability` (inventory-page requirement modified:
  export availability and the copy that names the control).
- Affected code: `apps/web/src/components/InventoryPage.tsx`, `ImportForm.tsx`,
  `DataPortability.tsx`, `CatalogPage.tsx`, `DiagnosePage.tsx`,
  `apps/web/src/inventory.ts` (one currency predicate), `apps/web/src/styles/app.css`,
  the Vitest page tests, `e2e/inventory.spec.ts`, `e2e/catalog.spec.ts`,
  `e2e/portability.spec.ts`, `docs/`.
- Unchanged: every API shape, route, schema version, validation code, rule,
  engine value and data source; the CSV preview/correction/commit flow, duplicate
  decisions, catalog opt-in, backup inspect/restore confirmation, focus moves,
  live regions, keyboard order, the vendored and brand token files, and every
  existing e2e assertion.
- Non-goals: no crawler, fetcher, sync, scheduler, background job, third-party
  catalog, browser schema editor or cross-instance sharing.

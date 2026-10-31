# Catalog ingestion browser and accessibility evidence

Captured: 2026-11-01, against the working tree based on revision `2ef6f86`
(`Record data portability evidence`). The implementation commit is recorded in
`HANDOFF.md`.

- Build under test: `npm run build` → `apps/web/dist`, served by `target/debug/rigcraft`.
- Service: `RIGCRAFT_ADDR=127.0.0.1:8799 RIGCRAFT_DATA_DIR=/tmp/opencode/rigcraft-catalog-evidence ./target/debug/rigcraft`
  (port 8080 is occupied on this host and was not used).
- Browser: Google Chrome through Chrome DevTools MCP, not the Vite dev server.
- Data: `tests/fixtures/catalog/synthetic.json`, a small synthetic catalog with three
  parts (two CPUs and one GPU), dataset `synthetic-fixture`, commit `fixture-1`,
  licence `CC0-1.0`. No third-party dataset was read, uploaded or committed.

## Captures

| File | Viewport | State proved |
| --- | --- | --- |
| `docs/assets/catalog-ingestion-upload-desktop.png` | 1440×900 | Accepted report: `synthetic-fixture` added, 3 parts, `2 cpu · 1 gpu`, licence `CC0-1.0`, version `fixture-1`, provenance form `catalog:synthetic-fixture@fixture-1`; registry row with the same licence, 3 parts and `Active`. |
| `docs/assets/catalog-ingestion-search-desktop.png` | 1440×900 | Ranked search: 2 hits for `SYN-CPU-1` in `cpu`, the exact SKU first, each naming the dataset and `CC0-1.0`; the opened part shows three `Catalog` specification badges and `specs: Catalog · synthetic-fixture@fixture-1 · https://example.invalid/cpu-one`. |
| `docs/assets/catalog-ingestion-refused-desktop.png` | 1440×900 | Refused document: alert focused, `catalog_invalid`, `Row 1 · model — model is required.`, the accepted specification keys listed, and the existing registry unchanged (no `refused-fixture`). |
| `docs/assets/catalog-ingestion-enriched-desktop.png` | 1440×900 | Import review: `Matched by sku to synthetic-fixture (v fixture-1)`, `Will add from the catalog: specs.igpu, specs.socket, specs.tdp`, the per-row `Apply catalog proposals` checkbox checked, and the operator's values still shown. |
| `docs/assets/catalog-ingestion-provenance-desktop.png` | 1440×900 | Committed revision 1: three specifications with the `Catalog` badge, `match catalog-declared`, `specs: Catalog · synthetic-fixture@fixture-1 · https://example.invalid/cpu-one`, while `model`, `sku`, `price` and `quantity` remain `User-provided · csv:enrich-row.csv`. |
| `docs/assets/catalog-ingestion-mobile.png` | 390×844 | The upload, registry and search stacked at phone width. |

## Journey exercised

1. Opened `/catalog`. Chose `synthetic.json`, checked the acknowledgement, and clicked
   **Import catalog**. The status announced `Catalog accepted: 3 parts from
   synthetic-fixture.`, the `Catalog accepted` heading took focus, and the report
   showed `3 parts · 2 cpu · 1 gpu`, licence `CC0-1.0`, version `fixture-1`, source URL
   and the future provenance form.
2. Searched `SYN-CPU-1` in category `cpu`. The status announced `2 catalog matches;
   showing 2.` The exact-SKU part came first, followed by the token-overlap part, and
   both rows named the dataset and licence. **View part** opened `Synthetic CPU One`
   with its manufacturer, three specifications and per-field provenance.
3. Uploaded `refused-catalog.json`, whose second part has an empty model. The
   `role="alert"` took focus with `Catalog refused`, code `catalog_invalid`,
   `Row 1 · model — model is required.` and the accepted specification keys. The
   registry still showed only `synthetic-fixture`; `refused-fixture` was absent, so
   nothing was stored.
4. Opened `/inventory` and previewed a one-row CSV that supplies a model and SKU but
   no specifications. The review reported `Matched by sku to synthetic-fixture (v
   fixture-1)` and `Will add from the catalog: specs.igpu, specs.socket, specs.tdp`,
   with the operator's model marked `Operator-entered facts` and `match unverified`
   until the opt-in is applied.
5. Checked **Apply catalog proposals** and committed. The committed snapshot showed
   three specifications with the `Catalog` badge, `match catalog-declared`, and
   `specs: Catalog · synthetic-fixture@fixture-1 · https://example.invalid/cpu-one`.
   `model`, `sku`, `quantity` and `price` stayed `User-provided · csv:enrich-row.csv`;
   no operator value was replaced.
6. Checked the 390×844 viewport:
   `document.documentElement.scrollWidth === clientWidth === 390`.

## Keyboard and focus

- The skip link focused first and had a computed outline of
  `rgb(29, 78, 216) solid 3px`.
- After an accepted upload, focus moved to `Catalog accepted`; after a refusal, focus
  moved to the `role="alert"` region; after **View part**, focus moved to the stored
  part's heading; after a commit, the existing snapshot heading kept focus.
- The file input, acknowledgement checkbox, import button, registry actions, search
  fields and part button are native controls in document order; the import button is
  disabled until both a file and the acknowledgement are present.
- Removal is two-step: **Remove** opens an inline confirmation naming the dataset and
  part count and stating that inventory, snapshots, checks, recommendations and
  diagnoses are not affected; only **Confirm removal** calls
  `DELETE …?confirm=remove`.
- No state relies on colour alone: statuses carry a symbol and words, conflicts name
  both values, and the refusal names the row and field.

## Measured contrast

`npm run check` re-resolved the vendored tokens; `docs/evidence/token-contrast.json`
keeps its 16 measured pairs and its lowest ratio of 5.46:1. The new page reuses
existing tokens only: body and secondary text on surface, primary and secondary
buttons, the danger button label on surface, the focus ring (7.29:1 on surface),
error text and the catalog badge pair (5.74:1). No new colour token was added.

## Lighthouse

Chrome DevTools Lighthouse, navigation mode, against the production build on
`http://127.0.0.1:8799/catalog`:

| Device | Accessibility | Best practices | SEO | Agentic browsing | Audits | Report |
| --- | --- | --- | --- | --- | --- | --- |
| Desktop | 100 | 100 | 100 | 100 | 52 passed, 0 failed | `docs/evidence/catalog-ingestion-lighthouse/report.json` |
| Mobile | 100 | 100 | 100 | 100 | 52 passed, 0 failed | `docs/evidence/catalog-ingestion-lighthouse-mobile/report.json` |

## Privacy review

- The service log contained the listen line only. No catalog document, part, row,
  licence acknowledgement or CSV content was logged.
- The uploaded catalog stayed in
  `/tmp/opencode/rigcraft-catalog-evidence/inventory.sqlite3` for the duration of the
  evidence run. It was not written to the repository, uploaded elsewhere or copied into
  a fixture; the committed fixture is the small synthetic document above.
- No request left the machine; the part URLs under `example.invalid` were never
  fetched.

## Limits

- Lighthouse performance was not measured; this change adds one page and one table
  column, not new rendering work.
- Only Chrome was used. There are no visual-diff baselines.
- The checked-in Playwright suite covers the same upload, refusal, search and enriched
  import journeys on every `npm run test:e2e` run (`e2e/catalog.spec.ts`); this document
  records the manual Chrome DevTools pass and its captures.

# DataOps console browser evidence (2026-11-01)

Source: the production Web build (`npm run build`, `index-B-c8V-0n.css` and
`index-DO5RYnOJ.js`) served by `target/debug/rigcraft` on
`127.0.0.1:46901` with the scratch data directory
`/tmp/opencode/dataops-evidence`. The shipped sample was imported through the
documented API: 10 valid rows, 0 errors, 0 duplicates, snapshot
`3deeda2f-d660-42a0-8da0-496407f1eea8` revision 1 (10 records, 10 units, value
212090 minor units, created `2026-11-01T04:56:15.227Z`). The synthetic catalog
fixture used by `e2e/catalog.spec.ts` was uploaded through the documented API:
dataset `synthetic-fixture` version `fixture-1`, licence `CC0-1.0`, 3 parts
(2 `cpu`, 1 `gpu`), source URL `https://example.invalid/catalog`, imported
`2026-11-01T04:56:15.233Z`. Tool: Chrome DevTools MCP against real Chrome;
viewports 1440×900 and 390×844.

## `/inventory` — the open snapshot comes first

With a committed snapshot and a fresh page load, the page opened revision 1 by
itself. The console bar reported `Open snapshot`, `✓ Latest revision`,
`Snapshot state`, `Revision 1`, `Records 10`, `Units in stock 10`, `Stock value
US$2,120.90`, `Source rigcraft-inventory-sample.csv · committed
2026-11-01T04:56:15.227Z`. The history card for the same revision showed
`Revision 1`, `✓ Viewing now`, the committed time, 10 records,
`US$2,120.90` and `View 3deeda2f…eea8`.

`Download CSV` and `Download JSON` were enabled on arrival and export needed no
extra click; `e2e/inventory.spec.ts` downloads the CSV after a fresh commit and
asserts the `rigcraft-inventory-snapshot-r1-*.csv` filename. `Start a new import`
still closes the view and is exercised by `e2e/portability.spec.ts`, which then
re-imports the exported CSV.

The automatic open did not take focus: `document.activeElement` was `BODY` after
load. The first Tab focused `Skip to inventory` with a `3px solid
rgb(34, 211, 238)` outline at a 2 px offset, `:focus-visible` true, top 8 and
bottom 52 px.

Without a file, `Preview CSV` was disabled, pointed at `#preview-hint` through
`aria-describedby`, and the hint read `Choose a CSV file to enable Preview CSV.`
`Download CSV` stayed enabled because a snapshot was open.

Captures: `docs/assets/dataops-inventory-desktop.png`,
`docs/assets/dataops-inventory-desktop-full.png`,
`docs/assets/dataops-inventory-mobile.png`.

## `/catalog` — dense registry and ranked search

The registry count strip read `Datasets 1`, `Parts 3`, `Active 1`. The row
carried `synthetic-fixture` with a `▣ Catalog` provenance chip, the source URL,
the import time, `CC0-1.0`, version `fixture-1`, 3 parts, `cpu 2, gpu 1`,
`✓ Active`, `Deactivate` and `Remove`.

With an empty query, `Search catalog` was disabled, pointed at
`#catalog-search-hint`, and the hint read `Enter a SKU, MPN, model or word to
enable search.` Searching `Synthetic` returned `3 catalog matches; showing 3
hits.` in rank order with the rank column, SKU, MPN, category, model, dataset
name with its chip, licence, version and `View part`.

Captures: `docs/assets/dataops-catalog-desktop.png`,
`docs/assets/dataops-catalog-registry-desktop.png`,
`docs/assets/dataops-catalog-search-desktop.png`,
`docs/assets/dataops-catalog-mobile.png`,
`docs/assets/dataops-catalog-search-mobile.png`.

## `/diagnose` — real examples and guarded actions

The parts-list placeholder read `cpu: AMD Ryzen 7 7800X3D`, `gpu: NVIDIA GeForce
RTX 4070 Founders Edition`, `psu: Corsair RM750e`; the manual-entry specification
example read `socket=AM5;tdp=120W;igpu=yes`. These are the shipped sample's own
parts and specifications, not fixture hardware.

With no pasted text, `Match this list` was disabled with `Paste at least one part
above to enable matching.`; with no reviewed or hand-added part, `Diagnose this
build` was disabled with `Add a matched line or a part by hand to enable the
diagnosis.` Each control points at its reason through `aria-describedby`. The
handlers and the service checks are unchanged; `e2e/diagnosis.spec.ts` still
presses `Diagnose this build` before the review is resolved and reads the real
`review_incomplete` refusal.

## Measurements

| Measurement | 1440×900 | 390×844 |
| --- | --- | --- |
| `/inventory` `scrollWidth === clientWidth` | 1440 = 1440 | 390 = 390 |
| `/catalog` `scrollWidth === clientWidth` | 1440 = 1440 | 390 = 390 |
| `/inventory` console bar width | 1216 px | 358 px |
| `/inventory` record caption | 1182 × 32 px | 332 × 56 px |
| `/catalog` registry caption | 1182 × 32 px | 332 × 56 px |
| `/catalog` search caption | 1182 × 32 px | card layout, no overflow |
| History card width | — | 332 px |
| Skip-link focus ring | `3px solid rgb(34, 211, 238)`, offset 2 px | same on `/catalog` |

The record caption spans the card at 390 px instead of collapsing, and long
provenance URLs wrap inside their cell.

## Lighthouse

Chrome DevTools MCP `lighthouse_audit`, navigation mode, production build:

| Route | Device | Accessibility | Best practices | SEO | Agentic browsing | Passed / failed |
| --- | --- | --- | --- | --- | --- | --- |
| `/inventory` | desktop | 100 | 100 | 100 | 100 | 58 / 0 |
| `/inventory` | mobile | 100 | 100 | 100 | 100 | 58 / 0 |
| `/catalog` | desktop | 100 | 100 | 100 | 100 | 55 / 0 |
| `/catalog` | mobile | 100 | 100 | 100 | 100 | 55 / 0 |

Reports: `docs/evidence/dataops-lighthouse/inventory-desktop/`,
`inventory-mobile/`, `catalog-desktop/` and `catalog-mobile/`.

The first inventory run before the initial render was gated measured cumulative
layout shift **0.593** (desktop) and **0.451** (mobile): the automatic open added
the summary bar and records above the import panel after paint, which scored
agentic browsing 56 and 60. The page now renders nothing below the status line
until the first snapshot state has resolved, so the automatic open appends its
bar and records instead of pushing existing content down. The re-run reported
100 on every category and 0 failed audits above.

## Conformance

- `ui_conformance.py` reports `PASS: rigcraft ui_files=72 errors=0
  review_required=0 baselined=0`; the 28 opened content files include the new
  console classes.
- `apps/web/src/styles/vendor/tokens.css` still hashes
  `7ca6b7956b3f6b475254b4a5d0cf599724e4e1af5f0f88aba4fbccb9f38fc19b`.
- No colour literal outside `brand.css`, no new token, no inline style, no
  `!important` outside the reduced-motion block, no new dependency and no
  webfont. The console classes consume existing semantic aliases only.
- No API shape, route, schema, engine value, validation code or data source
  changed. The catalog remains an operator upload; no crawler, fetcher, sync,
  scheduler, background job or browser schema editor exists or is implied.

## Not done

- Lighthouse performance was not measured; the categories above exclude it.
- No visual-diff baseline, no non-Chrome browser and no manual screen-reader
  pass; the accessibility tree, Lighthouse, focus measurements and the keyboard
  suite are the record.
- The sample and catalog were seeded through the documented API for this visual
  pass; the upload, refusal and search journeys are driven through the UI by
  `e2e/catalog.spec.ts`.

# Catalog seed and provenance browser evidence

Status: captured 2026-10-31 against the production build served by the Rust process,
working tree based on `main` at `bfab08a`. Tool: Chrome DevTools MCP. Viewports:
1440×900 desktop and 390×844 mobile. Implementation commit is recorded in `HANDOFF.md`
because this document ships in that commit.

## Data

- Shipped sample: `GET /api/imports/sample`, 2704 bytes, SHA-256
  `2486517c5b490a02bab101a00eb7333e09da77642f66553a1d9cb91b4338aa6e`, 10 rows, one
  AM5 build, `source=catalog:manufacturer@2026-10-31` and a manufacturer page per row.
- Real catalog import: a sparse clone of `buildcores/buildcores-open-db` at commit
  `d8d0149dbb32f75e9408d0476b231984c1fc2619`, converted with
  `node scripts/opendb-to-csv.mjs … --price 1.00 --quantity 1 --only CPU --only PCCase --only CPUCooler --only Storage`.
  The converter emitted 10,527 rows with 607 warnings (shared MPNs, over-length MPNs
  replaced by their `opendb_id`, two unusable manufacturer URLs, two over-length model
  names). `POST /api/imports` accepted the 2,349,400-byte file: 9,619 valid, 2 `model
  too_long`, 906 duplicate rows.
- No personal or shop inventory was used. Prices in both files are operator-entered
  sample values, not market data.

## Captures

| Capture | Route | Viewport | Condition |
| --- | --- | --- | --- |
| `docs/assets/catalog-import-empty-desktop.png` | `/inventory` | 1440×900 | Empty state; download area states CC BY 4.0, the 2026-10-31 check date and operator-entered prices |
| `docs/assets/catalog-import-review-desktop.png` | `/inventory` | 1440×900 | Sample preview: `10 rows · 0 errors · 0 duplicates`, every row `Catalog facts · manufacturer@2026-10-31`, specs badged `▣ Catalog` |
| `docs/assets/catalog-import-provenance-desktop.png` | `/inventory` | 1440×900 | Committed revision 1; CPU provenance expanded: `model`/`specs` cite `Catalog · manufacturer@2026-10-31` with the AMD URL, `sku`/`quantity`/`price` cite `User-provided · csv:sample.csv` |
| `docs/assets/catalog-import-provenance-mobile.png` | `/inventory` | 390×844 | Same revision as stacked cards; no horizontal overflow (`scrollWidth === clientWidth === 390`) |
| `docs/assets/catalog-sample-check-desktop.png` | `/builds/check` | 1440×900 | The committed sample with one part per category: `12` rules passed, `0` conflicts, `2` unknown, complete build |

## Journey

1. `/inventory` empty state. The download link is described by the licence paragraph:
   “The sample is one real AM5 build … checked on 2026-10-31 … Sample data is CC BY 4.0
   (see NOTICE); its quantity and price are operator-entered stock values, not market
   data.”
2. Uploaded the downloaded sample unchanged and chose **Preview CSV**. Live status:
   `Preview ready: 10 rows · 0 errors · 0 duplicates.` Focus moved to the
   `Review import` heading. Each row shows `Catalog facts · manufacturer@2026-10-31 ·
   match catalog-declared`; each specification badge reads `▣ Catalog` with the source
   in its `title`.
3. **Commit snapshot** produced revision 1: 10 records, 10 units, `US$2,120.90` stock
   value, snapshot `3ef39f57…544d`. The expanded CPU provenance showed catalog facts with
   the manufacturer URL and operator stock facts with `csv:sample.csv` in the same
   disclosure, so the two are distinguishable without opening another screen.
4. `/builds/check` with the CPU, motherboard, memory kit, graphics card, power supply,
   case, cooler and one SSD selected: status
   `Check complete: Unknown. 12 compatible · 0 conflicts · 2 unknown.` The two unknowns
   are `cpu-bios` (no sourced BIOS statement) and `psu-capacity` (no sourced peak power),
   each with a specific follow-up. The rule table reports the socket rule as
   `Compatible` and the PCIe rule as `Graphics card PCIe generation 4 fits the board slot
   generation 5.`
5. Mobile: the same revision at 390×844 stacks into cards; `Catalog facts` and the
   `▣ Catalog` badges remain visible, and `document.documentElement.scrollWidth` equals
   `clientWidth` at 390.

## Keyboard

From a fresh `/inventory`, real Tab key presses moved focus in DOM order: skip link
(`3px` outline) → `RigCraft` → `Inventory` → `Compatibility` → `Recommend` → `Diagnose`
→ the file input → batch currency → `Preview CSV` → `Download sample CSV` at the 11th
stop. The link has `aria-describedby="sample-licence"` and a `3px` focus outline. After
the preview the review heading programmatically receives focus, as before.

## Accessibility

Lighthouse, navigation mode on `/inventory` after the change:

| Device | Accessibility | Best practices | SEO | Agentic browsing | Audits |
| --- | --- | --- | --- | --- | --- |
| Desktop | 100 | 100 | 100 | 100 | 51 passed, 0 failed |
| Mobile | 100 | 100 | 100 | 100 | 51 passed, 0 failed |

Reports: `docs/evidence/catalog-lighthouse/report.{json,html}` and
`docs/evidence/catalog-lighthouse-mobile/report.{json,html}`. The new `Catalog` badge
uses `color-primary` on `color-muted`; `node scripts/check-contrast.mjs` measures that
pair at **5.74:1**, above the 4.5:1 requirement. Every badge also carries a symbol and
text, so none relies on colour alone.

## Privacy

The browser made only same-origin requests to `127.0.0.1:41115` (`/inventory`, the
built assets and `/api/…`). No catalog service, AI provider or third party was contacted.
`docs/data-sources.md` records that OpenDB is not distributed and that the operator runs
the converter locally.

## Limitations

There is still no checked-in Playwright suite. The journeys, keyboard path, responsive
width and Lighthouse runs above were executed through Chrome DevTools MCP; Vitest covers
the badge, label and download-area behaviour.

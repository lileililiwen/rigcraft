# Browser regression suite

The browser behaviour recorded in the other `docs/evidence/*-browser.md` documents is
now executable. `e2e/` holds a Playwright suite that drives the bundle the Rust service
serves, against the shipped sample, with no mocked API and no network access. This
document is the authoritative statement those documents link to.

## Running it

```sh
cargo build          # the service binary the suite starts
npm run build        # the bundle the service serves
npx playwright install firefox webkit   # the two non-Chrome engines (once)
npm run test:e2e     # 50 Chrome journeys + 6 smoke tests each in Firefox and WebKit
```

`npm run test:e2e` selects a free loopback port, starts `target/debug/rigcraft` with a
fresh `RIGCRAFT_DATA_DIR` under the operating system's temporary directory, waits for
`/api/health`, and removes the scratch directory when the run ends. Port 8080 is never
used, a developer's inventory is never read, and `RIGCRAFT_E2E_PORT` pins a port only
when one is needed. The configuration is `playwright.config.ts`: a headless `chromium`
project on `channel: 'chrome'` (the installed Google Chrome, so no browser download),
plus `firefox` and `webkit` projects whose `testMatch` is `/smoke\.spec\.ts/`. One
worker, no retries, list and JSON reporters, and a trace only on retry.

The engines are not Chromium and do not inherit the Chrome channel. The `firefox` and
`webkit` projects run the smoke subset only, so they cannot silently replace a Chrome
journey and the Chrome project keeps every journey, including the pixel baselines. The
WebKit host-library workaround and the exact errors are in
[`cross-browser-smoke.md`](cross-browser-smoke.md).

## What it runs

| Spec | What fails if it regresses |
| --- | --- |
| `e2e/inventory.spec.ts` | The sample download (filename, CSV header, 10 rows, `catalog:manufacturer@2026-10-31`, the AMD page URL), the preview counts and 10 review rows, the commit heading `Committed snapshot · revision 1`, 10 committed records with every seeded SKU, the provenance split: `model`/`specs` as `Catalog · manufacturer@2026-10-31` with the manufacturer URL, `sku`/`quantity`/`price` as `User-provided · csv:rigcraft-inventory-sample.csv` and never as catalog facts, and export being enabled and downloadable straight after the commit. |
| `e2e/compatibility.spec.ts` | Selecting the full seeded AM5 build and reading `12 compatible · 0 conflicts · 2 unknown`, summary 12/0/2, an `Unknown` overall badge that is not `Compatible`, and the `cpu-bios` / `psu-capacity` follow-ups; the 14-row rule matrix with `cpu-socket` values and provenance; and a CPU-only check whose gap reads `A motherboard is required.` / `Select a motherboard from this snapshot.` and never `is required from this snapshot`. |
| `e2e/recommendation.spec.ts` | A 1,800.00 budget returns three ranked options; option 1 is labelled `Unknown` rather than `Compatible`, carries a "Confirm before building" list, and its total is under budget. A 900.00 budget returns `Budget constraint`, "The budget is below the cheapest complete build.", the 1,123.90 minimum and no candidates. |
| `e2e/diagnosis.spec.ts` | One exact, one ambiguous and one unmatched pasted line; no preselected candidate on the ambiguous line; "Resolve the match review first" before resolution; then `Not verified — no rule evaluated (2)`, `Fine as is (0)`, two unverified badges, no compatible badge in the unverified group and no `keep` card at all. Also an all-evaluated two-part diagnosis that prints `Every described part was checked by at least one rule.` exactly once (no `N of N` fraction) and keeps the grammatical `A power supply is required.` gap. |
| `e2e/regression.spec.ts` | `/assets/nope.js`, `/robots.txt` and `/llms.txt` return 404 without the shell while `/diagnose` returns 200 HTML; a malformed JSON POST is `application/json` with `error.code` `invalid_json`; `/diagnose` renders sections 1–5 in order before any match; a `<time>` renders ISO 8601 UTC under `locale: 'de-DE'`. |
| `e2e/shell.spec.ts` | A table caption at 390 px keeps more than 150 px of rendered width (the pre-fix value was 92.58 px); the unfocused skip link's bottom edge is at or above 0 and the first Tab makes it fully visible; `/favicon.ico` and `/favicon.svg` return 200 with an image type, `index.html` links an icon and no icon request fails on a real page load. |
| `e2e/keyboard-responsive.spec.ts` | The first Tab lands on the skip link, Tab reaches the budget field, Enter generates a recommendation with no pointer input, and `document.documentElement.scrollWidth === clientWidth` at 390 px on `/inventory`, `/builds/check`, `/recommend` and `/diagnose`. Also that `Preview CSV`, `Search catalog`, `Match this list` and `Diagnose this build` are disabled with their `aria-describedby` reason visible at 390 px, with no overflow on those pages. |
| `e2e/catalog.spec.ts` | Upload of a small synthetic `rigcraft-catalog` document through `/catalog`, the accepted report and `catalog:synthetic-fixture@fixture-1` provenance, the registry row with `CC0-1.0`/`fixture-1`/3 parts/`Active`, ranked search with the exact SKU first and a stored part opening with its source URL; a refused document naming `Row 1 · model` while its dataset name is absent; and an import enriched from the catalog with the per-row opt-in, three `Catalog` badges and `specs` provenance `Catalog · synthetic-fixture@fixture-1` plus the part URL, while SKU, quantity and price stay `User-provided`. Also asserts no horizontal overflow at 1440 px and 390 px on `/catalog`. |
| `e2e/portability.spec.ts` | CSV and JSON export filenames and bodies (10 lots, revision, authoritative JSON with per-field provenance), re-import of the exported CSV with catalog facts intact, a backup whose first bytes are the SQLite header, a non-SQLite upload rejected as `backup_not_sqlite` with the committed snapshot still visible, an inspected backup showing integrity/schema/counts, the replace button disabled until `RESTORE` is typed exactly, and success naming `inventory.sqlite3.pre-restore-`. |
| `e2e/delivery.spec.ts` | All six client routes render under the CSP with an empty CSP-violation and page-error log, `/recomend` is still a real 404 with no `#root`, and the assistant reaches a Node loopback provider through `/api/ai/parse`, shows the proposal and applies `2000.00` to the budget. |
| `e2e/visual.spec.ts` | Twelve committed pixel baselines: `/inventory`, `/catalog`, `/builds/check`, `/recommend`, `/diagnose` and the `/nope` 404 at 1440×900 and 390×844. Each test re-seeds the sample or resets and re-uploads the synthetic catalog immediately before the capture, masks the dynamic ids, timestamps, sha256 fragment and snapshot `<select>`, and fails on a change beyond the bounded tolerance with a diff artifact. See `visual-regression.md`. |
| `e2e/landmarks.spec.ts` | Each of the six routes exposes exactly one `main` with its heading as the accessible name, and no rendered form is left unnamed (a `<form>` is a `form` landmark only when it has a name); the import, restore, upload, search, parts, budget and assistant-transport forms resolve by `getByRole("form", { name })` on the real pages; and the skip link still focuses the named `main`. See `assistive-tech-gaps.md`. |
| `e2e/smoke.spec.ts` | The six-route shell and titles, the skip link as the first Tab stop and its focus move to `main`, `scrollWidth === clientWidth` at 390 px on every route, `/recomend` as the real 404, a seeded 1,800.00 recommendation with the gaming rule, and the inventory CSV download filename. Runs in the Chromium project and as the only spec in the Firefox and WebKit projects; it asserts no pixels. |

Each test seeds its own snapshot (the inventory journey through the page, the rest
through the documented API) and selects that snapshot by id, so the order does not
matter and one test never reads another's rows. The catalog journeys use a small
synthetic document written to the test output directory; no third-party dataset is
read, uploaded or committed.

## What it does not cover

- The BYOK assistant panel. Driving it would need a provider key or the loopback stub;
  `apps/web/src/components/AssistantPanel.test.tsx`, `tests/ai_planning_api.rs` and
  `tests/ai_provider_transport.rs` remain its coverage.
- Lighthouse and engines other than Chrome beyond the smoke subset. Firefox and
  WebKit run the six smoke journeys only; they have no pixel baselines and no
  full journey parity. The visual record is the committed Chrome baseline set in
  `e2e/visual.spec.ts-snapshots/`; the Lighthouse reports stay the performance
  and accessibility record.
- Any screen-reader result. `assistive-technology-review.md` records the
  accessibility tree, focus order and live regions and hands over the human
  script; no NVDA, VoiceOver or JAWS output is claimed.
- Performance. The suite asserts behaviour and layout, not timings.

## Artifacts

`test-results/` (gitignored) holds the JSON report, a failure screenshot, a trace on
retry and, for a visual failure, the expected, actual and diff images. The twelve
baseline images in `e2e/visual.spec.ts-snapshots/` are committed; no inventory
database or sample copy is.

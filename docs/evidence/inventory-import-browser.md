# Inventory import browser and accessibility evidence

Captured: 2026-10-31, against the working tree based on revision `f6ebf7d4979e0da838946b05f9d848205f545e3` (`Author the five rigcraft implementation packages`). The implementation commit and this evidence are recorded in `HANDOFF.md`.

- Build under test: `npm run build` → `apps/web/dist`, served by `cargo run` from the same process.
- Service: `RIGCRAFT_DATA_DIR=/tmp/opencode/rigcraft-evidence/data RIGCRAFT_ADDR=127.0.0.1:8091 cargo run`
- Browser: Google Chrome through Chrome DevTools MCP (production build, not the Vite dev server).
- Data: `tests/fixtures/inventory/mixed.csv` and the sample CSV served by `GET /api/imports/sample`. Both are synthetic; no personal or real shop inventory was used.

## Captures

| File | Viewport | State proved |
| --- | --- | --- |
| `docs/assets/inventory-empty-desktop.png` | 1440×900 | Empty state, sample download, currency field, privacy note. |
| `docs/assets/inventory-review-desktop.png` | 1440×900 | 6-row mixed import, 3 field errors with row numbers, duplicate group resolution, corrected row included, commit enabled. |
| `docs/assets/inventory-correction-desktop.png` | 1440×900 | Row-correction editor focused on the invalid quantity field. |
| `docs/assets/inventory-committed-desktop.png` | 1440×900 | Committed revision 1, totals, per-record provenance disclosure, unknown-state badges, history. |
| `docs/assets/inventory-revision2-desktop.png` | 1440×900 | Correction created revision 2 with parent link and updated stock value. |
| `docs/assets/inventory-earlier-revision-desktop.png` | 1440×900 | Revision 1 still readable after revision 2; earlier-revision notice and provenance expanded. |
| `docs/assets/inventory-retry-desktop.png` | 1440×900 | Offline network failure: `role="alert"` with `network_error`, selected file preserved, Retry offered. |
| `docs/assets/inventory-focus-desktop.png` | 1440×900 | Skip link focused with a 3 px visible outline. |
| `docs/assets/inventory-review-mobile.png` | 390×844 | Stacked row cards with visible labels, duplicate and error states, no horizontal overflow. |
| `docs/assets/inventory-committed-mobile.png` | 390×844 | Committed records and provenance at phone width. |
| `docs/assets/inventory-revision2-mobile.png` | 390×844 | Corrected revision at phone width. |

## Journey exercised

1. Loaded `/inventory`; empty state announced `No CSV selected.`
2. Uploaded `mixed.csv` with currency `USD`; preview announced `Preview ready: 6 rows · 3 errors · 2 duplicates.`
3. Confirmed row-level messages: row 4 `quantity_format`, row 5 required price, row 6 `currency_mismatch`.
4. Opened row 4 correction; focus landed on Quantity (the first invalid field), changed `abc` to `3`, applied, and included the corrected row (badge changed to `Corrected`).
5. Chose `Merge quantities` for duplicate group `dup-1`; commit became enabled; committed revision 1 with 3 records, 7 units, `US$823.00`.
6. Created revision 2 for CPU-1: quantity 3 → 2, price 199.00 → 189.50, note `Stock recount after mixed import`. Revision 2 shows 3 records, 6 units, `US$605.00`, parent link to revision 1.
7. Opened revision 1 from history: quantity 3 and `US$823.00` unchanged, with the earlier-revision notice.
8. Took the browser offline, attempted preview, saw `Request failed` / `network_error` with the file still selected; restored the network and clicked Retry, reaching `Preview ready: 8 rows · 0 errors · 0 duplicates.`
9. Repeated review and committed views at 390×844 and confirmed `scrollWidth === clientWidth === 390` (no horizontal overflow).

## Keyboard and focus

Real Tab presses through the empty state, with computed styles read from the focused element:

| Tab | Focused element | Outline |
| --- | --- | --- |
| 1 | Skip to inventory | solid, 3 px, `rgb(29, 78, 216)` |
| 2 | RigCraft brand link | solid, 3 px |
| 3 | Inventory navigation link | solid, 3 px |
| 4 | Inventory CSV file input | solid, 3 px |

- The skip link moves from `top: -2rem` to `top: 8px` when focused, and targets `#main`.
- After preview succeeds, focus moves to the `Review import` heading (`tabIndex=-1`); after commit or snapshot navigation, focus moves to `Committed snapshot · revision N`.
- After a failed preview/commit, focus moves to the `role="alert"` error region.
- The row-correction editor focuses the first invalid field (Quantity in the captured flow), not merely the first input.
- The page needs no pointer gesture: file selection, currency, mapping, decisions, duplicate resolution and correction are standard form controls; the table is not a drag surface.

## Measured contrast

`node scripts/check-contrast.mjs` resolved the vendored tokens and measured every pair the interface uses. Full values and the token source SHA-256 are in `docs/evidence/token-contrast.json`.

| Pair | Ratio | Minimum |
| --- | --- | --- |
| Body text on page | 17.06:1 | 4.5:1 |
| Body text on surface | 9.90:1 | 4.5:1 |
| Secondary text on page | 16.30:1 | 4.5:1 |
| Secondary text on muted | 9.45:1 | 4.5:1 |
| Primary button label | 9.90:1 | 4.5:1 |
| Primary button hover label | 12.56:1 | 4.5:1 |
| Link on surface | 5.74:1 | 4.5:1 |
| Error text on surface | 5.46:1 | 4.5:1 |
| Success text on muted | 7.37:1 | 4.5:1 |
| Warning text on muted | 7.18:1 | 4.5:1 |
| Control border on surface | 6.97:1 | 3:1 |
| Focus ring on page | 12.56:1 | 3:1 |
| Focus ring on surface | 7.29:1 | 3:1 |

Statuses never rely on color: `✓ Valid`, `✕ Error`, `⇄ Duplicate SKU`, `? Unknown` and `✓ Corrected` carry text and symbols.

## Lighthouse

Snapshot audits of the committed revision page:

- `docs/evidence/inventory-lighthouse/report.json` and `.html` — desktop (1440×900), accessibility **100**, best practices 100, SEO 100, 36 passed, 0 failed.
- `docs/evidence/inventory-lighthouse-mobile/report.json` and `.html` — mobile device profile (390×844), accessibility **100**, best practices 100, SEO 100, 36 passed, 0 failed.

The automated audits found no failing accessibility audit. This is not a manual screen-reader pass: no NVDA/VoiceOver session was run, and that limitation is recorded rather than waived.

Reduced motion is handled with a `prefers-reduced-motion: reduce` block that removes the spinner animation and caps transition duration; this was verified in the stylesheet, not by a recorded motion trace.

## Privacy review

- Both fixtures are synthetic test data created for this repository.
- The service ran with an isolated data directory under `/tmp/opencode/rigcraft-evidence`; no workspace file and no home-directory inventory database was touched.
- Server output contained only startup lines. `grep` for `CPU-1`, `RAM-1`, `mixed.csv` and `socket` in the service log returned 0 matches, so CSV content and prices were not logged.
- Screenshots show only synthetic rows and the local snapshot IDs; no token, credential or personal identifier appears.
- The UI states that inventory stays local and is not sent to a catalog or AI provider. No network destination other than the same-origin local service was contacted during the journey.

## Final build smoke check

After the capture session, two non-visual corrections were made: an explicitly empty mapping now means "not mapped" instead of silently auto-detecting, and focus now moves to the new snapshot heading after a correction. The final build was rebuilt, the full Forge Gate re-ran green, and a fresh database smoke flow confirmed:

- upload `mixed.csv` → merge duplicate `dup-1` → commit revision 1 (`2 records`, `4 units`, `US$676.00`);
- correct CPU-1 quantity 3 → 2 → revision 2 (`2 records`, `3 units`, `US$477.00`, parent `434b4b50…e384`);
- `document.activeElement` after the correction is the `Committed snapshot · revision 2` heading (`tabIndex=-1`), so focus is not lost when the editor unmounts.

The screenshots above were captured immediately before those two corrections; the correction/commit/revision states and layout are unchanged, but the final smoke result above is the authoritative check for the shipped build.

## Reproduction caveats

- The browser checks are now executable: `e2e/inventory.spec.ts` replays this journey (download, preview, commit, provenance split) on every run, and `e2e/keyboard-responsive.spec.ts` holds the 390 px width. The captures above were made with Chrome DevTools MCP and remain the visual record. See `docs/evidence/browser-suite.md`.
- Evidence is bound to the revision named above. Re-run the commands after any change to the UI, tokens or API before reusing these captures.

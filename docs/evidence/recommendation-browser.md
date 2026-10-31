# Build recommendation browser and accessibility evidence

Captured: 2026-10-31, against the working tree based on revision `99583ae`
(`Record compatibility implementation evidence`). The implementation commit and this
evidence are recorded in `HANDOFF.md`.

- Build under test: `npm run build` → `apps/web/dist` (`index-D0wejJyE.js`, `index-e-eAo_uI.css`), served by the Rust process from the same directory. The captures below were taken against this bundle; a later file-size split with no behavioural change produced `index-kJYHle2W.js`, and the final smoke check at the end was re-run against it.
- Service: `RIGCRAFT_DATA_DIR=/tmp/opencode/rigcraft-recommend-evidence/data RIGCRAFT_ADDR=127.0.0.1:8094 ./target/debug/rigcraft`
- Browser: Google Chrome through Chrome DevTools MCP (production build, not the Vite dev server). Desktop 1440×900 and mobile 390×844.
- Data: synthetic fixtures in `tests/fixtures/recommendation/` (`feasible.csv`, `conflict-only.csv`) and the existing `tests/fixtures/compatibility/unknown.csv`. Every model name starts with `Fixture`; no licensed catalog data was used or implied.
- The journey involved no AI provider, no API key and no network service other than the local process.

## Captures

| File | Viewport | State proved |
| --- | --- | --- |
| `docs/assets/recommendation-empty-desktop.png` | 1440×900 | No snapshot exists; the page explains the empty state and links to `/inventory`. |
| `docs/assets/recommendation-form-desktop.png` | 1440×900 | Snapshot loaded, budget and use case fields, inventory-only statement, collapsed lock/exclude pickers. |
| `docs/assets/recommendation-results-desktop.png` | 1440×900 | Three options, `✓ Compatible`, totals, budget deltas, stock units, scores, reasons and the parts/price/stock table. |
| `docs/assets/recommendation-evidence-desktop.png` | 1440×900 | Expanded rule evidence with the compared values and their `csv:` provenance. |
| `docs/assets/recommendation-compare-desktop.png` | 1440×900 | Comparison table: total, budget delta, compatibility, completeness, stock, score, every category part and the check report id. |
| `docs/assets/recommendation-export-desktop.png` | 1440×900 | JSON export downloaded and announced: `rigcraft-recommendation-c316fdf0.json`. |
| `docs/assets/recommendation-no-solution-desktop.png` | 1440×900 | Budget binding: 1,012.00 minimum vs an 800.00 budget, 212.00 shortfall, cheapest part per category, adjustable constraints. |
| `docs/assets/recommendation-compatibility-desktop.png` | 1440×900 | Compatibility binding: zero candidates, 4 complete combinations rejected by `cpu-socket`. |
| `docs/assets/recommendation-unknown-desktop.png` | 1440×900 | Two `? Unknown` options with BIOS and front-radiator clearance follow-ups; unknown is distinct from verified compatibility. |
| `docs/assets/recommendation-focus-desktop.png` | 1440×900 | Skip link focused with a visible 3 px outline. |
| `docs/assets/recommendation-results-mobile.png` | 390×844 | Ranked option cards stack at phone width; no horizontal overflow. |
| `docs/assets/recommendation-compare-mobile.png` | 390×844 | Comparison rows stack into labelled cards at phone width. |
| `docs/assets/recommendation-no-solution-mobile.png` | 390×844 | Binding constraint, blockers and adjustable constraints readable at phone width. |

## Journey exercised

1. Opened `/recommend` with an empty data directory: the live status announced `No inventory snapshot is available. Import a CSV first.` and the page linked to `/inventory`.
2. Imported `feasible.csv` through the inventory UI (file picker, batch currency USD, preview `13 rows · 0 errors · 0 duplicates`) and committed it. The status announced `Snapshot committed. Revision 1, 13 records, US$3,934.00 stock value.`
3. Returned to `/recommend`, entered `2000.00` and generated with the gaming use case. The live region announced `Recommendation complete: 3 options within US$2,000.00.` and focus moved to the `Recommended builds` heading.
4. Read the three options: `1. Best overall` 1,362.00 USD (638.00 under budget, score 64.65, 8 stock units), `2. Lowest cost` 1,012.00 USD (988.00 under, score 59.45) and `3. Alternative build` 1,402.00 USD (598.00 under, score 64.05). Each was complete and `✓ Compatible`, with reasons naming cost, stock and rule evidence.
5. Expanded `Rule evidence for this build`: 14 rules with their compared values, `user` provenance references and `compat-rules-v1`. Expanded `Alternatives not selected` to read why a same-category part was not chosen.
6. Compared the options: the comparison table shows totals, budget deltas, compatibility, completeness, stock, score, every category part and the check report id. `search` reported 192 complete combinations checked out of 100,000 nodes, 64 valid builds and `limited: false`.
7. Downloaded the JSON export; the status announced `Export downloaded: rigcraft-recommendation-c316fdf0.json`.
8. Changed the budget to `800.00`: the previous results were marked stale in a bordered banner and announced. Generating returned `No solution. The budget is below the cheapest complete build.` with the 1,012.00 minimum, 212.00 shortfall, the cheapest part per required category and the two adjustable constraints. No candidate was shown.
9. Imported `conflict-only.csv` and generated against it: `No solution. No complete build passes the compatibility rules.` with 0 candidates, 4 combinations rejected by `cpu-socket`, and no invalid build presented.
10. Imported `unknown.csv` and generated: 2 options, both `? Unknown`, with the specific BIOS-support and front-radiator-clearance follow-ups. Unknown results are visually and textually distinct from verified ones.
11. Repeated the results, comparison and no-solution states at 390×844. `document.documentElement.scrollWidth === clientWidth` (390) in every state; no horizontal page overflow.

## Keyboard and focus

Real Tab, Shift+Tab and Enter presses through the production build:

| Step | Focused element | Computed outline |
| --- | --- | --- |
| 1 | `Skip to recommendation` (top moves to 8 px) | solid, 3 px, `rgb(29, 78, 216)` |
| 2–5 | RigCraft brand, Inventory, Compatibility, Recommend navigation links | solid, 3 px |
| 6 | Snapshot select | solid, 3 px |
| 7 | `Total budget (USD)` text field (typed `2000.00`) | solid, 3 px |
| 8 | `Primary use` select | solid, 3 px |
| 9–10 | `Choose owned parts`, `Choose excluded parts` disclosures | solid, 3 px |
| 11 | `Generate recommendations` button, activated with Enter | solid, 3 px |
| Result | Focus moves to `Recommended builds` (`tabIndex=-1`) | heading, no outline by design |

- The generation was completed with keyboard only: no pointer gesture was used to fill or submit the form.
- Lock/exclude choices are native checkboxes inside native `<details>` disclosures, so they are reachable and expandable from the keyboard.
- Rule evidence and alternatives are native `<details>/<summary>` disclosures.
- The live status region announced loading, completion, no solution and export results. The error alert (covered by the component tests) receives focus.
- Status is symbol plus text: `✓ Compatible`, `? Unknown`, plus a text banner for stale and search-limited states.

## Measured contrast

`node scripts/check-contrast.mjs` resolved the vendored tokens and measured every pair the interface uses. Full values and the token source SHA-256 are in `docs/evidence/token-contrast.json`.

| Pair | Ratio | Minimum |
| --- | --- | --- |
| Body text on page | 17.06:1 | 4.5:1 |
| Body text on surface | 9.90:1 | 4.5:1 |
| Secondary text on page | 16.30:1 | 4.5:1 |
| Secondary text on muted | 9.45:1 | 4.5:1 |
| Primary button label | 9.90:1 | 4.5:1 |
| Link on surface | 5.74:1 | 4.5:1 |
| Error text on surface (no-solution block) | 5.46:1 | 4.5:1 |
| Error text on muted (incompatible badge) | 5.46:1 | 4.5:1 |
| Success text on muted (compatible badge) | 7.37:1 | 4.5:1 |
| Warning text on muted (unknown badge) | 7.18:1 | 4.5:1 |
| Warning text on page (follow-up text) | 12.38:1 | 4.5:1 |
| Control border on surface | 6.97:1 | 3:1 |
| Focus ring on page | 12.56:1 | 3:1 |
| Focus ring on surface | 7.29:1 | 3:1 |

## Lighthouse

Snapshot audits of the unknown-result page:

- `docs/evidence/recommendation-lighthouse/report.json` and `.html` — desktop (1440×900), accessibility **100**, best practices 100, 38 audits passed, 0 accessibility audits failed.
- `docs/evidence/recommendation-lighthouse-mobile/report.json` and `.html` — mobile device profile (390×844), accessibility **100**, best practices 100, 38 audits passed, 0 accessibility audits failed.

Not all categories are 100: SEO scored 80 because `/robots.txt` is not served (the SPA fallback returns the shell), and the agentic-browsing audits (`llms.txt`, `ai-catalog.json`) are not provided by this product. Those are recorded rather than presented as clean. The automated audits are not a manual screen-reader pass: no NVDA/VoiceOver session was run, and that limitation is recorded rather than waived.

Reduced motion is handled by the existing `prefers-reduced-motion: reduce` block; the recommendation page adds no animation beyond the existing loading spinner, which that block removes.

## Export round-trip check

Generated through the API against the same snapshot, then downloaded both formats:

- `GET /api/recommendations/{id}/export?format=json` — `exportVersion: recommend-export-v1`; the embedded `report` object equals the stored report; every `checks[]` entry names the candidate's `checkReportId`, `compat-rules-v1`, its status and summary; the snapshot object carries id, revision 1, currency USD, `feasible.csv` and its SHA-256.
- `GET /api/recommendations/{id}/export?format=markdown` — 7,752 bytes, contains `## Ranked options`, the same check report ids, `## Comparison`, `## Evidence sources` and the manual-confirmation notice.
- `format=xml` returned HTTP 400 `export_format_unsupported`.

## Privacy and stock review

- All fixtures are synthetic test data created for this repository and labelled `Fixture` in every model name. They were imported as user-owned inventory; no data was presented as licensed catalog content.
- The service ran with an isolated data directory under `/tmp/opencode/rigcraft-recommend-evidence`; no workspace file and no home-directory inventory database was touched.
- The service log had 1 line (startup). `grep` for `FIX-`, `feasible.csv`, `conflict-only.csv`, `unknown.csv`, `socket=`, `perf_score` and `budgetMinor` returned 0 matches, so inventory rows, specifications, constraints and recommendation requests were not logged.
- The snapshot totals stayed at 13 records / 26 units / 3,934.00 USD across repeated recommendations: generating a plan writes reports only and never reserves or decrements stock.
- Screenshots show only synthetic rows, local snapshot and report ids, and rule evidence; no token, credential or personal identifier appears.
- The UI states that the search runs locally, consults no catalog or AI provider, and does not reserve stock.

## Final build smoke check

After the capture session the final binary and Web bundle were restarted and checked:

- Final bundle `index-kJYHle2W.js` (after the pure file-size split) served with `/recommend` → `200 text/html`; `/api/health` → `{"status":"ok"}`. The split moved the candidate card into `apps/web/src/components/CandidateCard.tsx`; the rendered results were identical: three options at 1,362.00 / 1,012.00 / 1,402.00 USD with scores 64.65 / 59.45 / 64.05.
- `POST /api/recommendations` for `feasible.csv`: `status ok`, `recommend-score-v1`, `compat-rules-v1`, 3 candidates, all complete and compatible, 64 valid builds from 192 complete combinations and 646 nodes inside the 100,000-node limit.
- `GET /api/recommendations/{id}` returned the stored report with `stale: false` and `currentSnapshotRevision: 1`.

## Reproduction caveats

- The browser checks are now executable: `e2e/recommendation.spec.ts` replays this journey (a ranked option under budget with an honest `Unknown` label, the budget no-solution with its minimum) and `e2e/keyboard-responsive.spec.ts` completes the request from the keyboard alone at 390 px. The captures above were made with Chrome DevTools MCP and remain the visual record. See `docs/evidence/browser-suite.md`.
- The search-limit path is proven by `tests/recommendation_api.rs::a_large_inventory_stops_at_the_stated_work_limit`; the browser dataset was small enough to complete, which is the intended experience.
- Evidence is bound to the revision named above. Re-run the commands after any change to the UI, tokens, scoring version, rule set or API before reusing these captures.

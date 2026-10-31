# Compatibility check browser and accessibility evidence

Captured: 2026-10-31, against the working tree based on revision `0418f7a` (`Record inventory import implementation evidence`). The implementation commit and this evidence are recorded in `HANDOFF.md`.

- Build under test: `npm run build` → `apps/web/dist`, served by `cargo run` from the same process.
- Service: `RIGCRAFT_DATA_DIR=/tmp/opencode/rigcraft-compat-evidence/data RIGCRAFT_ADDR=127.0.0.1:8092 cargo run`
- Browser: Google Chrome through Chrome DevTools MCP (production build, not the Vite dev server).
- Data: the three synthetic CSVs in `tests/fixtures/compatibility/` (`compatible.csv`, `conflict.csv`, `unknown.csv`), imported through `POST /api/imports` and committed as ordinary user-owned snapshots. Every model name starts with `Fixture`; no licensed catalog data was used or implied.
- A second service with an empty data directory on port 8093 produced the no-inventory capture.

## Captures

| File | Viewport | State proved |
| --- | --- | --- |
| `docs/assets/compatibility-empty-inventory-desktop.png` | 1440×900 | No snapshot exists; the page explains the empty state and links to `/inventory`. |
| `docs/assets/compatibility-empty-desktop.png` | 1440×900 | Snapshot loaded with no parts selected; single-part radios, memory/storage checkboxes, disabled quantity fields, per-part facts disclosure. |
| `docs/assets/compatibility-compatible-desktop.png` | 1440×900 | `✓ Compatible`, complete build, 14 rules passed, 0 conflicts, 0 unknown. |
| `docs/assets/compatibility-stale-desktop.png` | 1440×900 | Editing the CPU quantity marks the previous verdict stale and announces it; re-check is offered. |
| `docs/assets/compatibility-conflict-desktop.png` | 1440×900 | `✕ Incompatible` with nine named conflicts: socket, memory type, GPU length, cooler clearance, cooler mounting, board/case form factor, PSU mounting, GPU connectors, storage interface. |
| `docs/assets/compatibility-unknown-desktop.png` | 1440×900 | `? Unknown` with no conflicts; BIOS support and liquid-radiator clearance follow-ups are named. |
| `docs/assets/compatibility-unknown-evidence-desktop.png` | 1440×900 | Expanded `cooler-clearance` evidence: `radiator_size=240 mm` cites `csv:unknown.csv`; `front_radiator_mm` is `Not provided` with no source. |
| `docs/assets/compatibility-retry-desktop.png` | 1440×900 | Offline check failure: `role="alert"` with `network_error`, selection preserved, Retry offered. Retry after reconnect reached a result. |
| `docs/assets/compatibility-focus-desktop.png` | 1440×900 | Skip link focused with a 3 px visible outline. |
| `docs/assets/compatibility-select-mobile.png` | 390×844 | Stacked part cards at phone width with visible labels, no horizontal overflow. |
| `docs/assets/compatibility-unknown-mobile.png` | 390×844 | Unknown verdict, stacked summary and follow-ups at phone width. |
| `docs/assets/compatibility-conflict-mobile.png` | 390×844 | Incompatible verdict and conflict diagnostics at phone width. |
| `docs/assets/compatibility-compatible-mobile.png` | 390×844 | Compatible verdict at phone width. |

## Journey exercised

1. Opened `/builds/check` with no inventory: live status announced `No inventory snapshot is available. Import a CSV first.` and the page linked to `/inventory`.
2. Imported `compatible.csv`, `conflict.csv` and `unknown.csv` through the inventory API and committed each snapshot. The snapshot selector shows the source filename and short snapshot ID so three same-second revisions are distinguishable.
3. Selected all eight fixture parts; the live status moved from `0 units selected` to `8 units selected` and each quantity field became enabled with its stock maximum.
4. Ran the check on `compatible.csv`: overall `✓ Compatible`, complete build, 14 compatible rules, 0 conflicts, 0 unknown. Focus moved to `Compatibility result`.
5. Changed the CPU quantity from 1 to 2: a stale banner appeared, the live region announced the stale verdict, and the old verdict stayed visible only as stale until re-check.
6. Ran the check on `conflict.csv`: overall `✕ Incompatible`, 9 conflicts and 1 unknown (BIOS). Conflict diagnostics and the rule table both named the compared values.
7. Ran the check on `unknown.csv`: overall `? Unknown`, 12 compatible, 0 conflicts, 2 unknown. The BIOS rule asked which installed BIOS supports the CPU; the cooler rule asked for front-radiator clearance and named the missing `front_radiator_mm` fact.
8. Expanded the cooler evidence: the radiator size cites `user` / `csv:unknown.csv` / observation time; the missing clearance shows `Not provided` and `No source recorded`.
9. Took the browser offline and submitted a check: the alert showed `Request failed` / `network_error`, the CPU radio stayed selected, and Retry was offered. After restoring the network, Retry produced a result.
10. Repeated the compatible, conflict and unknown result states at 390×844. `document.documentElement.scrollWidth === clientWidth === 390` in each state; no horizontal overflow.

## Keyboard and focus

Real Tab, arrow and Shift+Tab presses through the production build:

| Step | Focused element | Computed outline |
| --- | --- | --- |
| 1 | `Skip to compatibility check` (top moves to 8 px) | solid, 3 px, `rgb(29, 78, 216)` |
| 2 | RigCraft brand link | solid, 3 px |
| 3 | Inventory navigation link | solid, 3 px |
| 4 | Compatibility navigation link (`aria-current="page"`) | solid, 3 px |
| 5 | Snapshot select | solid, 3 px |
| 6 | CPU radio group (`No cpu selected`) | solid, 3 px |
| 7 | ArrowDown selects `Fixture AM5 CPU` | solid, 3 px |
| 8 | Tab reaches `Quantity for FIX-CPU-AM5`, now enabled | solid, 3 px |
| Result | After the check, focus moves to `Compatibility result` (`tabIndex=-1`) | heading, no outline by design |

- Radio groups are one tab stop, so ArrowDown/ArrowUp select a part; Tab then reaches that part's quantity field. No pointer gesture is required.
- The result heading receives focus after a successful check; `role="alert"` failures receive focus, and `role="status"` announces loading, check completion and staleness.
- Evidence is native `<details>/<summary>`, so it is expandable from the keyboard.
- Status is symbol plus text: `✓ Compatible`, `✕ Incompatible`, `? Unknown`; stale state adds a bordered text banner.

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
| Error text on muted (incompatible badge) | 5.46:1 | 4.5:1 |
| Success text on muted (compatible badge) | 7.37:1 | 4.5:1 |
| Warning text on muted (unknown badge) | 7.18:1 | 4.5:1 |
| Warning text on page (follow-up text) | 12.38:1 | 4.5:1 |
| Control border on surface | 6.97:1 | 3:1 |
| Focus ring on page | 12.56:1 | 3:1 |
| Focus ring on surface | 7.29:1 | 3:1 |

## Lighthouse

Snapshot audits of the compatible result page:

- `docs/evidence/compatibility-lighthouse/report.json` and `.html` — desktop (1440×900), accessibility **100**, best practices 100, SEO 100, 38 passed, 0 failed.
- `docs/evidence/compatibility-lighthouse-mobile/report.json` and `.html` — mobile device profile (390×844), accessibility **100**, best practices 100, SEO 100, 38 passed, 0 failed.

The automated audits found no failing accessibility audit. This is not a manual screen-reader pass: no NVDA/VoiceOver session was run, and that limitation is recorded rather than waived.

Reduced motion is handled by the existing `prefers-reduced-motion: reduce` block that removes the spinner animation and caps transition duration; the compatibility page adds no new animation.

## Privacy review

- All three fixtures are synthetic test data created for this repository and labelled `Fixture` in every model name. They were imported as user-owned inventory; no data was presented as licensed catalog content.
- The service ran with an isolated data directory under `/tmp/opencode/rigcraft-compat-evidence`; no workspace file and no home-directory inventory database was touched.
- The service log had 5 lines. `grep` for `FIX-`, `compatible.csv`, `conflict.csv`, `unknown.csv`, `socket=` and `bios_supports` returned 0 matches, so check selections, specifications and fixture contents were not logged.
- Screenshots show only synthetic rows, local snapshot IDs and rule evidence; no token, credential or personal identifier appears.
- The UI states that the check runs locally, consults no catalog or AI provider, and does not save the selection as stock.

## Final build smoke check

After the capture session, the SPA fallback was corrected: `ServeDir::not_found_service` had served the correct index body with a `404` status for deep links. The fallback now returns the shell with `200`, and `tests/compatibility_api.rs::spa_routes_serve_the_shell_with_200` checks `/inventory`, `/builds/check` and a nested client route. The final binary was restarted and confirmed:

- `GET /`, `GET /inventory` and `GET /builds/check` → `200 text/html`.
- `POST /api/builds/check` for the compatible fixture → `compatible`, 14 rules, rule set `compat-rules-v1`, completeness `complete`.
- Final Web build hash `index-WKsqFuqM.js`; the rendered states captured above are unchanged.

## Reproduction caveats

- The browser checks are now executable: `e2e/compatibility.spec.ts` replays this journey (the full seeded build, the 12/0/2 verdict, the named unknowns) on every run, and `e2e/keyboard-responsive.spec.ts` holds the 390 px width. The captures above were made with Chrome DevTools MCP and remain the visual record. See `docs/evidence/browser-suite.md`.
- Evidence is bound to the revision named above. Re-run the commands after any change to the UI, tokens, rule set or API before reusing these captures.

# Documentation capture plan

Source: the local Web UI at `/inventory` and `/builds/check`. Capture tool: Chrome DevTools MCP against the production build served by `cargo run`; `shot-scraper` remains an alternative for deterministic stills. Viewports: 1440×900 desktop and 390×844 mobile.

Wait for the page's stable ready state, then capture a state only after the live status region names it (`Preview ready…`, `Committed snapshot…`, `Snapshot committed…`). Store screenshots in `docs/assets/`, and record the source revision, route, viewport, action or wait condition, command and privacy review with each capture.

## Delivered captures (2026-10-31)

| Capture | Route | Viewport | Condition |
| --- | --- | --- | --- |
| `inventory-empty-desktop.png` | `/inventory` | 1440×900 | No file selected |
| `inventory-review-desktop.png` | `/inventory` | 1440×900 | Mixed CSV preview, duplicates unresolved then resolved, correction applied |
| `inventory-correction-desktop.png` | `/inventory` | 1440×900 | Row 4 editor open, Quantity focused |
| `inventory-committed-desktop.png` | `/inventory` | 1440×900 | Snapshot revision 1 committed |
| `inventory-revision2-desktop.png` | `/inventory` | 1440×900 | Correction created revision 2 |
| `inventory-earlier-revision-desktop.png` | `/inventory` | 1440×900 | Revision 1 reopened, provenance expanded |
| `inventory-retry-desktop.png` | `/inventory` | 1440×900 | Offline preview failure with Retry |
| `inventory-focus-desktop.png` | `/inventory` | 1440×900 | Skip link focused via Tab |
| `inventory-review-mobile.png` | `/inventory` | 390×844 | Mixed CSV preview, stacked cards |
| `inventory-committed-mobile.png` | `/inventory` | 390×844 | Revision 1 records and provenance |
| `inventory-revision2-mobile.png` | `/inventory` | 390×844 | Revision 2 records |

### Compatibility captures (2026-10-31)

| Capture | Route | Viewport | Condition |
| --- | --- | --- | --- |
| `compatibility-empty-inventory-desktop.png` | `/builds/check` | 1440×900 | No inventory snapshot exists |
| `compatibility-empty-desktop.png` | `/builds/check` | 1440×900 | Snapshot loaded, no parts selected |
| `compatibility-compatible-desktop.png` | `/builds/check` | 1440×900 | Complete compatible fixture build |
| `compatibility-stale-desktop.png` | `/builds/check` | 1440×900 | Quantity edited after a check; stale verdict |
| `compatibility-conflict-desktop.png` | `/builds/check` | 1440×900 | Incompatible fixture with nine conflicts |
| `compatibility-unknown-desktop.png` | `/builds/check` | 1440×900 | Unknown fixture with BIOS and radiator follow-ups |
| `compatibility-unknown-evidence-desktop.png` | `/builds/check` | 1440×900 | Expanded rule evidence with a missing fact |
| `compatibility-retry-desktop.png` | `/builds/check` | 1440×900 | Offline check failure with Retry |
| `compatibility-focus-desktop.png` | `/builds/check` | 1440×900 | Skip link focused via Tab |
| `compatibility-select-mobile.png` | `/builds/check` | 390×844 | Stacked part selection |
| `compatibility-unknown-mobile.png` | `/builds/check` | 390×844 | Unknown verdict and follow-ups |
| `compatibility-conflict-mobile.png` | `/builds/check` | 390×844 | Incompatible verdict and conflicts |
| `compatibility-compatible-mobile.png` | `/builds/check` | 390×844 | Compatible verdict |

Details, commands, keyboard results, measured contrast and privacy review: `docs/evidence/inventory-import-browser.md` and `docs/evidence/compatibility-browser.md`. Measured token contrast: `docs/evidence/token-contrast.json`. Lighthouse reports: `docs/evidence/inventory-lighthouse*/` and `docs/evidence/compatibility-lighthouse*/`.

## Refresh rule

Refresh these captures after any change to `/inventory`, `/builds/check`, the shared tokens, the import or check API, the compatibility rules, or the rendered states. A capture that names an older revision is historical evidence, not current evidence.

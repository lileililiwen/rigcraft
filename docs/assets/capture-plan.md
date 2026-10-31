# Documentation capture plan

Source: the local Web UI at `/inventory`. Capture tool: Chrome DevTools MCP against the production build served by `cargo run`; `shot-scraper` remains an alternative for deterministic stills. Viewports: 1440×900 desktop and 390×844 mobile.

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

Details, commands, keyboard results, measured contrast and privacy review: `docs/evidence/inventory-import-browser.md`. Measured token contrast: `docs/evidence/token-contrast.json`. Lighthouse reports: `docs/evidence/inventory-lighthouse/` and `docs/evidence/inventory-lighthouse-mobile/`.

## Refresh rule

Refresh these captures after any change to `/inventory`, the shared tokens, the import API or the rendered states. A capture that names an older revision is historical evidence, not current evidence.

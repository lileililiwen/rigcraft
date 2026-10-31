# Documentation capture plan

Source: the local Web UI at `/inventory`, `/builds/check` and `/recommend`. Capture tool: Chrome DevTools MCP against the production build served by `cargo run`; `shot-scraper` remains an alternative for deterministic stills. Viewports: 1440×900 desktop and 390×844 mobile.

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

### Recommendation captures (2026-10-31)

| Capture | Route | Viewport | Condition |
| --- | --- | --- | --- |
| `recommendation-empty-desktop.png` | `/recommend` | 1440×900 | No inventory snapshot exists |
| `recommendation-form-desktop.png` | `/recommend` | 1440×900 | Snapshot loaded, budget and constraints empty |
| `recommendation-results-desktop.png` | `/recommend` | 1440×900 | Three compatible options with reasons and stock |
| `recommendation-evidence-desktop.png` | `/recommend` | 1440×900 | Expanded rule evidence with provenance |
| `recommendation-compare-desktop.png` | `/recommend` | 1440×900 | Comparison table across three options |
| `recommendation-export-desktop.png` | `/recommend` | 1440×900 | JSON export downloaded and announced |
| `recommendation-no-solution-desktop.png` | `/recommend` | 1440×900 | Budget binding constraint with shortfall |
| `recommendation-compatibility-desktop.png` | `/recommend` | 1440×900 | Compatibility binding: every combination rejected |
| `recommendation-unknown-desktop.png` | `/recommend` | 1440×900 | Two unknown options with follow-ups |
| `recommendation-focus-desktop.png` | `/recommend` | 1440×900 | Skip link focused via Tab |
| `recommendation-results-mobile.png` | `/recommend` | 390×844 | Ranked option cards stacked |
| `recommendation-compare-mobile.png` | `/recommend` | 390×844 | Comparison rows stacked as labelled cards |
| `recommendation-no-solution-mobile.png` | `/recommend` | 390×844 | Binding constraint, blockers and adjustments |

### BYOK planning captures (2026-10-31)

Captured against a loopback OpenAI-compatible stub (`scripts/mock-openai-provider.py`); no real provider and no real key were used.

| Capture | Route | Viewport | Condition |
| --- | --- | --- | --- |
| `byok-no-key-desktop.png` | `/recommend` | 1440×900 | No key configured; parse disabled with the reason |
| `byok-consent-desktop.png` | `/recommend` | 1440×900 | Transport fields and sentence filled, consent unticked |
| `byok-panel-desktop.png` | `/recommend` | 1440×900 | Assistant panel in the proposal state |
| `byok-proposal-desktop.png` | `/recommend` | 1440×900 | Proposed request with per-field sources |
| `byok-proposal-edited-desktop.png` | `/recommend` | 1440×900 | Budget edited; source line marks `· Edited by you` |
| `byok-confirmed-desktop.png` | `/recommend` | 1440×900 | Confirmed; values applied and focus on Generate |
| `byok-results-desktop.png` | `/recommend` | 1440×900 | Deterministic results after the confirmed request |
| `byok-results-options-desktop.png` | `/recommend` | 1440×900 | Results region with the locked part at zero cost |
| `byok-provider-error-desktop.png` | `/recommend` | 1440×900 | Provider HTTP 500 alert with retry and continue |
| `byok-ambiguity-desktop.png` | `/recommend` | 1440×900 | Untraceable budget marked `! Needs confirmation` |
| `byok-no-key-no-solution-desktop.png` | `/recommend` | 1440×900 | No-key journey, budget no-solution |
| `byok-no-key-results-desktop.png` | `/recommend` | 1440×900 | No-key journey, three compatible options |
| `byok-no-key-form-mobile.png` | `/recommend` | 390×844 | Assistant panel stacked, no overflow |
| `byok-ambiguity-mobile.png` | `/recommend` | 390×844 | Proposal cards and ambiguity block stacked |
| `byok-no-key-results-mobile.png` | `/recommend` | 390×844 | Option cards stacked, no overflow |
| `byok-results-mobile.png` | `/recommend` | 390×844 | Results after the no-key journey |

Details, commands, keyboard results, measured contrast and privacy review: `docs/evidence/inventory-import-browser.md`, `docs/evidence/compatibility-browser.md`, `docs/evidence/recommendation-browser.md` and `docs/evidence/byok-planning-browser.md`. Measured token contrast: `docs/evidence/token-contrast.json`. Lighthouse reports: `docs/evidence/inventory-lighthouse*/`, `docs/evidence/compatibility-lighthouse*/`, `docs/evidence/recommendation-lighthouse*/` and `docs/evidence/byok-lighthouse*/`.

### Use-case integrity captures (2026-11-01)

| Capture | Route | Viewport | Condition |
| --- | --- | --- | --- |
| `use-case-gaming-gpu-desktop.png` | `/recommend` | 1440×900 | Gaming 1,800.00: one GPU-inclusive build, rule stated, RTX 4070 present |
| `use-case-gpu-bound-desktop.png` | `/recommend` | 1440×900 | Gaming 1,500.00: budget binding names the card, 1,672.90 minimum and 172.90 shortfall |
| `use-case-no-padding-desktop.png` | `/recommend` | 1440×900 | Gaming 1,900.00: three valid storage variants collapse to one shown option |
| `use-case-gaming-gpu-mobile.png` | `/recommend` | 390×844 | GPU-inclusive result stacked, no overflow |
| `use-case-gpu-bound-mobile.png` | `/recommend` | 390×844 | Named GPU no-solution stacked, no overflow |
| `use-case-no-padding-mobile.png` | `/recommend` | 390×844 | One-option statement stacked, no overflow |

Details: `docs/evidence/use-case-diversity-browser.md`. Lighthouse reports: `docs/evidence/use-case-lighthouse/` and `docs/evidence/use-case-lighthouse-mobile/`.

## Refresh rule

Refresh these captures after any change to `/inventory`, `/builds/check`, `/recommend`, the shared tokens, the import, check, recommendation or AI plan/revision API, the compatibility rules, the scoring version, the interpretation contract or the rendered states. A capture that names an older revision is historical evidence, not current evidence.

### Catalog seed captures (2026-10-31)

| Capture | Route | Viewport | Condition |
| --- | --- | --- | --- |
| `catalog-import-empty-desktop.png` | `/inventory` | 1440×900 | Empty state with the sample licence, check date and operator-price note |
| `catalog-import-review-desktop.png` | `/inventory` | 1440×900 | Shipped sample preview, 10 valid rows, catalog facts per row |
| `catalog-import-provenance-desktop.png` | `/inventory` | 1440×900 | Committed revision 1; CPU provenance expanded with catalog and operator facts |
| `catalog-import-provenance-mobile.png` | `/inventory` | 390×844 | The same revision as stacked cards |
| `catalog-sample-check-desktop.png` | `/builds/check` | 1440×900 | Sample build with 12 compatible, 0 conflicts, 2 unknown |

### Data portability captures (2026-11-01)

| Capture | Route | Viewport | Condition |
| --- | --- | --- | --- |
| `data-portability-export-desktop.png` | `/inventory` | 1440×900 | Export panel with a committed snapshot, CSV and JSON buttons enabled |
| `data-portability-export-mobile.png` | `/inventory` | 390×844 | The same panel stacked, no overflow |
| `data-portability-inspected-desktop.png` | `/inventory` | 1440×900 | Backup inspected: integrity ok, schema 4, counts, replace button disabled |
| `data-portability-confirmed-desktop.png` | `/inventory` | 1440×900 | Typed RESTORE, replace enabled |
| `data-portability-restored-desktop.png` | `/inventory` | 1440×900 | Restored summary and the pre-restore safety-copy path |
| `data-portability-rejected-desktop.png` | `/inventory` | 1440×900 | Non-SQLite upload rejected with `backup_not_sqlite`, inventory still visible |
| `data-portability-rejected-mobile.png` | `/inventory` | 390×844 | The rejected state stacked, no overflow |

Details: `docs/evidence/data-portability-browser.md`. Lighthouse reports: `docs/evidence/data-portability-lighthouse/` and `docs/evidence/data-portability-lighthouse-mobile/`.

| `catalog-ingestion-upload-desktop.png` | `/catalog` | 1440×900 | Synthetic catalog uploaded: accepted report, provenance form and registry with licence |
| `catalog-ingestion-refused-desktop.png` | `/catalog` | 1440×900 | Refused document: row index, field and reason; no dataset added |
| `catalog-ingestion-search-desktop.png` | `/catalog` | 1440×900 | Ranked search results and an opened stored part with its source URL |
| `catalog-ingestion-enriched-desktop.png` | `/inventory` | 1440×900 | Import row matched to the catalog, proposals listed, opt-in checked |
| `catalog-ingestion-provenance-desktop.png` | `/inventory` | 1440×900 | Committed snapshot with catalog-sourced specifications and `catalog:<name>@<version>` provenance |
| `catalog-ingestion-mobile.png` | `/catalog` | 390×844 | Stacked registry and search with no horizontal overflow |

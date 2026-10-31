# Recommend and compare inventory builds

## ADDED Requirements

### Requirement: Recommendation uses only complete, in-stock inventory builds
The system SHALL build candidates exclusively from the records of one immutable inventory snapshot. Every candidate SHALL be a complete build: CPU, motherboard, memory, power supply, case and startup storage, plus a graphics card unless the CPU has sourced `igpu=yes` evidence, plus a cooler unless the CPU has sourced `bundled_cooler=yes` evidence. Each selected part's quantity SHALL NOT exceed the stock in that snapshot. A SKU excluded by the request SHALL never appear in a candidate. A locked SKU SHALL be included in every candidate as an owned part at zero incremental price and with zero stock consumption, and SHALL still exist in the snapshot. A missing sale price, zero available quantity, mixed currency or ambiguous duplicate lot SHALL remove that part from consideration and be reported rather than guessed.

#### Scenario: Over-stock and excluded parts are never selected
Given a snapshot where one graphics card has two units and a request that excludes another card, when recommendations are generated, then no candidate requests more than two units of that card and no candidate contains the excluded SKU.

#### Scenario: Owned parts cost nothing and take no stock
Given a request that locks a CPU the operator already owns, when recommendations are generated, then every candidate contains that CPU with `owned` true, a zero line total, and a stock line recording zero units used.

#### Scenario: Missing evidence is not invented
Given a snapshot with no sale price, a zero-quantity part or duplicate lots with conflicting facts, when recommendations are generated, then that part is absent from every candidate and an exclusion states the reason.

### Requirement: Search is deterministic and bounded
The system SHALL evaluate candidates with a deterministic depth-first search in a fixed category order, pruning partial builds that already exceed the budget. The search SHALL stop after a stated work limit of 100,000 expanded nodes and SHALL report the limit, nodes examined, complete combinations evaluated, valid builds and whether the limit was reached. When the limit is reached the report SHALL use `search_limited`, SHALL state that the result is best effort, and SHALL NOT claim a global optimum. An identical request against an unchanged snapshot revision, rule-set version and scoring version SHALL produce the same candidate order, part sets, totals and scores; only persisted identifiers and timestamps may differ.

#### Scenario: Repeated identical requests match
Given the same snapshot and request, when the recommendation is requested twice, then the ranked objectives, part SKUs and quantities, totals and scores are identical while each report has its own identifier.

#### Scenario: The work limit is enforced and disclosed
Given an inventory whose combination space exceeds the work limit, when recommendations are generated, then the response reports `search_limited`, `search.limited` is true, the node count has reached 100,000, and no optimality claim is made.

### Requirement: Results are ranked, explainable and at most three
The system SHALL present no more than three candidates with distinct part sets. Hard constraints SHALL be applied before scoring. Scoring SHALL use published fixed weights per use case (`gaming`, `office`, `creation`) over the budget-headroom, sourced performance-proxy and upgrade-room dimensions. A dimension whose evidence is absent SHALL be marked unavailable with a `null` value and zero effective weight; it SHALL NOT be scored as zero, and score confidence SHALL become `partial`. Each candidate SHALL state its total price, budget delta, stock use, completeness, overall compatibility verdict, the check report it passed, why it was selected (cost, stock and rule evidence) and which same-category alternatives were not selected. Incompatible or incomplete builds SHALL NOT be candidates; an unknown verdict SHALL be shown as unknown and distinguished from a verified result.

#### Scenario: Three distinct objectives are explained
Given a feasible inventory, when recommendations are generated, then the options are labelled by objective (for example best overall, lowest cost, performance first), each has a different SKU set, and each lists its reasons and unselected alternatives.

#### Scenario: Missing performance evidence lowers confidence instead of scoring zero
Given no confirmed `perf_score` on the CPU or graphics card, when recommendations are generated, then the performance dimension is unavailable, its value is `null`, its weight is zero, the score confidence is `partial`, and the reason names the missing evidence.

### Requirement: No solution explains the binding constraint
When no candidate can satisfy the hard constraints, the system SHALL return zero candidates and a `noSolution` object whose `binding` is `budget`, `stock`, `compatibility` or `search_limit`, with a readable explanation, concrete blockers and the constraints the operator can adjust. For a budget constraint the report SHALL name the cheapest structurally complete cost and the shortfall. For a stock constraint it SHALL name each unusable required category with the reason (not recorded, zero quantity, excluded, over budget or ambiguous). For a compatibility constraint it SHALL count the rejected complete combinations by conflicting rule or missing category. The system SHALL NOT output a build that violates a hard constraint.

#### Scenario: Budget below the cheapest build
Given required parts whose cheapest combination costs 2,450.00 and a 1,200.00 budget, when recommendations are generated, then there are no candidates, the binding is `budget`, and the explanation names the 1,250.00 shortfall and suggests raising the budget.

#### Scenario: Missing required stock
Given a snapshot with no storage records, when recommendations are generated, then there are no candidates, the binding is `stock`, and the blocker names storage as unrecorded.

### Requirement: Recommendation API is validated and revision-bound
`POST /api/recommendations` SHALL accept `snapshotId`, `budgetMinor`, `currency`, `useCase`, `inventoryOnly`, `lockedSkus` and `excludedSkus`. It SHALL return HTTP 400 with `snapshot_required`, `budget_required`, `budget_invalid`, `currency_required`, `currency_format`, `currency_mismatch`, `use_case_required`, `use_case_unsupported`, `catalog_unavailable`, `duplicate_sku`, `locked_sku_invalid`, `excluded_sku_invalid`, `locked_excluded_conflict` or `locked_category_conflict` for malformed requests; HTTP 404 with `snapshot_not_found`; and HTTP 422 with `locked_sku_not_found`, `excluded_sku_not_found` or `locked_sku_out_of_stock`. A successful response SHALL be persisted in SQLite with the snapshot ID and revision, rule-set version `compat-rules-v1`, scoring version `recommend-score-v1`, the request echo and every candidate's check report. `GET /api/recommendations/{id}` SHALL return the stored report and SHALL mark it `stale` with a reason when the snapshot chain has advanced or the snapshot no longer exists. The API SHALL NOT mutate a snapshot, reserve stock or persist a partial report.

#### Scenario: Malformed request has a stable field code
Given an absent budget or a currency different from the snapshot currency, when the recommendation is requested, then the service returns HTTP 400 with `budget_required` or `currency_mismatch` and a field name.

#### Scenario: A correction makes the stored report stale
Given a persisted recommendation for snapshot revision 1, when a correction creates revision 2 and the report is read, then `stale` is true, the current revision is 2, and the reason names both revisions.

### Requirement: Export round-trips the shown report with its versions
The system SHALL export a stored recommendation as versioned machine-readable JSON and as a human-readable Markdown plan. The JSON export SHALL contain `exportVersion` `recommend-export-v1`, the export time, the snapshot id, revision, currency, source filename and checksum, the exact stored report object, and one entry per candidate naming its check report id, rule-set version, overall status, summary and unknown follow-ups. The Markdown plan SHALL repeat the same options, prices, stock, compatibility verdicts, reasons, alternatives, unknown follow-ups, evidence source references and a manual-confirmation notice. Export SHALL NOT recompute prices or verdicts. An unsupported `format` query value SHALL return HTTP 400 with `export_format_unsupported`, and an unknown report SHALL return HTTP 404 with `recommendation_not_found`.

#### Scenario: Export matches the stored report
Given a generated recommendation, when the JSON export is downloaded, then its `report` object equals the stored report and each candidate's `checkReportId` matches the embedded check report.

#### Scenario: Human-readable export carries the same facts
Given a generated recommendation, when the Markdown export is downloaded, then it names the options, budget use, compatibility verdicts, check report identifiers, evidence sources and the manual-confirmation notice.

### Requirement: The recommendation Web flow is keyboard and screen-reader usable
The `/recommend` route SHALL let the operator choose a snapshot, enter a budget, choose a use case, lock or exclude parts, generate recommendations, compare options, read why each was chosen, read the binding constraint when there is no solution, and download either export. It SHALL cover loading, empty inventory, empty or invalid budget, generating, results, unknown warnings, search-limited, no solution, stale-after-edit, export success and export error states. It SHALL preserve the request on failure and offer retry; announce asynchronous start, completion, no-solution and export results in a live region; move focus to the result or no-solution heading and to the error alert; use text and a symbol rather than color alone for status; keep all actions reachable by keyboard with a visible focus ring; and stack the layout at phone width without horizontal page overflow.

#### Scenario: Keyboard-only generation and export
Given the recommendation page at phone width, when the operator moves with Tab and Enter only, then the budget, use case, lock and exclude controls, generate action, comparison, rule evidence and export actions are reachable with visible focus and no pointer gesture.

#### Scenario: No solution is readable at phone width
Given a no-solution report at 390 px width, when it is rendered, then the binding constraint, blockers and adjustable constraints are visible, stacked and announced, and the page has no horizontal overflow.

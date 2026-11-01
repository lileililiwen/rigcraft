# build-recommendation Specification

## Purpose
Define the deterministic, inventory-only recommendation contract: complete and compatible candidates from one immutable snapshot, a bounded search, published scoring with explicit missing-evidence handling, up to three explained and comparable options, a specific no-solution explanation for the binding constraint, revision-bound persistence, versioned JSON and Markdown export, and the accessible `/recommend` Web flow.
## Requirements
### Requirement: Recommendation uses only complete, in-stock inventory builds
The system SHALL build candidates exclusively from the records of one immutable inventory snapshot. Every candidate SHALL be a complete build: CPU, motherboard, memory, power supply, case and startup storage, plus a graphics card unless the use case and the CPU's sourced evidence allow otherwise, plus a cooler unless the CPU has sourced `bundled_cooler=yes` evidence. `gaming` SHALL require a discrete graphics card in every candidate; a CPU's sourced `igpu=yes` SHALL NOT complete a gaming build. `office` SHALL accept a CPU with sourced `igpu=yes` when no discrete card is selected. `creation` SHALL keep the optional-graphics behaviour and SHALL state that a discrete graphics card was not required. Each selected part's quantity SHALL NOT exceed the stock in that snapshot. A SKU excluded by the request SHALL never appear in a candidate. A locked SKU SHALL be included in every candidate as an owned part at zero incremental price and with zero stock consumption, and SHALL still exist in the snapshot. A missing sale price, zero available quantity, mixed currency or ambiguous duplicate lot SHALL remove that part from consideration and be reported rather than guessed. A missing graphics card for `gaming` SHALL be a hard constraint, never a candidate that omits it.

#### Scenario: Gaming excludes an integrated-graphics-only build
Given a snapshot whose only CPU publishes `igpu=yes` and whose graphics card is absent or excluded, when a `gaming` recommendation is generated, then there are no candidates and the graphics card is named as the binding constraint.

#### Scenario: Office accepts sourced integrated graphics
Given the same snapshot and an `office` request, when the recommendation is generated, then a complete candidate without a discrete graphics card is returned and its reasons state that the sourced `igpu=yes` fact is accepted for office.

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
The system SHALL present no more than three candidates that are materially different. A candidate's structural signature SHALL be the chosen parts (category, SKU and quantity), and a difference that is only storage capacity SHALL NOT be material: candidates whose signatures differ only by storage SHALL collapse to one representative. Each presented candidate SHALL carry a distinct objective (best overall, lowest cost, highest performance or alternative) and a distinct signature. Hard constraints SHALL be applied before scoring. Scoring SHALL use published fixed weights per use case (`gaming`, `office`, `creation`) over the budget-headroom, sourced performance-proxy and upgrade-room dimensions. A dimension whose evidence is absent SHALL be marked unavailable with a `null` value and zero effective weight; it SHALL NOT be scored as zero, and score confidence SHALL become `partial`. Each candidate SHALL state its total price, budget delta, stock use, completeness, overall compatibility verdict, the check report it passed, why it was selected (cost, stock and rule evidence), how the use-case completeness rule applied, and which same-category alternatives were not selected. When only one or two materially different builds exist, the report SHALL return exactly that many and SHALL NOT invent or pad a third. Incompatible or incomplete builds SHALL NOT be candidates; an unknown verdict SHALL be shown as unknown and distinguished from a verified result.

#### Scenario: Three distinct objectives are explained
Given a feasible inventory with at least three material signatures, when recommendations are generated, then three candidates are returned, each labelled by a distinct objective, each with a different signature, and each listing its reasons and unselected alternatives.

#### Scenario: Storage-only variants collapse
Given an inventory where every complete build uses the same parts except for three storage capacities, when recommendations are generated, then exactly one candidate is returned with the cheapest representative and the report does not present the drives as separate options.

#### Scenario: One distinct build is not padded
Given an inventory with exactly one material signature inside the budget, when recommendations are generated, then exactly one candidate is returned and the report and Web flow state that no option was padded to reach three.

#### Scenario: Missing performance evidence lowers confidence instead of scoring zero
Given no confirmed `perf_score` on the CPU or graphics card, when recommendations are generated, then the performance dimension is unavailable, its value is `null`, its weight is zero, the score confidence is `partial`, and the reason names the missing evidence.

### Requirement: No solution explains the binding constraint
When no candidate can satisfy the hard constraints, the system SHALL return zero candidates and a `noSolution` object whose `binding` is `budget`, `stock`, `compatibility` or `search_limit`, with a readable explanation, concrete blockers and the constraints the operator can adjust. For a budget constraint the report SHALL name the cheapest structurally complete cost and the shortfall. For a stock constraint it SHALL name each unusable required category with the reason (not recorded, zero quantity, excluded, over budget or ambiguous). For a compatibility constraint it SHALL count the rejected complete combinations by conflicting rule or missing category. When the request is `gaming`, every no-solution explanation SHALL name the graphics card requirement: the headline, explanation or a blocker SHALL say that gaming requires a discrete graphics card, and a budget or stock constraint SHALL state the cheapest GPU-inclusive total and the shortfall when they are known. The system SHALL NOT output a build that violates a hard constraint, and SHALL NOT answer a `gaming` request with an integrated-graphics-only build.

#### Scenario: Budget below the cheapest build
Given required parts whose cheapest combination costs 2,450.00 and a 1,200.00 budget, when recommendations are generated, then there are no candidates, the binding is `budget`, and the explanation names the 1,250.00 shortfall and suggests raising the budget.

#### Scenario: Budget below the cheapest GPU-inclusive build
Given a snapshot with an `igpu=yes` CPU and a discrete card whose cheapest complete build costs 1,330.00, and a 1,000.00 budget, when a `gaming` recommendation is generated, then there are no candidates, the binding is `budget`, the graphics card and the 1,330.00 minimum are named, and the 330.00 shortfall is stated.

#### Scenario: Gaming has no graphics card in stock
Given a snapshot with an `igpu=yes` CPU and no graphics card, when a `gaming` recommendation is generated, then there are no candidates, the binding is `stock`, and the graphics-card blocker states that gaming requires a discrete card.

#### Scenario: Missing required stock
Given a snapshot with no storage records, when recommendations are generated, then there are no candidates, the binding is `stock`, and the blocker names storage as unrecorded.

### Requirement: Recommendation API is validated and revision-bound
`POST /api/recommendations` SHALL accept `snapshotId`, `budgetMinor`, `currency`, `useCase`, `inventoryOnly`, `lockedSkus` and `excludedSkus`. It SHALL return HTTP 400 with `snapshot_required`, `budget_required`, `budget_invalid`, `currency_required`, `currency_format`, `currency_mismatch`, `use_case_required`, `use_case_unsupported`, `catalog_unavailable`, `duplicate_sku`, `locked_sku_invalid`, `excluded_sku_invalid`, `locked_excluded_conflict` or `locked_category_conflict` for malformed requests; HTTP 404 with `snapshot_not_found`; and HTTP 422 with `locked_sku_not_found`, `excluded_sku_not_found` or `locked_sku_out_of_stock`. A successful response SHALL be persisted in SQLite with the snapshot ID and revision, rule-set version `compat-rules-v1`, scoring version `recommend-score-v1`, the request echo and every candidate's check report. `GET /api/recommendations/{id}` SHALL return the stored report and SHALL mark it `stale` with a reason when the snapshot chain has advanced or the snapshot no longer exists. The API SHALL NOT mutate a snapshot, reserve stock or persist a partial report.

#### Scenario: Malformed request has a stable field code
Given an absent budget or a currency different from the snapshot currency, when the recommendation is requested, then the service returns HTTP 400 with `budget_required` or `currency_mismatch` and a field name.

#### Scenario: A correction makes the stored report stale
Given a persisted recommendation for snapshot revision 1, when a correction creates revision 2 and the report is read, then `stale` is true, the current revision is 2, and the reason names both revisions.

### Requirement: Export round-trips the shown report with its versions
The system SHALL export a stored recommendation as versioned machine-readable JSON and as a human-readable Markdown plan. The JSON export SHALL contain `exportVersion` `recommend-export-v1`, the export time, the snapshot id, revision, currency, source filename and checksum, the exact stored report object, and one entry per candidate naming its check report id, rule-set version, overall status, summary and unknown follow-ups. The Markdown plan SHALL repeat the same options, prices, stock, compatibility verdicts, reasons, alternatives, unknown follow-ups, evidence source references and a manual-confirmation notice. Every dynamic Markdown table cell SHALL escape backslash as `\\` and pipe as `\|`, replace line breaks with a space, and neutralise `<` and `>` as `&lt;` and `&gt;`, so operator text can neither add a column nor inject HTML. The JSON export SHALL remain exact data and SHALL NOT apply these escapes. Export SHALL NOT recompute prices or verdicts. An unsupported `format` query value SHALL return HTTP 400 with `export_format_unsupported`, and an unknown report SHALL return HTTP 404 with `recommendation_not_found`.

#### Scenario: Export matches the stored report
Given a generated recommendation, when the JSON export is downloaded, then its `report` object equals the stored report and each candidate's `checkReportId` matches the embedded check report.

#### Scenario: Human-readable export carries the same facts
Given a generated recommendation, when the Markdown export is downloaded, then it names the options, budget use, compatibility verdicts, check report identifiers, evidence sources and the manual-confirmation notice.

#### Scenario: Operator text cannot break the Markdown table
Given a part model containing a pipe, a backslash and an HTML tag, when the Markdown export is generated, then the pipe and backslash are escaped, the tag is rendered as text, and every row of the parts table still has the same declared column count.

### Requirement: The recommendation Web flow is keyboard and screen-reader usable
The `/recommend` route SHALL let the operator choose a snapshot, enter a budget, choose a use case, lock or exclude parts, generate recommendations, compare options, read why each was chosen, read the binding constraint when there is no solution, and download either export. It SHALL cover loading, empty inventory, empty or invalid budget, generating, results, unknown warnings, search-limited, no solution, stale-after-edit, export success and export error states. It SHALL state the use-case completeness rule that applied, including that gaming required a discrete graphics card, and SHALL state the real number of materially different options so a report with one or two options never implies three. It SHALL preserve the request on failure and offer retry; announce asynchronous start, completion, no-solution and export results in a live region; move focus to the result or no-solution heading and to the error alert; use text and a symbol rather than color alone for status; keep all actions reachable by keyboard with a visible focus ring; and stack the layout at phone width without horizontal page overflow.

#### Scenario: Keyboard-only generation and export
Given the recommendation page at phone width, when the operator moves with Tab and Enter only, then the budget, use case, lock and exclude controls, generate action, comparison, rule evidence and export actions are reachable with visible focus and no pointer gesture.

#### Scenario: The use-case rule is visible
Given a `gaming` report, when the results render, then the page states that gaming requires a discrete graphics card, and every candidate includes one.

#### Scenario: Fewer than three options is stated
Given a report with one materially different build, when the results render, then the page shows one option and says it was not padded to three.

#### Scenario: No solution is readable at phone width
Given a no-solution report at 390 px width, when it is rendered, then the binding constraint, blockers and adjustable constraints are visible, stacked and announced, and the page has no horizontal overflow.

### Requirement: Budget input accepts an unambiguous grouped amount
The budget field SHALL accept a comma or a single space as a thousands separator
only when the layout is unambiguous: the first group has one to three digits and
every following group has exactly three digits, optionally followed by a decimal
point and one or two fraction digits. `1,800` and `1 800` SHALL parse to 180000
minor units and `1,800.50` to 180050. Any other shape — including `1.800,50`,
`1,80`, `1,8000` and `1.234` — SHALL be refused and SHALL keep the existing
`budget_invalid` guidance rather than being guessed.

#### Scenario: A grouped amount is accepted
Given the budget field, when the operator enters `1,800`, then the request carries `budgetMinor` 180000.

#### Scenario: A space-grouped amount is accepted
Given the budget field, when the operator enters `1 800.50`, then the request carries `budgetMinor` 180050.

#### Scenario: An ambiguous amount is refused
Given the budget field, when the operator enters `1.800,50` or `1,80`, then no request is sent and the `budget_invalid` guidance is shown.

### Requirement: Generate requires a parsable budget
The `Generate recommendations` control SHALL be disabled while the budget field is
empty or does not parse, and SHALL be enabled once it parses to a positive amount.
A persistent explanation associated with the control SHALL state that requirement,
and the invalid state SHALL be exposed through `aria-invalid` and the associated
guidance rather than only through colour.

#### Scenario: Empty field keeps the action unavailable
Given a loaded snapshot and an empty budget field, when the page renders, then `Generate recommendations` is disabled and its accessible description explains why.

#### Scenario: Invalid field keeps the action unavailable
Given the budget field containing `1.234`, when the page renders, then `Generate recommendations` is disabled and the budget field is marked invalid.

#### Scenario: A parsable field enables generation
Given the budget field containing `1,800`, when the page renders, then `Generate recommendations` is enabled and one activation sends exactly one recommendation request.

### Requirement: Counts and money use one agreed form
The page SHALL render every count it composes through one shared pluralisation
helper that yields the singular form for one and the plural form otherwise, and no
page-composed string SHALL contain an `(s)` pseudo-plural. Money SHALL be rendered
in one fixed form — `US$1,672.90` — that does not depend on the operator's browser
locale. Engine-authored sentences SHALL be rendered verbatim, including the amount
and wording the export carries.

#### Scenario: Plural forms follow the count
Given the pluralisation helper, when it is called with 1 unit and with 2 units, then it returns `1 unit` and `2 units`.

#### Scenario: Money is fixed
Given a report in USD, when the page renders a total of 167290 minor units, then it shows `US$1,672.90` regardless of the browser locale.

#### Scenario: Engine prose is kept
Given a report whose reason states the total in the engine's money form, when the option renders, then that sentence is present unchanged.

### Requirement: Report and export prose uses grammatical plural forms
Every sentence the recommendation engine composes for a candidate reason, a
no-solution explanation or the Markdown export SHALL spell a count with the
singular noun only for exactly one and the plural noun otherwise, so zero reads
`0 units` and two reads `2 units`. The sentence SHALL be carried unchanged in
the API response and in both exports; the JSON reason strings and the Markdown
line SHALL hold the same grammatical sentence the API returned. Identifiers,
error codes, JSON field names, the report structure and the Markdown layout
SHALL NOT change. No recommendation-composed sentence SHALL contain the `(s)`
pseudo-plural. A sentence the browser suite already asserts, including
`1 materially different build found; no option was padded to reach three.`,
SHALL remain unchanged.

#### Scenario: Zero, one and many units read correctly
Given a recommendation whose reason reports the units taken from snapshot stock, when the count is zero, one and two, then the reason reads `0 units`, `1 unit` and `2 units`.

#### Scenario: The no-solution counts are grammatical
Given a compatibility no-solution over two complete combinations with one rejection, when it renders, then the explanation contains `2 complete combinations` and `1 rejection`.

#### Scenario: Markdown and JSON carry the grammatical sentence
Given a generated recommendation, when the Markdown and JSON exports are read, then the Markdown line and the JSON reason string each contain the grammatical count sentence the API returned.


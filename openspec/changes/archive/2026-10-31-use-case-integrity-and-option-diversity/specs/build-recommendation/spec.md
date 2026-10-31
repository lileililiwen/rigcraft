# build-recommendation deltas

## MODIFIED Requirements

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

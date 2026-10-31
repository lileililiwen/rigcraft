# Build diagnosis correctness deltas

## MODIFIED Requirements

### Requirement: Reuse advice separates must replace, should upgrade and keep
For every described part the report SHALL state one action — `must_replace`, `should_upgrade`, `keep`, or `unverified` — with the rule evidence behind it and its cost implication. `must_replace` SHALL be assigned to a part that participates in a definite conflict. `should_upgrade` SHALL be assigned, with `conditional: true`, to a part that participates only in rules whose required fact is missing, and its reason SHALL name the missing fact and state that replacement is justified only if confirmation shows the part is inadequate. `keep` SHALL be assigned to a part for which at least one rule passed and no rule conflicted or stayed unknown, or to a part whose recorded conflict is resolved by the plan replacing another part; a part whose rules all passed SHALL NOT be reclassified. `unverified` SHALL be assigned to a part that participates in zero evaluated rules, with a reason stating that no rule that could run involves it and that its compatibility is therefore not verified. The report SHALL group the advice in exactly those four sections, SHALL cite the rule identifiers and their evidence, and SHALL report the cost of keeping as zero and the cost of the cheapest non-conflicting replacement from the snapshot, or state that the snapshot holds no replacement. Retained owned parts SHALL NOT be charged again, and no trade-in or resale value SHALL be assumed.

#### Scenario: Each defect lands in the right group
Given a build with one socket conflict, one missing BIOS fact, one part whose rules all pass and one part involved in no evaluated rule, when it is diagnosed, then the conflicting part is `must_replace`, the missing-fact part is `should_upgrade` with the BIOS question as its reason and `conditional: true`, the passing part is `keep`, the unevaluated part is `unverified`, and the four groups are reported separately.

#### Scenario: An unevaluated part is never called fine
Given a described build containing only a CPU, a graphics card and memory, when it is diagnosed, then each part is in the `unverified` group, the summary says zero rules were evaluated, and no part carries a verified or fine-as-is label.

#### Scenario: Cost does not double-count kept parts
Given an existing build whose parts are all owned, when replacement costs are reported, then every kept part costs zero and the plan cost is the sum of the replacement prices only.

### Requirement: The diagnosis Web flow is complete, keyboard usable and announced
The `/diagnose` route SHALL let the operator choose a snapshot and currency, paste a list, review every line's match status and resolve each ambiguity by choosing a candidate SKU or discarding the line, add or edit parts manually, run the diagnosis, read the four advice groups with their evidence and cost, and read the upgrade plan with its validation state. It SHALL render a stable numbered skeleton without gaps: sections for snapshot, paste, match review, manual entry and result SHALL keep their numbers whether or not a list has been matched. The match-review section SHALL have a real empty state before a list is matched and SHALL receive focus only when a list exists. The page SHALL cover the empty, paste/manual entry, match review, diagnosing, results, ambiguity, no-replacement and error states, SHALL preserve entered data when a request fails, and SHALL offer a retry. It SHALL use labelled controls, a live status region, a `role="alert"` failure, focus on the match-review and result headings, symbol-plus-text status (never colour alone), visible focus, stacked cards at phone width without horizontal page overflow, and reduced-motion-safe transitions. It SHALL state that matching and diagnosis consult only the local snapshot and that no catalog or AI provider is used.

#### Scenario: Ambiguous matches are resolved by keyboard only
Given a pasted list with two ambiguous lines at phone width, when the operator uses Tab, the arrow keys and Enter only, then each candidate radio group is reachable with visible focus, a choice is announced, the resolved SKU reaches the diagnosis request, and the page has no horizontal overflow.

#### Scenario: The plan states whether the checker validated it
Given a diagnosis with a plan, when the result is displayed, then the plan shows `Validated`, `Needs evidence` or `No validated plan` as text beside its symbol, lists each step and its cost, and never shows a rejected plan as valid.

#### Scenario: Numbering does not skip before a list is matched
Given a loaded snapshot and no matched list, when the page first renders, then the visible sections are numbered 1, 2, 3, 4 and 5 in order, and section 3 shows an empty state rather than disappearing.

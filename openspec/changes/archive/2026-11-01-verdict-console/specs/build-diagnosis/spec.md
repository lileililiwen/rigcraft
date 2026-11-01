# build-diagnosis deltas

## MODIFIED Requirements

### Requirement: The diagnosis Web flow is complete, keyboard usable and announced
The `/diagnose` route SHALL let the operator choose a snapshot and currency, paste a list, review every line's match status and resolve each ambiguity by choosing a candidate SKU or discarding the line, add or edit parts manually, run the diagnosis, read the four advice groups with their evidence and cost, and read the upgrade plan with its validation state. It SHALL render a stable numbered skeleton without gaps: sections for snapshot, paste, match review, manual entry and result SHALL keep their numbers whether or not a list has been matched. The match-review section SHALL have a real empty state before a list is matched and SHALL receive focus only when a list exists. The page SHALL cover the empty, paste/manual entry, match review, diagnosing, results, ambiguity, no-replacement and error states, SHALL preserve entered data when a request fails, and SHALL offer a retry. It SHALL use labelled controls, a live status region, a `role="alert"` failure, focus on the match-review and result headings, symbol-plus-text status (never colour alone), visible focus, stacked cards at phone width without horizontal page overflow, and reduced-motion-safe transitions. It SHALL state that matching and diagnosis consult only the local snapshot and that no catalog or AI provider is used. The result SHALL state its evaluated-part coverage exactly once: when any described part was not evaluated it names how many of how many were checked and how many remain unverified; when every part was evaluated it states that fact once and SHALL NOT also print the checked-part fraction. A candidate whose token set exactly equals the pasted line's SHALL NOT display added or missing token lists; a non-exact candidate SHALL keep them.

#### Scenario: Ambiguous matches are resolved by keyboard only
Given a pasted list with two ambiguous lines at phone width, when the operator uses Tab, the arrow keys and Enter only, then each candidate radio group is reachable with visible focus, a choice is announced, the resolved SKU reaches the diagnosis request, and the page has no horizontal overflow.

#### Scenario: The plan states whether the checker validated it
Given a diagnosis with a plan, when the result is displayed, then the plan shows `Validated`, `Needs evidence` or `No validated plan` as text beside its symbol, lists each step and its cost, and never shows a rejected plan as valid.

#### Scenario: Numbering does not skip before a list is matched
Given a loaded snapshot and no matched list, when the page first renders, then the visible sections are numbered 1, 2, 3, 4 and 5 in order, and section 3 shows an empty state rather than disappearing.

#### Scenario: Coverage is stated once
Given a described build whose every part participates in at least one evaluated rule, when the result renders, then the page states that every described part was checked by at least one rule exactly once and prints no `N of N` checked-part sentence.

#### Scenario: Unverified parts are counted, not repeated
Given a described build with parts that participate in no evaluated rule, when the result renders, then the page states how many of how many described parts were checked and names how many remain unverified, without also printing the all-checked sentence.

#### Scenario: Exact matches hide empty token lists
Given an exact matched candidate and an ambiguous candidate in the same match review, when the page renders, then the exact candidate shows no added or missing token list and the ambiguous candidate still shows its added and missing tokens.

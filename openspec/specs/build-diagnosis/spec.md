# build-diagnosis Specification

## Purpose
Define the diagnosis of a build the operator already owns: a bounded pasted list or hand-entered parts matched honestly against one snapshot, a three-state verdict from the same compatibility engine and rule set as `/builds/check`, reuse advice grouped into must replace, should upgrade and keep with cited evidence and cost, and an upgrade plan validated as one whole build before it is presented. Ambiguous or unmatched input is never coerced to a nearby SKU, a missing fact is never inferred, and a plan the checker rejects is never shown as valid.
## Requirements
### Requirement: An existing build is described from owned parts or a bounded external list
The system SHALL diagnose a build the operator already has, described either by manually entered owned parts or by a pasted external list. A submitted build SHALL contain between 1 and 40 parts. Each part SHALL carry `source` (`owned` or `snapshot`), a category from the supported set, a model of 1–200 characters, a positive whole quantity, optional `specs` in `key=value` form, and an optional reported price. A `snapshot` part SHALL name an existing SKU in the selected snapshot and SHALL take its category, model, specifications and provenance from that record; a SKU absent from the snapshot SHALL fail with HTTP 422 `sku_not_found`. An `owned` part SHALL be validated with the inventory rules, and its specifications SHALL be parsed with the same parser, so an unrecognised specification key stays `unknown` rather than being defaulted.

Pasted-list input to `POST /api/diagnoses/parse` SHALL be limited to 8 KiB and 40 non-empty lines, and SHALL return HTTP 400 `text_required` for empty input, `text_too_large` for an oversized text, `line_limit_exceeded` (with the limit in `details`) when the line limit is exceeded, `currency_required` or `currency_format` for a bad currency, `currency_mismatch` when the currency differs from the selected snapshot, and 404 `snapshot_not_found` for an unknown snapshot. Every line SHALL be reported with its own status and issues; no line is silently dropped. A duplicate line SHALL keep its own status and SHALL name the earlier line as `duplicateOf`. Parsing SHALL NOT persist anything, and a diagnosis SHALL NOT be persisted unless the whole request validates, so no partial build is ever stored.

#### Scenario: A 41-line list is refused before anything is stored
Given a pasted list of 41 parts, when it is parsed, then the service returns HTTP 400 `line_limit_exceeded` naming the 40-line limit, returns no candidates, writes no report and leaves the snapshot unchanged.

#### Scenario: An unknown snapshot cannot be diagnosed against
Given a diagnosis request naming a snapshot ID that does not exist, when it is submitted, then the service returns HTTP 404 `snapshot_not_found` and persists no diagnosis.

#### Scenario: A partly invalid build is rejected whole
Given a build whose fourth part has an unsupported category, when it is diagnosed, then the service returns HTTP 400 `part_invalid` with `row: 4`, and no diagnosis is stored.

### Requirement: Part matching is honest about ambiguity and misses
The system SHALL match a pasted line against the records of the selected snapshot only. There is no licensed hardware catalog in this package, so a manual entry path SHALL always remain available. Matching SHALL normalise a model into lowercase alphanumeric tokens, splitting letter and digit runs, and SHALL rank candidates by the share of the pasted tokens they contain. It SHALL return at most five candidates per line, with exact-token matches ordered before fuzzy matches, and SHALL mark a line `matched` only when exactly one candidate has exactly the same token set. A line with several equally exact candidates, or with only fuzzy candidates, SHALL be `ambiguous`; a line with no candidate above the disclosure threshold SHALL be `unmatched`. Every candidate SHALL state the tokens it shares, the tokens it adds beyond the input, and the record provenance used. Every ambiguous or unmatched line SHALL name what evidence would disambiguate or identify it, and SHALL offer no default selection. A category alone SHALL never produce a candidate, and no specification SHALL be inferred from a model name. A matched line is a suggestion: the diagnosis request SHALL name the SKU explicitly, so no match is auto-confirmed.

#### Scenario: Two variants are ambiguous, not nearest-matched
Given a pasted line `gpu: Fixture GeForce RTX 4070` and two snapshot records that both contain every pasted token but add different suffix tokens, when the line is parsed, then its status is `ambiguous`, both candidates are listed with their extra tokens, the disambiguation names those suffix tokens and the SKU as the evidence to supply, and no candidate is preselected.

#### Scenario: An unmatched line never falls back to a nearby SKU
Given a pasted line whose model shares no token above the disclosure threshold with any snapshot record, when it is parsed, then its status is `unmatched`, the candidate list is empty, and the advice is to add the category and full model or describe the part manually with its specifications.

#### Scenario: An exact single match is still confirmed by the request
Given a pasted line with exactly one snapshot record carrying the same token set, when the line is parsed, then its status is `matched` with that SKU, and no diagnosis report exists until a request names the SKU explicitly.

### Requirement: Diagnosis reuses the deterministic compatibility engine
`POST /api/diagnoses` SHALL build the described build from the confirmed parts and SHALL evaluate it with the same compatibility engine and rule set (`compat-rules-v1`) that `POST /api/builds/check` uses. Owned parts SHALL be included at zero incremental price and SHALL take no stock; they SHALL NOT be treated as purchasable inventory. The report SHALL embed that check report, its completeness and every unknown follow-up, and SHALL NOT claim compatibility that the engine did not return. A build with a definite conflict SHALL NOT be reported as reusable as-is. Every diagnosis SHALL record the snapshot ID, snapshot revision, rule-set version, match version and advice version, and re-reading it SHALL mark it stale when the snapshot's correction chain has advanced.

#### Scenario: A conflicting existing build is reported as incompatible
Given an owned AM4 CPU described against an AM5 board, when the build is diagnosed, then the embedded check returns `incompatible` with `cpu-socket`, the report does not call the build reusable as-is, and the CPU and board are named as the conflicting parts.

#### Scenario: Missing facts stay unknown
Given an owned part whose required specification was not supplied, when the build is diagnosed, then the affected rule is `unknown` with its specific follow-up question, and no specification is filled in from a similar model.

#### Scenario: A staler snapshot is visible on re-read
Given a diagnosis checked against revision 1 and a later correction that produces revision 2, when the diagnosis is read again, then `stale` is true and `staleReason` names both revisions.

### Requirement: Reuse advice separates must replace, should upgrade and keep
For every described part the report SHALL state one action — `must_replace`, `should_upgrade`, `keep`, or `unverified` — with the rule evidence behind it and its cost implication. `must_replace` SHALL be assigned to a part that participates in a definite conflict. `should_upgrade` SHALL be assigned, with `conditional: true`, to a part that participates only in rules whose required fact is missing, and its reason SHALL name the missing fact and state that replacement is justified only if confirmation shows the part is inadequate. `keep` SHALL be assigned to a part for which at least one rule passed and no rule conflicted or stayed unknown, or to a part whose recorded conflict is resolved by the plan replacing another part; a part whose rules all passed SHALL NOT be reclassified. `unverified` SHALL be assigned to a part that participates in zero evaluated rules, with a reason stating that no rule that could run involves it and that its compatibility is therefore not verified. The report SHALL group the advice in exactly those four sections, SHALL cite the rule identifiers and their evidence, and SHALL report the cost of keeping as zero and the cost of the cheapest non-conflicting replacement from the snapshot, or state that the snapshot holds no replacement. Retained owned parts SHALL NOT be charged again, and no trade-in or resale value SHALL be assumed.

#### Scenario: Each defect lands in the right group
Given a build with one socket conflict, one missing BIOS fact, one part whose rules all pass and one part involved in no evaluated rule, when it is diagnosed, then the conflicting part is `must_replace`, the missing-fact part is `should_upgrade` with the BIOS question as its reason and `conditional: true`, the passing part is `keep`, the unevaluated part is `unverified`, and the four groups are reported separately.

#### Scenario: An unevaluated part is never called fine
Given a described build containing only a CPU, a graphics card and memory, when it is diagnosed, then each part is in the `unverified` group, the summary says zero rules were evaluated, and no part carries a verified or fine-as-is label.

#### Scenario: Cost does not double-count kept parts
Given an existing build whose parts are all owned, when replacement costs are reported, then every kept part costs zero and the plan cost is the sum of the replacement prices only.

### Requirement: The upgrade plan is validated before it is presented
When at least one part must be replaced, the system SHALL construct an upgrade plan from snapshot records that the same compatibility engine validates as a whole. The report SHALL present each step with the part being replaced, the replacement, its price, the rule identifiers that motivated it and the validation result. A plan SHALL be labelled `validated` only when the composed build returns `compatible`. A plan that returns `unknown` SHALL be labelled `needs_evidence`, SHALL list the follow-ups, and SHALL NOT be called valid. A plan whose composed build returns `incompatible` SHALL NOT be presented as steps; the report SHALL state that no validated plan exists and SHALL name the rejected candidates and the rules that rejected them. When no replacement candidate exists, the report SHALL state that the snapshot has no option instead of inventing one. A plan SHALL NOT reserve stock, change a snapshot, or write a purchase order.

#### Scenario: A validated plan passes the checker as a whole
Given an AM4 CPU in an AM5 board and a compatible AM5 CPU in the snapshot, when the build is diagnosed, then the plan replaces the CPU, its composed check returns `compatible`, the plan is labelled `validated`, and the additional cost equals the replacement price.

#### Scenario: A plan the checker rejects is never shown as valid
Given a replacement candidate that conflicts with the retained parts, when the plan is composed, then its check returns `incompatible`, no steps are presented as valid, and the report names that candidate and the rejecting rule.

#### Scenario: A snapshot with no replacement says so
Given a conflicting part and no candidate of that category in the snapshot, when the build is diagnosed, then the plan reports no option for that category and invents no SKU or price.

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


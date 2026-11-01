# ui-foundation deltas

## ADDED Requirements

### Requirement: The compatibility check is presented as a rule console
The `/builds/check` result SHALL open with one summary bar carrying the overall
status, the count of rules passed, the count of conflicts, the count of unknown
rules and the count of selected parts that no evaluated rule covered. The summary
SHALL remain visible above the rule matrix while the result is read at desktop
width and SHALL become static at phone width. The rule matrix SHALL present every
rule result in one row or one labelled card containing a verdict chip of symbol,
word and border; every compared value with its field; the provenance source,
reference and observation time for each value; and the rule's follow-up. Every
rule, value, explanation, method, rule identifier and rule version the report
carries SHALL remain rendered, and the part selector SHALL keep its source order.

#### Scenario: The summary names all five counts
Given a completed check, when the result renders, then the summary bar names the overall status, rules passed, conflicts, unknown rules and parts not evaluated, each as a number or a labelled status.

#### Scenario: A rule row carries its verdict, values and source
Given any rule result, when its matrix row is read, then it contains a chip with the status symbol and word, every compared field and value, the provenance source and reference for each value, and the follow-up sentence when one exists.

#### Scenario: Status is never colour alone
Given an incompatible, an unknown and a compatible rule, when their chips render, then each contains its symbol, its word and a border in addition to its colour.

#### Scenario: No rule or value is dropped
Given a report with a rule whose evidence value is absent, when the matrix renders, then the row still states `Not provided`, keeps the rule identifier and version, and the total row count equals the report's rule count.

### Requirement: The diagnosis uses the same console language
The `/diagnose` result SHALL present the four advice groups — must replace,
should upgrade and verify first, fine as is, and not verified — as console group
cards, each carrying a status chip of symbol, word and border beside its existing
heading and count. The described-build evidence table and the upgrade plan SHALL
keep every part, quantity, source, rule identifier, price, follow-up, rejected
candidate and validation state. The stable numbered skeleton, its section
headings, ids and statuses SHALL NOT be renumbered or reordered.

#### Scenario: Each group is a card with a status chip
Given a diagnosis with all four advice groups, when the result renders, then each group is a card whose header contains the unchanged heading and count plus a symbol-and-word chip.

#### Scenario: Every diagnosis fact survives
Given a diagnosis with steps, rejected candidates, follow-ups and an unverified part, when the result renders, then every step, price, rule identifier, rejected candidate, follow-up and unverified part is still present.

#### Scenario: The skeleton does not move
Given the diagnosis page before and after a result, when the numbered headings are read, then they are still `1. Choose an inventory snapshot` through `5. Read the diagnosis` in that order with the same ids.

### Requirement: Console counts use real plural forms
The check and diagnosis interfaces SHALL render every count they compose through
one shared pluralisation helper that yields the singular noun for a count of one
and the plural noun otherwise. No check-composed or diagnosis-composed string
SHALL contain an `(s)` pseudo-plural. Recommendation prose and exported report
text pinned by the workbench package SHALL remain verbatim.

#### Scenario: Singular and plural forms follow the count
Given the shared helper, when it is called with one unit and with two units, then it returns `1 unit` and `2 units`.

#### Scenario: No page-composed pseudo-plural remains
Given a completed check and a completed diagnosis at any count, when the rendered page text is searched for `line(s)`, `part(s)`, `unit(s)`, `rule(s)` or `conflict(s)`, then none is found in text the page composed.

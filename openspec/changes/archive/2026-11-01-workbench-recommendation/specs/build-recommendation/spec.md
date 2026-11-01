# build-recommendation deltas

## ADDED Requirements

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

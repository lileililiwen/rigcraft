# build-diagnosis deltas

## ADDED Requirements

### Requirement: Plan summary prose uses grammatical plural forms
The upgrade-plan summary and the no-option summary SHALL spell a count with the
singular noun only for exactly one and the plural noun otherwise, and the verb
around the count SHALL agree with it, so one step reads `1 step adds` and two
read `2 steps add`. No diagnosis-composed sentence SHALL contain the `(s)`
pseudo-plural. Identifiers, rule ids, plan status values (`validated`,
`needs_evidence`, `no_option`, `not_needed`), prices and the report structure
SHALL NOT change.

#### Scenario: One step reads as one step
Given a validated plan with one step costing USD 379.00, when the summary renders, then it reads `1 step adds USD 379.00` and not `1 step(s) add`.

#### Scenario: Several steps read as several
Given a validated plan with two steps, when the summary renders, then it reads `2 steps add` with the same cost wording.

#### Scenario: The no-option rule count is grammatical
Given a snapshot with no validated replacement for one rule, when the plan summary renders, then it names `1 rule` and the rule identifier.

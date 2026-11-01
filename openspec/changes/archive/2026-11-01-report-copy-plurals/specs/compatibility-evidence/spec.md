# compatibility-evidence deltas

## ADDED Requirements

### Requirement: Stock refusal prose uses grammatical plural forms
An `insufficient_stock` refusal SHALL spell the requested quantity with the
singular noun only for exactly one and the plural noun otherwise, so one unit
reads `needs 1 unit` and two read `needs 2 units`. The error code, its HTTP
status and its `details` fields SHALL NOT change, and no check-composed refusal
SHALL contain the `(s)` pseudo-plural.

#### Scenario: One requested unit is singular
Given a selection that requests one unit of a SKU with none available, when the check is requested, then the refusal is `insufficient_stock` and the message reads `needs 1 unit`.

#### Scenario: Several requested units are plural
Given a selection that requests two units of a SKU with one available, when the check is requested, then the message reads `needs 2 units` and `details` still carries the requested and available counts.

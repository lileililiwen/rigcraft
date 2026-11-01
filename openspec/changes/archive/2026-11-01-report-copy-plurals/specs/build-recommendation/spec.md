# build-recommendation deltas

## ADDED Requirements

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

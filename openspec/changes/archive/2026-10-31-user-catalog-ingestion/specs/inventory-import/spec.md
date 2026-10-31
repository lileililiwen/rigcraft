# Inventory import deltas

## ADDED Requirements

### Requirement: A commit can apply catalog proposals per row and never overwrite an operator value
`POST /api/imports/{previewId}/commit` SHALL accept an optional per-row catalog
opt-in naming a stored part. When present, the commit SHALL apply only the fields
the row leaves empty and SHALL report conflicts in the match response without
resolving them. An applied field SHALL carry the source
`catalog:<dataset-name>@<version-or-commit>` and the part's source URL as the
reference; the row's SKU, quantity, price and currency SHALL remain `user` facts.
An operator-supplied model or specification SHALL never be replaced by a catalog
value, and a correction SHALL still own the field it edits.

#### Scenario: Opt in and add only the missing specification
Given a preview row with no specifications that matched an active catalog part, when the operator opts in for that row and commits, then the new record has the catalog specifications with catalog provenance and the operator's model, SKU, quantity and price unchanged.

#### Scenario: Operator correction outranks the catalog
Given a row that matched a catalog part, when the operator corrects a specification to a different value and commits, then the committed record has the operator's value and operator provenance, and no catalog value replaces it.

#### Scenario: An unknown part ID is refused
Given a commit decision that names a catalog part that does not exist, when the commit runs, then the response is 422 `catalog_part_not_found` and no snapshot is written.

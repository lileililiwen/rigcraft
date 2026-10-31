# Import and correct owned inventory

## ADDED Requirements

### Requirement: inventory-import provides a versioned, private, usable inventory snapshot
The system SHALL deliver a versioned, private, usable inventory snapshot through /inventory, using POST /api/imports previews a UTF-8 CSV and returns rows plus field/error codes; POST /api/imports/{id}/commit accepts explicit row decisions and returns snapshotId; GET /api/inventory/snapshots/{id} returns the immutable snapshot. Reject files above 10 MiB and non-UTF-8 input with 400; no partial commit. It SHALL preserve this invariant: Duplicate SKU requires an explicit merge/replace/keep-separate decision. Missing required identity, quantity, currency or price prevents that row from commit. Unknown hardware specifications remain unknown with a source tag.

#### Scenario: Import valid stock
Given a UTF-8 CSV with supported columns and a currency, when the operator maps fields and commits valid rows, then the snapshot shows original values, normalized values, stock, price and provenance.

#### Scenario: Reject bad row without silent loss
Given invalid quantity, price or duplicate SKU, when preview completes, then the UI names row and field, requires a correction or exclusion, and no invalid row is committed.

#### Scenario: Reproduce snapshot
Given a committed snapshot, when inventory is edited later, then the original snapshot remains readable and its ID and values do not change.

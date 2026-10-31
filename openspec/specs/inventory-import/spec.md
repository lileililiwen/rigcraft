# inventory-import Specification

## Purpose

Define the versioned, private inventory import contract: CSV validation and mapping, explicit duplicate decisions, immutable snapshots, visible provenance, and the accessible `/inventory` Web flow.

## Requirements
### Requirement: Inventory preview validates a UTF-8 CSV and reports every row
The system SHALL preview a UTF-8 CSV through `POST /api/imports`, accept one batch currency, return the detected column mapping, and return every parsed row with its original values, normalized values, specs, provenance, and field-coded errors. It SHALL reject files above 10 MiB, non-UTF-8 input, missing required columns, and unsupported categories with HTTP 400 and a stable error code. Empty input SHALL produce an empty state rather than a snapshot.

#### Scenario: Import valid stock
Given a UTF-8 CSV with supported columns and one currency, when the operator previews and commits the rows, then the snapshot shows the original values, normalized values, quantity, price, and per-field provenance.

#### Scenario: Reject a bad row without silent loss
Given invalid quantity, price, currency, or category values, when the preview completes, then the UI names the row and field with a suggestion, the row is excluded by default, and no invalid row is committed.

#### Scenario: Reject an unreadable file
Given a file above 10 MiB or bytes that are not UTF-8, when preview is requested, then the service returns HTTP 400 with `file_too_large` or `not_utf8` and the UI keeps the selected file so the operator can choose another.

### Requirement: Duplicate SKUs require an explicit merge, replace, or keep-separate decision
The system SHALL group rows that share a normalized SKU and SHALL require an explicit merge, replace, or keep-separate decision before those rows can be committed. Merge SHALL sum quantities and SHALL refuse rows whose price, model, category, or specs conflict. Replace SHALL keep the last included row. Keep-separate SHALL keep each included row as a distinct lot. The system SHALL NOT silently overwrite or drop a duplicate row.

#### Scenario: Merge two stock lines
Given two rows with the same SKU, price, model, and specs, when the operator chooses merge and commits, then one record is created with the summed quantity and both rows are accounted for.

#### Scenario: Block commit until duplicates are resolved
Given two included rows with the same SKU, when no resolution is chosen, then commit returns HTTP 422 with `duplicate_unresolved` and no snapshot is written.

### Requirement: Snapshots are immutable, versioned, and readable after correction
The system SHALL persist each committed snapshot in SQLite with a UUID, revision number, parent snapshot ID, source file, checksum, currency, and integer minor-unit prices. `GET /api/inventory/snapshots/{id}` SHALL return the immutable snapshot. A correction through `POST /api/inventory/snapshots/{id}/corrections` SHALL create a new revision with new provenance for changed fields and SHALL never edit the old snapshot. Commit SHALL be atomic: a failed commit writes no snapshot. Repeating the same preview and decision set SHALL return the same snapshot, while a different decision set SHALL return HTTP 409. Preview IDs SHALL expire after 24 hours.

#### Scenario: Reproduce a snapshot after correction
Given a committed snapshot, when the operator changes a quantity or price, then a new revision with a parent link is created and the original snapshot ID and values remain readable and unchanged.

#### Scenario: Fail without a partial commit
Given a preview with at least one included invalid row, when commit is requested, then the service returns HTTP 422 with the offending rows and the snapshot list remains unchanged.

#### Scenario: Expire an old preview
Given a preview older than 24 hours, when commit is requested, then the service returns HTTP 410 with `preview_expired` and writes nothing.

### Requirement: Provenance and unknown specifications stay visible end to end
The system SHALL label each value by source (`user`, `rigcraft-normalizer`, or a licensed source), reference, and observation time. Normalization SHALL be lexical only; the system SHALL NOT invent Socket, dimensions, BIOS support, or connector values. Specifications absent from the import SHALL remain `Unknown` with the user source tag, and the committed snapshot and the Web UI SHALL both display that state without relying on color alone.

#### Scenario: Show an unverified match and unknown specs
Given an imported model with no catalog match and no specification values, when the snapshot is displayed, then the UI shows an unverified match and an `Unknown` specification state with the source reference.

#### Scenario: Distinguish user and normalized provenance
Given a row with a model and a quantity, when the snapshot is displayed, then the model normalization cites `rigcraft-normalizer` while the user-entered quantity cites `user` and the CSV reference.

### Requirement: The Web flow is keyboard usable, responsive, and self-hosted
The `/inventory` route SHALL cover empty, selecting, parsing, review, row correction, validation error, commit pending, committed, retry, and earlier-revision states. It SHALL use labelled controls, visible focus, status announcements, non-color-only outcomes, a desktop layout and a stacked mobile layout, and SHALL preserve entered form values on validation or network failure. Inventory SHALL be stored outside the repository in a local data directory chosen by `RIGCRAFT_DATA_DIR`, and CSV content and cost fields SHALL be excluded from logs.

#### Scenario: Complete the journey without a pointer at narrow width
Given a phone-width viewport and keyboard-only operation, when the operator previews, corrects, resolves duplicates, and commits, then every action is reachable, the focus order is visible, and no content overflows horizontally.

#### Scenario: Preserve work and offer retry after a network failure
Given a selected file or a parsed preview, when the local service becomes unreachable, then the UI announces the failure, keeps the current values, and offers a retry action.

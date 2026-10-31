# Inventory import deltas

## ADDED Requirements

### Requirement: A committed snapshot can be exported back into the import format
The inventory contract SHALL expose a CSV export whose columns are exactly the
import contract's accepted columns, so an exported catalog can be previewed and
committed again without editing. Operator stock facts SHALL remain operator facts and
declared catalog facts SHALL remain declared catalog facts after that round trip. The
import documentation SHALL cross-reference the export and repeat that CSV cannot carry
lot identity or per-field provenance references.

#### Scenario: Exported CSV is accepted by the importer
Given a committed snapshot, when its CSV export is uploaded to `POST /api/imports` with the same batch currency, then the preview reports no field error and every row is valid.

#### Scenario: Provenance survives the round trip
Given an exported CSV from a snapshot that mixed catalog and operator facts, when it is committed again, then the specification and model provenance sources are unchanged.

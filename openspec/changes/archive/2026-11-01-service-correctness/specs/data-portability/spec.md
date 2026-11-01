# data-portability deltas

## MODIFIED Requirements

### Requirement: A snapshot can be exported as CSV in the importer's own columns
The system SHALL serve `GET /api/inventory/snapshots/{id}/export?format=csv` as a
UTF-8 CSV with exactly the header
`sku,category,model,quantity,price,currency,specs,source,sourceUrl`, one row per lot
in stored order. Each row SHALL reproduce the committed `sku`, canonical `category`,
original model text, integer quantity, two-decimal price, currency, raw
`key=value` specifications, the model's declared catalog source (or `user`), and an
absolute HTTP(S) `sourceUrl` when the provenance reference is one.

Before a value is written, the export SHALL neutralise it by prefixing one
apostrophe (`'`) when its first character is `=`, `+`, `-`, `@`, a tab or a carriage
return, and also when its first character is a single or double quote followed by
one of those characters. Every other value SHALL be written unchanged. The rule
SHALL apply to every column, and the JSON export SHALL keep the raw value. The
neutralisation SHALL be documented as lossy: a value that legitimately begins with
one of those characters gains one apostrophe when the file is re-imported.

Re-importing that CSV through `POST /api/imports` SHALL reproduce the same records
and the same provenance sources: operator fields stay `user`, catalog part facts keep
their `catalog:<id>` value.

CSV SHALL be documented and header-announced as lossy: it carries no lot identity,
snapshot metadata, per-field provenance reference, or match confidence. The response
SHALL include `X-Rigcraft-Export-Lossy: true` and a human-readable lossiness note
header.

#### Scenario: CSV export re-imports to the same records and provenance sources
Given a committed snapshot with catalog and operator facts, when its CSV export is imported again, then the new snapshot has the same SKUs, categories, models, quantities, prices, currencies and specifications, the model facts still cite `catalog:<id>`, and the stock facts still cite `user`.

#### Scenario: CSV states its own limitation
Given a successful CSV export, when the response is inspected, then the content type is CSV, the lossiness headers are present, and the filename contains the snapshot revision.

#### Scenario: One row per lot
Given a snapshot with two lots that share an SKU, when it is exported as CSV, then the file has two data rows and the header appears once.

#### Scenario: A formula-leading cell is neutralised in CSV and raw in JSON
Given a record whose model is `=1+1`, when it is exported in both formats, then the CSV model cell is `'=1+1`, the JSON value is `=1+1`, and no other byte of the CSV cell changes.

#### Scenario: An ordinary cell is untouched
Given a record whose model is `AMD Ryzen 7 7800X3D`, when it is exported as CSV, then the model cell is written unchanged with no apostrophe.

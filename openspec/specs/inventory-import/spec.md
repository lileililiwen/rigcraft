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

Every line total (`priceMinor × quantity`) and every snapshot total SHALL be computed with checked arithmetic. A commit or correction whose line total, unit count, or value total cannot be represented in a signed 64-bit integer SHALL fail with HTTP 422 `price_overflow` and SHALL write no snapshot or revision. Totals SHALL NOT saturate at `i64::MAX`.

#### Scenario: Reproduce a snapshot after correction
Given a committed snapshot, when the operator changes a quantity or price, then a new revision with a parent link is created and the original snapshot ID and values remain readable and unchanged.

#### Scenario: Fail without a partial commit
Given a preview with at least one included invalid row, when commit is requested, then the service returns HTTP 422 with the offending rows and the snapshot list remains unchanged.

#### Scenario: Expire an old preview
Given a preview older than 24 hours, when commit is requested, then the service returns HTTP 410 with `preview_expired` and writes nothing.

#### Scenario: Reject an unrepresentable correction
Given a correction that sets a record price and quantity whose line total overflows a signed 64-bit integer, when it is submitted, then the service returns HTTP 422 `price_overflow` with field `price`, the stored snapshot is unchanged, and no revision is created.

#### Scenario: Reject an unrepresentable commit
Given a CSV row whose unit price and quantity each parse but whose line total overflows, when the preview is committed, then the service returns HTTP 422 `price_overflow` and the snapshot list stays empty.

### Requirement: Provenance and unknown specifications stay visible end to end
The system SHALL label each value by source (`user`, `rigcraft-normalizer`, or a declared catalog source), reference, and observation time. Normalization SHALL be lexical only; the system SHALL NOT invent Socket, dimensions, BIOS support, or connector values. Specifications absent from the import SHALL remain `Unknown` and SHALL NOT be filled in from a similar name.

Optional `source` and `sourceUrl` CSV columns SHALL let a row declare where its part facts came from. A `source` value SHALL be either `user` or `catalog:<id>`, and a `sourceUrl` value SHALL be an absolute `http://` or `https://` URL. When a catalog source is declared, the row's `model` and every parsed specification SHALL cite that source with `sourceUrl` (or the CSV reference when no URL is recorded) as the reference. `sku`, `quantity`, `price` and `currency` SHALL remain `user` facts. A missing or empty `source` or `sourceUrl` column SHALL leave provenance exactly as before. An invalid value SHALL be a field error with a stable code, and the row SHALL be excluded by default.

The committed snapshot and the Web UI SHALL show whether a fact is operator-entered, catalog-provided, normalized, or unknown without relying on colour alone. The preview and the snapshot SHALL show the catalog source when one is declared, and a correction to model or specifications SHALL replace the catalog claim with `user` provenance.

#### Scenario: Show an unverified match and unknown specs
Given an imported model with no catalog source and no specification values, when the snapshot is displayed, then the UI shows an `unverified` match and an `Unknown` specification state with the source reference.

#### Scenario: Distinguish user and normalized provenance
Given a row with a model and a quantity and no source column, when the snapshot is displayed, then the model normalization cites `rigcraft-normalizer` while the user-entered quantity cites `user` and the CSV reference.

#### Scenario: Keep operator stock facts out of a catalog claim
Given a row with `source=catalog:buildcores@a1b2c3d` and a manufacturer URL, when it is committed, then the model and every specification cite `catalog:buildcores@a1b2c3d` with that URL, while the SKU, quantity and price still cite `user` and the CSV reference.

#### Scenario: Fall back when the columns are absent
Given a CSV with no `source` or `sourceUrl` column, when it is previewed and committed, then its provenance is identical to the behaviour before these columns existed.

#### Scenario: Reject an invalid catalog claim
Given a row whose `source` is not `user` or `catalog:<id>`, or whose `sourceUrl` is not an absolute HTTP(S) URL, when it is previewed, then the row has a `source_format` or `source_url_format` field error and is excluded by default.

#### Scenario: An operator correction owns the edited facts
Given a catalog-sourced row, when the operator corrects its model or specifications before commit, then the corrected row cites `user` with the correction reference instead of the catalog source.

### Requirement: The Web flow is keyboard usable, responsive, and self-hosted
The `/inventory` route SHALL cover empty, selecting, parsing, review, row correction, validation error, commit pending, committed, retry, and earlier-revision states. It SHALL use labelled controls, visible focus, status announcements, non-color-only outcomes, a desktop layout and a stacked mobile layout, and SHALL preserve entered form values on validation or network failure. Inventory SHALL be stored outside the repository in a local data directory chosen by `RIGCRAFT_DATA_DIR`, and CSV content and cost fields SHALL be excluded from logs.

#### Scenario: Complete the journey without a pointer at narrow width
Given a phone-width viewport and keyboard-only operation, when the operator previews, corrects, resolves duplicates, and commits, then every action is reachable, the focus order is visible, and no content overflows horizontally.

#### Scenario: Preserve work and offer retry after a network failure
Given a selected file or a parsed preview, when the local service becomes unreachable, then the UI announces the failure, keeps the current values, and offers a retry action.

### Requirement: Currency codes are case-insensitive and normalized to uppercase
`parse_currency` SHALL trim the submitted value, accept letters in any case, and normalize them to three uppercase ASCII letters. A value that is not exactly three ASCII letters after trimming SHALL fail with HTTP 400 `currency_format`; an empty value SHALL fail with `currency_required` or `currency_format` according to the endpoint. Every endpoint that accepts a currency (import, recommendation, diagnosis parse and diagnosis) SHALL use this one normalization. The Web UI SHALL continue to send uppercase codes.

#### Scenario: Lowercase currency is normalized
Given a request body with `"currency":"usd"`, when it is accepted, then the response and every stored record show `USD`.

#### Scenario: Two-letter currency is rejected
Given a request body with `"currency":"us"`, when it is submitted, then the service returns HTTP 400 `currency_format` and performs no other change.

### Requirement: Every rejected HTTP request uses the documented JSON error envelope
Every rejected request, including extractor failures that occur before a handler runs, SHALL return `application/json` with the shape `{ "error": { "code", "message", "field"?, "row"?, "details"? } }`. Unparseable JSON and wrongly typed JSON fields SHALL return HTTP 400 `invalid_json`; a missing or unsupported JSON content type, a malformed multipart body, and a body-limit rejection SHALL return HTTP 400 or the extractor's status with `invalid_body`. No rejection SHALL return `text/plain`.

#### Scenario: Wrongly typed JSON field
Given `POST /api/builds/check` with `{"selection":[{"quantity":1.5}]}`, when it is submitted, then the service returns HTTP 400 with `application/json` and `error.code` `invalid_json`.

#### Scenario: Malformed JSON body
Given `POST /api/builds/check` with the body `{bad`, when it is submitted, then the service returns HTTP 400 with `application/json` and `error.code` `invalid_json`.

#### Scenario: Malformed multipart body
Given `POST /api/imports` without a usable multipart boundary, when it is submitted, then the service returns HTTP 400 with `application/json` and `error.code` `invalid_body`.

### Requirement: The SPA shell is served only for client routes, not static paths
The service SHALL serve the built SPA shell only for real client routes and history-mode application paths that have no file extension and are not under `/assets/`. A path whose final segment contains a dot SHALL be treated as a static or metadata file: it SHALL be served when it exists and SHALL otherwise return HTTP 404, never the SPA shell. Unknown `/api/*` routes SHALL remain JSON `route_not_found` 404 responses. The service SHALL NOT invent the content of `robots.txt`, `llms.txt`, `ai-catalog.json`, or any other file the build does not ship.

#### Scenario: Unknown asset is a 404
Given a built Web app, when `/assets/nope.js` is requested, then the service returns HTTP 404 and does not return `index.html`.

#### Scenario: Unshipped metadata is a 404
Given a built Web app, when `/robots.txt`, `/llms.txt`, or `/ai-catalog.json` is requested, then the service returns HTTP 404 with no HTML shell.

#### Scenario: A client route still returns the shell
Given a built Web app, when `/diagnose` or another extensionless application path is requested, then the service returns the SPA shell with HTTP 200 and `text/html`.

### Requirement: The shipped sample build is sourced, verified and labelled
`GET /api/imports/sample` SHALL serve a CSV whose rows form one physically possible, internally consistent build in the supported categories. Each compatibility fact SHALL be taken from the manufacturer's own product page, SHALL record that page in `sourceUrl`, and SHALL record the dataset revision and verification date in `source`. A fact the cited page does not state SHALL be omitted so that it stays unknown; it SHALL NOT be inferred. The sample SHALL declare `price` and `quantity` as operator-entered stock values rather than market or catalog data.

The `/inventory` download area SHALL state that the sample data is licensed CC BY 4.0, when its facts were checked, and that prices are operator-entered.

#### Scenario: The sample passes the compatibility engine without a conflict
Given the shipped sample committed to a snapshot and one unit of every category selected, when the compatibility check runs, then no rule reports a conflict, and every rule that lacks a sourced fact reports `unknown` with a follow-up.

#### Scenario: The impossible socket pairing is gone
Given the shipped sample, when its CPU and motherboard rows are compared, then the CPU Socket equals the motherboard Socket and no row declares an AM4 processor as AM5.

#### Scenario: A missing manufacturer fact stays unknown
Given a manufacturer page that does not state a value, when the sample is authored, then that specification is absent and the affected rule reports `unknown` rather than a guessed value.

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


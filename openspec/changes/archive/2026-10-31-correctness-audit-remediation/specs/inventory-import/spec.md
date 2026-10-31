# Inventory import correctness deltas

## MODIFIED Requirements

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

## ADDED Requirements

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

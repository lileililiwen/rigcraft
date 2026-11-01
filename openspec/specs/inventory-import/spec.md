# inventory-import Specification

## Purpose

Define the versioned, private inventory import contract: CSV validation and mapping, explicit duplicate decisions, immutable snapshots, visible provenance, and the accessible `/inventory` Web flow.
## Requirements
### Requirement: Inventory preview validates a UTF-8 CSV and reports every row
The system SHALL preview a UTF-8 CSV through `POST /api/imports`, accept one batch currency, return the detected column mapping, and return every parsed row with its original values, normalized values, specs, provenance, and field-coded errors. It SHALL reject files above 10 MiB, non-UTF-8 input, missing required columns, and unsupported categories with HTTP 400 and a stable error code. Empty input SHALL produce an empty state rather than a snapshot.

When the required columns are missing and the parsed header row is a single column that contains `;`, the `mapping_missing` refusal SHALL name the likely cause in its message: that the header looks semicolon-delimited and that comma-separated columns are expected. The code, the field and the missing-column list SHALL stay unchanged, and the refusal SHALL NOT echo file content beyond a normalised header shape.

#### Scenario: Import valid stock
Given a UTF-8 CSV with supported columns and one currency, when the operator previews and commits the rows, then the snapshot shows the original values, normalized values, quantity, price, and per-field provenance.

#### Scenario: Reject a bad row without silent loss
Given invalid quantity, price, currency, or category values, when the preview completes, then the UI names the row and field with a suggestion, the row is excluded by default, and no invalid row is committed.

#### Scenario: Reject an unreadable file
Given a file above 10 MiB or bytes that are not UTF-8, when preview is requested, then the service returns HTTP 400 with `file_too_large` or `not_utf8` and the UI keeps the selected file so the operator can choose another.

#### Scenario: A semicolon-delimited CSV names the delimiter
Given a CSV whose first row is `sku;category;model;quantity;price;currency`, when preview is requested, then the response is HTTP 400 `mapping_missing` with the usual field and missing-column list, and the message states that the header looks semicolon-delimited and that comma-separated columns are expected.

#### Scenario: A genuine missing column gets no delimiter hint
Given a comma-separated CSV whose header has several columns and omits `category`, when preview is requested, then the response is `mapping_missing` and the message does not claim a semicolon delimiter.

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
The `/inventory` route SHALL cover empty, selecting, parsing, review, row
correction, validation error, commit pending, committed, retry, earlier-revision
and restored states. When snapshot history is not empty, the route SHALL open the
newest revision automatically on first load, announcing it through the polite
status region and without moving focus, so the export and backup actions are
usable without a prior selection step. `Start a new import` SHALL close the open
snapshot and return the route to the empty import state, and it SHALL NOT be undone
by a later history refresh. The route SHALL use labelled controls, visible focus,
status announcements, non-color-only outcomes, a desktop layout and a stacked
mobile layout, and SHALL preserve entered form values on validation or network
failure. Inventory SHALL be stored outside the repository in a local data directory
chosen by `RIGCRAFT_DATA_DIR`, and CSV content and cost fields SHALL be excluded
from logs.

#### Scenario: Complete the journey without a pointer at narrow width
Given a phone-width viewport and keyboard-only operation, when the operator previews, corrects, resolves duplicates, and commits, then every action is reachable, the focus order is visible, and no content overflows horizontally.

#### Scenario: Preserve work and offer retry after a network failure
Given a selected file or a parsed preview, when the local service becomes unreachable, then the UI announces the failure, keeps the current values, and offers a retry action.

#### Scenario: The newest revision opens on arrival
Given at least one committed snapshot, when `/inventory` is loaded with no preview open, then the newest revision's summary, records and export actions are available without activating a history control, and the first Tab stop is still the skip link.

#### Scenario: Starting a new import closes the open snapshot
Given an automatically opened snapshot, when the operator activates `Start a new import`, then the snapshot view closes, the import panel is ready for a new file, and the view does not reopen on its own.

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
The service SHALL serve the built SPA shell only for the client routes `/`, `/inventory`, `/catalog`, `/builds/check`, `/recommend` and `/diagnose`, each also accepted with one trailing slash. Any other extensionless path SHALL return HTTP 404 with a not-found document that is not the SPA shell. A path whose final segment contains a dot SHALL be treated as a static or metadata file: it SHALL be served when it exists and SHALL otherwise return HTTP 404, never the SPA shell. `GET /api`, `GET /api/` and every unknown path under `/api/` SHALL return HTTP 404 in the documented JSON error envelope with code `route_not_found`, never HTML, while a request with a wrong method to a real API route SHALL keep its HTTP 405. The service SHALL NOT invent the content of `robots.txt`, `llms.txt`, `ai-catalog.json`, or any other file the build does not ship.

#### Scenario: Unknown asset is a 404
Given a built Web app, when `/assets/nope.js` is requested, then the service returns HTTP 404 and does not return `index.html`.

#### Scenario: Unshipped metadata is a 404
Given a built Web app, when `/robots.txt`, `/llms.txt`, or `/ai-catalog.json` is requested, then the service returns HTTP 404 with no HTML shell.

#### Scenario: A client route still returns the shell
Given a built Web app, when `/`, `/inventory`, `/catalog`, `/builds/check`, `/recommend` or `/diagnose` is requested, then the service returns the SPA shell with HTTP 200 and `text/html`, and `/inventory/` returns the same.

#### Scenario: An unknown extensionless path is a real 404
Given a built Web app, when `/nope`, `/recomend` or another unlisted extensionless path is requested, then the service returns HTTP 404 with a document that is not the SPA shell and is not the Inventory page.

#### Scenario: The API root is a JSON 404
Given a running service, when `GET /api` or `GET /api/` is requested, then the response is HTTP 404 `application/json` with `error.code` `route_not_found`, and an unknown `/api/*` path behaves the same.

#### Scenario: A wrong method keeps its 405
Given a running service, when `POST /api/health` is requested, then the response is HTTP 405 rather than a 404 or the SPA shell.

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

### Requirement: Text responses negotiate gzip or brotli and declare the negotiation
The service SHALL honour `Accept-Encoding` for text responses whose media type is
`text/*`, `application/json`, `application/javascript` or `image/svg+xml`, and SHALL
answer with `Content-Encoding: gzip` or `Content-Encoding: br` when the client asks
for one of those codings and the plain body is at least 256 bytes. A response that is
compressed SHALL NOT carry a `Content-Length` that describes the uncompressed body and
SHALL keep range semantics by never compressing a response that has `Content-Range`.
A response whose `Content-Encoding` is not negotiated SHALL be the identity bytes with
an accurate `Content-Length`. An `image/x-icon` response SHALL NOT be re-encoded even
when a coding is offered. Every text response SHALL declare `Vary: Accept-Encoding`,
whether or not this request was compressed.

#### Scenario: A page negotiates compression
Given a built Web app and a client that sends `Accept-Encoding: gzip`, when `/recommend` is requested, then the response has `Content-Encoding: gzip`, no stale `Content-Length`, and `Vary: Accept-Encoding`.

#### Scenario: A client that asks for nothing gets the plain bytes
Given a built Web app and a client with no `Accept-Encoding`, when `/recommend` is requested, then the response has no `Content-Encoding`, its `Content-Length` equals the body length, and it still declares `Vary: Accept-Encoding`.

#### Scenario: An icon is never re-compressed
Given a built Web app and a client that accepts `gzip` and `br`, when `/favicon.ico` is requested, then the response is the identity icon with `Content-Length` and no `Content-Encoding`.

### Requirement: Static assets and pages declare one cache policy
The service SHALL apply one cache policy to the whole delivery surface. A successful
`GET /assets/*` SHALL be `Cache-Control: public, max-age=31536000, immutable`.
`index.html` and every HTML response SHALL be `Cache-Control: no-cache`.
`/favicon.ico` and `/favicon.svg` SHALL be `Cache-Control: public, max-age=86400`.
Every response under `/api` SHALL be `Cache-Control: no-store`. A missing asset, a
404 document and any other response SHALL NOT receive the immutable policy. The policy
SHALL be decided in one place for every route rather than by an individual handler.

#### Scenario: A hashed bundle is immutable
Given a built Web app, when `/assets/index-abc123.js` is requested, then the response is `200` with `Cache-Control: public, max-age=31536000, immutable`.

#### Scenario: The shell is revalidated
Given a built Web app, when `/` or `/recommend` is requested, then the response is `200` `text/html` with `Cache-Control: no-cache`.

#### Scenario: A missing asset is not immutable
Given a built Web app, when `/assets/missing.js` is requested, then the response is `404` and carries no immutable `Cache-Control`.

#### Scenario: Inventory and assistant traffic is not stored
Given a running service, when `/api/health` is requested, then the response is `Cache-Control: no-store`.

### Requirement: Every response carries a baseline security header policy
Every response SHALL carry `X-Content-Type-Options: nosniff`,
`Referrer-Policy: no-referrer`, `X-Frame-Options: DENY` and
`Content-Security-Policy` with the value
`default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'self' https: http://127.0.0.1:* http://localhost:*; frame-ancestors 'none'; base-uri 'none'; object-src 'none'; form-action 'self'`.
This SHALL include an API response, a static asset and the 404 document, not only a
rendered page. The build SHALL contain no inline `<script>`, so `script-src 'self'`
loads the bundle. With the policy active, all six client routes SHALL render, the
assistant SHALL still reach an operator-supplied loopback provider through the service,
and the browser console SHALL report no CSP violation.

#### Scenario: A page carries the full policy
Given a built Web app, when `/recommend` is requested, then the response has `nosniff`, `no-referrer`, `X-Frame-Options: DENY`, and a CSP whose `script-src` is `'self'` without `'unsafe-inline'` or `'unsafe-eval'`.

#### Scenario: An API error carries the same policy
Given a running service, when `/api/nope` is requested, then the JSON 404 response also carries the four security headers.

#### Scenario: The 404 document carries the same policy
Given a built Web app, when `/recomend` is requested, then the plain-text 404 response also carries the four security headers.

#### Scenario: The policy does not break the app
Given the production build served with the policy, when each of `/`, `/inventory`, `/catalog`, `/builds/check`, `/recommend` and `/diagnose` is loaded and the assistant is pointed at a loopback provider, then every route renders, the provider request completes, and the console shows no CSP violation.

#### Scenario: A typo is still a real 404 under the policy
Given the production build served with the policy, when `/recomend` is visited, then the browser receives status `404` and the not-found document rather than the SPA shell.


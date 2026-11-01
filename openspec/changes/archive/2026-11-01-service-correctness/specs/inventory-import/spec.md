# inventory-import deltas

## MODIFIED Requirements

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

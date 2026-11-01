# Proposal: Make the HTTP surface and the CSV export honest

## Why

Four confirmed review defects let the service say something untrue.
`GET /api` and `GET /api/` return the SPA shell with 200, because the JSON 404
fallback (`/api/{*rest}`) does not match `/api` and the remaining fallback calls
every extensionless path a client route. Any unknown extensionless path (`/foo`,
`/recomend`) also returns the shell with 200 and renders Inventory, so a user who
mistypes a route is told nothing went wrong. The CSV export writes every cell raw,
so a model stored as `=1+1` exports as an executable formula into an operator's
spreadsheet, fed by third-party catalog text. A semicolon-delimited CSV reports only
"required columns are missing", naming nothing the operator can act on.

## What Changes

- **API surface.** `GET /api`, `GET /api/` and every unknown path under `/api/`
  return the existing JSON error envelope with 404 `route_not_found`. A wrong
  method on a real route keeps its 405. No other API shape changes.
- **Client routes.** The SPA shell is served only for `/`, `/inventory`,
  `/catalog`, `/builds/check`, `/recommend` and `/diagnose`, each with an
  optional trailing slash. Every other extensionless path returns a real 404
  document that is not the shell. `/assets/*` and metadata files keep their
  current 404 behaviour.
- **CSV neutralisation.** `snapshot_csv` prefixes a cell with one apostrophe when
  its first character is `=`, `+`, `-`, `@`, TAB or CR, or when it begins with a
  quote followed by one of those. JSON export keeps the raw value. The rule and
  its lossiness are documented.
- **Delimiter hint.** A semicolon-delimited header that leaves required columns
  missing now names the likely cause in the `mapping_missing` message, with the
  same code, field and missing-column list.
- **Tests and evidence.** A Rust route matrix covers `/api`, `/api/`, `/api/nope`,
  `/nope`, `/recomend`, the six client routes, `/inventory/` and
  `/assets/missing.js`; unit/HTTP tests cover the neutralisation and the hint; a
  Playwright journey visits a typo path and asserts the real 404. The `curl -i`
  matrix, the exported cell bytes and captures at 1440 and 390 are recorded in
  `docs/evidence/service-correctness-browser.md`.

## Impact

- Affected specs: `inventory-import` (the SPA/client-route requirement is
  modified to enumerate the routes and to make `/api`, `/api/` JSON 404s; the
  CSV preview requirement gains the delimiter hint), `data-portability` (the CSV
  export requirement gains neutralisation and its documented lossiness).
- Affected code: `src/api.rs` (route table and fallback), `src/csv_import.rs`
  (refusal message), `src/portability.rs` (cell neutralisation), `tests/`,
  `e2e/`, `docs/`.
- Unchanged: every request and response field, every error code, the schema, the
  engine, the recommendation and diagnosis scoring, the validation rules, the
  vendored token file, the brand layer and every existing journey assertion.
- Non-goals: no compression, cache or security headers (package 6), no new
  dependency, no redirect, no custom 404 page that needs a build step.

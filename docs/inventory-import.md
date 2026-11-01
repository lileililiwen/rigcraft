# Inventory import and provenance

Status: implemented in `inventory-import-and-provenance` (2026-10-31).

This package is the first product slice of `requirement.md` (REQ-D01–D06). It delivers a private, versioned inventory snapshot through `/inventory`. Compatibility judgement, build recommendation, catalog distribution and AI planning are out of scope.

## Runtime

| Target | Command | Notes |
| --- | --- | --- |
| Rust API | `cargo run` | Binds `127.0.0.1:8080` by default; override with `RIGCRAFT_ADDR`. |
| Web build | `npm run build` | Builds `apps/web` to `apps/web/dist`, served by the Rust process. |
| Web dev | `npm run dev --workspace @rigcraft/web` | Vite proxies `/api` to `127.0.0.1:8080`. |
| Tests | `cargo test`, `npm test` | Rust unit/API tests and Vitest component tests. |
| Verification | `./verify-local.sh` | Format, clippy, Cargo tests, Web check/typecheck/tests. |

Inventory data lives outside the repository:

1. `RIGCRAFT_DATA_DIR` when set.
2. `$XDG_DATA_HOME/rigcraft`.
3. `$HOME/.local/share/rigcraft`.

The SQLite file is `inventory.sqlite3`. Schema version is tracked in `PRAGMA user_version` (currently `5`; forward migrations add compatibility `check_reports`, recommendation reports, diagnosis reports and the operator-supplied catalog registry and parts described in [catalog-ingestion.md](catalog-ingestion.md)). Provenance stays in the existing record JSON; the catalog tables never reference inventory. CSV contents and cost values are not logged; `tower-http` request tracing records method/URI only at debug level.

## HTTP contract

All JSON uses camelCase. Errors use:

```json
{ "error": { "code": "mapping_missing", "message": "...", "field": "category", "row": 4, "details": {} } }
```

Every rejection is `application/json` in that shape, including failures raised before a handler runs. Unparseable or wrongly typed JSON is HTTP 400 `invalid_json`; a missing JSON content type, a malformed multipart body and a body-limit rejection are `invalid_body`. No rejection returns `text/plain`.

### `POST /api/imports`

`multipart/form-data` with `file` (UTF-8 CSV, ≤ 10 MiB), `currency` (ISO 4217) and optional `mapping` (JSON object of logical field to column header).

Currency is case-insensitive and normalized to three uppercase ASCII letters: `usd` and `USD` both become `USD`, while `us` is `currency_format`.

Logical fields: `sku`, `category`, `model`, `quantity`, `price`, `currency`, `specs`, `cost`, `source`, `sourceUrl`. Required: the first five. Header aliases are detected case-insensitively; an explicit mapping wins. Ambiguous aliases require an explicit choice. `source` and `sourceUrl` are optional; a file without them behaves exactly as before they existed.

Returns a preview:

```text
previewId, createdAt, expiresAt, currency, filename, columns[], mapping{},
rows[] {
  row, status(valid|error|duplicate), values{}, normalized{}, specs[],
  errors[{field, code, message, suggestion}], duplicateGroupId,
  provenance{field: {source, reference, observedAt}}, match{basis, confidence}
},
duplicateGroups[{groupId, sku, rows[]}], summary{total, valid, errors, duplicateRows}
```

Stable 400 codes include `file_too_large`, `not_utf8`, `empty_file`, `no_data_rows`, `csv_headers_missing`, `csv_parse_error`, `mapping_invalid`, `mapping_missing`, `mapping_duplicate`, `ambiguous_column`, `currency_format`. Per-row source errors are `source_format` and `source_url_format`. Previews expire after 24 hours.

`mapping_missing` names the likely cause when the file is not comma-separated. A semicolon-delimited header parses as one column containing `;`, so every required column looks absent; in that case `details.delimiter` is `"semicolon"` and the message adds `The header looks semicolon-delimited; RigCraft expects comma-separated columns.` The code, the field and `details.missing` are unchanged, and the file's content is not echoed.

### `POST /api/imports/{previewId}/commit`

```json
{
  "decisions": [{
    "row": 4,
    "action": "accept|exclude",
    "overrides": { "quantity": "3" },
    "catalog": { "partId": "stored-part-id" }
  }],
  "duplicateResolutions": [{ "groupId": "dup-1", "action": "merge|replace|keep_separate" }]
}
```

The optional `catalog` opt-in applies that part's proposals to the fields this row
leaves empty, as described in [catalog-ingestion.md](catalog-ingestion.md). Only
missing fields are filled; a conflict is reported by the match endpoint and never
overwrites the operator's value. An applied field cites
`catalog:<dataset-name>@<version>` with the part's source URL, while SKU, quantity
and price stay `user` facts. A part ID that no longer exists returns `422
catalog_part_not_found` and writes no snapshot.

Every preview row needs a decision. Commit is atomic and returns `{snapshotId, snapshot}`. Replaying the same decision set returns the same snapshot; a different set returns `409 preview_already_committed`. Invalid accepted rows return `422 invalid_row` with `details.rows[]`; unresolved duplicates return `422 duplicate_unresolved`; an expired preview returns `410 preview_expired`.

Merge sums quantities and requires equal price, model, category and specs. Replace keeps the last included row. Keep-separate keeps each included row as a distinct lot.

### `GET /api/inventory/snapshots`

Returns summaries ordered newest first: `id, createdAt, revision, parentSnapshotId, currency, sourceFilename, recordCount, unitCount, valueMinor, note`.

### `GET /api/inventory/snapshots/{id}`

Returns the immutable snapshot:

```text
id, createdAt, revision, parentSnapshotId, currency,
source{filename, sha256}, note, totals{recordCount, unitCount, valueMinor},
records[] {
  id, lot, sku, category, modelRaw, modelNormalized, quantity, priceMinor,
  currency, specs[], provenance{}, match{basis, confidence}
}
```

Money is an integer in minor units with one currency per batch. `404 snapshot_not_found` for unknown IDs. Each line total (`priceMinor × quantity`) and each snapshot total is computed with checked arithmetic; an amount that cannot be represented in a signed 64-bit integer is never saturated, and the write that produced it is rejected.

### `GET /api/inventory/snapshots/{id}/export?format=csv|json`

Exports the immutable snapshot. `csv` writes one row per lot with exactly the import columns above (`sku,category,model,quantity,price,currency,specs,source,sourceUrl`), so the file can be previewed and committed again; it is lossy and omits lot identity and per-field provenance references. `json` is the authoritative artifact with snapshot metadata, revision, parent, every lot and every field's normalized value and provenance. Both are deterministic and name the snapshot revision in the download filename. An unsupported or missing format is `400 export_format_unsupported`. The guarantees, restore contract and error codes are in [docs/data-portability.md](data-portability.md).

### `POST /api/inventory/snapshots/{id}/corrections`

```json
{
  "note": "Stock recount",
  "changes": [{ "recordId": "…", "quantity": 2, "priceMinor": 18950 }]
}
```

Creates a new snapshot with `revision + 1`, `parentSnapshotId` set to the old ID, and new provenance for changed fields. The old snapshot is never edited. Supported changes: `quantity`, `priceMinor`, `sku`, `category`, `model`, `specs`. Unknown records return 404; an empty change set returns `422 no_changes`. A change whose line total or resulting snapshot total cannot be represented returns `422 price_overflow` with field `price` and writes no revision; a commit with the same condition is rejected the same way.

### Other routes

- `GET /api/imports/sample` — the sourced sample UTF-8 CSV download (see below).
- `GET /api/health` — service health.
- `GET /api/backup`, `POST /api/backup/inspect`, `POST /api/backup/restore` — whole-database backup and guarded restore, documented in [docs/data-portability.md](data-portability.md).
- `GET /inventory`, `GET /catalog`, `GET /builds/check`, `GET /recommend`, `GET /diagnose` and `GET /` — Web flows. The Rust process serves the built SPA shell only for these six routes, each with an optional trailing slash.
- `GET /catalog`, `POST /api/catalog/datasets`, `GET /api/catalog/datasets`, `PATCH|DELETE /api/catalog/datasets/{id}`, `GET /api/catalog/search`, `GET /api/catalog/parts/{id}`, `POST /api/catalog/match` — operator-supplied catalog ingestion and enrichment, documented in [catalog-ingestion.md](catalog-ingestion.md).
- Any other extensionless path (a typo such as `/recomend`, or `/API/health`) returns a plain-text `404 Not found.` document, not the SPA shell and not the Inventory page.
- An unknown static or metadata path (for example `/assets/nope.js`, `/robots.txt`, `/llms.txt` or `/ai-catalog.json`) returns 404 rather than the SPA shell, because the build does not ship that file.
- `GET /api`, `GET /api/` and every unknown `/api/*` route return JSON `route_not_found` with HTTP 404; a wrong method on a real API route keeps its 405.

## Provenance and unknown values

- Without `source`/`sourceUrl`, imported values cite `source: "user"` with `reference: "csv:<filename>"` and the observation timestamp.
- `source` is `user` or `catalog:<id>`, where `<id>` names the dataset and revision, for example `catalog:buildcores@a1b2c3d` or `catalog:manufacturer@2026-10-31`. Any other value is a `source_format` field error.
- `sourceUrl` is an absolute `http://` or `https://` URL. It becomes the provenance reference for the catalog facts; without it the CSV filename is used. A value that is not an absolute HTTP(S) URL is `source_url_format`.
- A declared catalog source applies to the `model` and every parsed specification. `sku`, `quantity`, `price` and `currency` stay `user` facts, because a catalog supplies part facts, not an operator's stock key or price.
- Correcting a row's model or specifications replaces that field's catalog claim with `source: "user"` and `reference: "correction:<snapshotId>"`. An unedited field keeps its catalog claim.
- Lexical normalization cites `source: "rigcraft-normalizer"` (`category-alias-v1` or `model-lexical-v1`).
- `match.confidence` is `catalog-declared` when a row declares a catalog source or an operator applies stored catalog proposals, and `unverified` otherwise. Neither value means RigCraft queried or verified the source; the basis says so.
- Stored catalog proposals add only missing fields with `catalog:<dataset-name>@<version>` provenance and the stored part's source URL. The row's SKU, quantity, price and currency never change, and an operator-supplied specification wins over a conflicting catalog value.
- Known numeric spec keys require units (`tdp`, `power_draw`, `wattage` in W; `length`, `height`, `width`, `depth` in mm; `capacity`, `vram` in GB; `rpm` in RPM). Unknown spec keys are retained with `status: "unknown"`; absent specs are shown as unknown.

## Shipped sample

The sample is one physically buildable AM5 machine: AMD Ryzen 7 7800X3D, ASUS ROG STRIX B650E-F GAMING WIFI, Kingston FURY Beast 32GB (2x16GB) DDR5-6000 CL36, NVIDIA GeForce RTX 4070 Founders Edition, Corsair RM750e, Corsair 4000D AIRFLOW, Thermalright Peerless Assassin 120 SE and three Corsair MP600 PRO NH drives. Every compatibility fact was checked on 2026-10-31 against the manufacturer page recorded in that row's `sourceUrl`; `source` is `catalog:manufacturer@2026-10-31`. The pages do not publish BIOS support or peak/transient power, so those facts are absent and stay unknown. Quantity and price are operator-entered sample stock values, not market or catalog data.

The sample rows are CC BY 4.0 with attribution in `NOTICE`. The wider data-source assessment, including why the much larger BuildCores OpenDB is not vendored, is in `docs/data-sources.md`. To import a local OpenDB checkout, use `scripts/opendb-to-csv.mjs` as documented there.

## Web states

`/inventory` covers empty, selecting, parsing, review, row correction, validation error, duplicate resolution, commit pending, committed, retry and earlier-revision states. On first load the newest snapshot opens automatically through the same path as an explicit `View`, so export is ready without a hidden step; the automatic open announces politely and does not move focus, and `Start a new import` closes it for good. An open snapshot leads with a summary bar (revision, records, units in stock, stock value, latest/earlier chip, source file and commit time), and the history is a card grid with the open revision marked `Viewing now`. `Preview CSV` is disabled with an `aria-describedby` reason until a file and a three-letter currency are present; the handler and the service validation remain authoritative. The flow uses labelled controls, a live status region, `role="alert"` errors, visible focus, non-color-only badges with symbols, stacked mobile table cards and reduced-motion-safe CSS. Focus moves to the review heading, the snapshot heading, or the error alert after async transitions.

Each specification is labelled `Operator-confirmed` (✎), `Catalog` (▣) or `Unknown` (?) from its recorded provenance, and every preview and snapshot row names whether its facts are catalog facts or operator-entered. The download area states the sample's CC BY 4.0 licence, the 2026-10-31 verification date and that prices are operator-entered.

Each preview row also shows its catalog status: matched (with the proposals that would be added and a per-row opt-in), ambiguous (candidates and no default) or unmatched. After commit, an applied field shows the `Catalog` badge and the catalog provenance entry. The catalog contract, format and matching order are in [catalog-ingestion.md](catalog-ingestion.md).

The `/inventory` page also has an **Export and backup** panel: CSV and JSON export of the selected snapshot, a whole-database SQLite download, and a restore that must be inspected and then confirmed by typing `RESTORE`, with a pre-restore safety copy named on success. See [docs/data-portability.md](data-portability.md).

## Evidence

- `docs/assets/inventory-*.png` — real browser states at 1440×900 and 390×844.
- `docs/evidence/inventory-import-browser.md` — capture commands, keyboard checks, measured widths and privacy review.
- `docs/evidence/token-contrast.json` — measured WCAG 2.2 AA contrast for every token pair the UI uses, produced by `node scripts/check-contrast.mjs`.
- `docs/evidence/inventory-lighthouse/report.json` and `docs/evidence/inventory-lighthouse-mobile/report.json` — Lighthouse accessibility score 100 with no failing accessibility audits.

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

The SQLite file is `inventory.sqlite3`. Schema version is tracked in `PRAGMA user_version` (currently `1`). CSV contents and cost values are not logged; `tower-http` request tracing records method/URI only at debug level.

## HTTP contract

All JSON uses camelCase. Errors use:

```json
{ "error": { "code": "mapping_missing", "message": "...", "field": "category", "row": 4, "details": {} } }
```

### `POST /api/imports`

`multipart/form-data` with `file` (UTF-8 CSV, ≤ 10 MiB), `currency` (ISO 4217) and optional `mapping` (JSON object of logical field to column header).

Logical fields: `sku`, `category`, `model`, `quantity`, `price`, `currency`, `specs`, `cost`. Required: the first five. Header aliases are detected case-insensitively; an explicit mapping wins. Ambiguous aliases require an explicit choice.

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

Stable 400 codes include `file_too_large`, `not_utf8`, `empty_file`, `no_data_rows`, `csv_headers_missing`, `csv_parse_error`, `mapping_invalid`, `mapping_missing`, `mapping_duplicate`, `ambiguous_column`, `currency_format`. Previews expire after 24 hours.

### `POST /api/imports/{previewId}/commit`

```json
{
  "decisions": [{ "row": 4, "action": "accept|exclude", "overrides": { "quantity": "3" } }],
  "duplicateResolutions": [{ "groupId": "dup-1", "action": "merge|replace|keep_separate" }]
}
```

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

Money is an integer in minor units with one currency per batch. `404 snapshot_not_found` for unknown IDs.

### `POST /api/inventory/snapshots/{id}/corrections`

```json
{
  "note": "Stock recount",
  "changes": [{ "recordId": "…", "quantity": 2, "priceMinor": 18950 }]
}
```

Creates a new snapshot with `revision + 1`, `parentSnapshotId` set to the old ID, and new provenance for changed fields. The old snapshot is never edited. Supported changes: `quantity`, `priceMinor`, `sku`, `category`, `model`, `specs`. Unknown records return 404; an empty change set returns `422 no_changes`.

### Other routes

- `GET /api/imports/sample` — sample UTF-8 CSV download.
- `GET /api/health` — service health.
- `GET /inventory` — Web flow; the Rust process serves the built SPA with an index fallback.
- Unknown `/api/*` routes return JSON `route_not_found`.

## Provenance and unknown values

- Imported values cite `source: "user"` with `reference: "csv:<filename>"` and the observation timestamp.
- Lexical normalization cites `source: "rigcraft-normalizer"` (`category-alias-v1` or `model-lexical-v1`).
- Corrections cite `source: "user"` with `reference: "correction:<snapshotId>"`.
- No hardware catalog is consulted. `match.confidence` is always `unverified` in this package, so a later catalog adapter must replace it rather than infer from a similar name.
- Known numeric spec keys require units (`tdp`, `power_draw`, `wattage` in W; `length`, `height`, `width`, `depth` in mm; `capacity`, `vram` in GB; `rpm` in RPM). Unknown spec keys are retained with `status: "unknown"`; absent specs are shown as unknown.

## Web states

`/inventory` covers empty, selecting, parsing, review, row correction, validation error, duplicate resolution, commit pending, committed, retry and earlier-revision states. The flow uses labelled controls, a live status region, `role="alert"` errors, visible focus, non-color-only badges with symbols, stacked mobile table cards and reduced-motion-safe CSS. Focus moves to the review heading, the snapshot heading, or the error alert after async transitions.

## Evidence

- `docs/assets/inventory-*.png` — real browser states at 1440×900 and 390×844.
- `docs/evidence/inventory-import-browser.md` — capture commands, keyboard checks, measured widths and privacy review.
- `docs/evidence/token-contrast.json` — measured WCAG 2.2 AA contrast for every token pair the UI uses, produced by `node scripts/check-contrast.mjs`.
- `docs/evidence/inventory-lighthouse/report.json` and `docs/evidence/inventory-lighthouse-mobile/report.json` — Lighthouse accessibility score 100 with no failing accessibility audits.

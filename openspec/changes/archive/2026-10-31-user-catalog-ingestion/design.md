# Design: Operator-supplied catalog ingestion and enrichment

## Context

The service owns one SQLite connection (`Mutex<rusqlite::Connection>`) and a
forward-only migration at `PRAGMA user_version = 4`. Inventory rows already carry
per-field provenance and a declared `catalog:<id>` source. This change adds a
catalog that is *stored*, not merely declared, so matching and enrichment can use
it.

## Decision 1 — one versioned document, validated before any write

```json
{
  "format": "rigcraft-catalog",
  "version": 1,
  "dataset": {
    "name": "buildcores-opendb",
    "sourceUrl": "https://github.com/buildcores/buildcores-open-db",
    "commit": "d8d0149dbb32f75e9408d0476b231984c1fc2619",
    "license": "CC BY-NC-SA 4.0",
    "retrievedAt": "2026-10-31T06:30:00Z"
  },
  "parts": [
    {
      "externalId": "amd-ryzen-7-7800x3d",
      "sku": "100-100000910WOF",
      "category": "cpu",
      "model": "AMD Ryzen 7 7800X3D",
      "manufacturer": "AMD",
      "specs": { "socket": "AM5", "tdp": "120W", "igpu": "yes" },
      "sourceUrl": "https://www.amd.com/en/products/cpu/amd-ryzen-7-7800x3d",
      "verifiedAt": "2026-10-31T00:00:00Z"
    }
  ]
}
```

The request body is `{ "acknowledged": true, "catalog": <document> }`. The boolean
must be literally `true`; it is not inferred from uploading a file.

Validation parses the body as an untyped JSON value first, so a bad row produces a
row-scoped error instead of a generic extractor rejection. Rules:

- `format` must be `rigcraft-catalog`; `version` must be `1`. Anything else is a
  400 with `catalog_format_unsupported` or `catalog_version_unsupported`.
- `dataset.name` is a token `^[A-Za-z0-9][A-Za-z0-9._@+/-]{0,119}$`, because it is
  embedded in `catalog:<name>@<version>` and may later be used in an import
  `source` column. `dataset.license` is required and stored verbatim.
  `sourceUrl` is optional and must be absolute HTTP(S). `retrievedAt` and per-part
  `verifiedAt` are optional RFC 3339. `commit` is an optional token stored
  verbatim.
- Each part requires `sku`, `category`, `model`. `sku` ≤120 characters, `model`
  ≤200, no control characters; `category` must normalize through the existing
  alias table; `specs` keys must be the engine's own keys after the existing
  key normalization, and each value must parse through the existing `parse_specs`
  (so a numeric key without its unit, or with the wrong unit, is refused rather
  than guessed). Unknown keys are refused with the accepted list.
- Duplicate `sku` (normalized case-insensitively) or duplicate `externalId` within
  one document is an error whose detail names both row indexes.
- Any error refuses the whole document. Nothing is written for a refused document.
  The 422 `catalog_invalid` envelope carries `details.rows` with
  `{row, field, code, message, suggestion?}` and `details.specKeys`.

Row indexes are zero-based array positions; the message also names the human row
number (`row index 0` is the first part) so the UI can render either without
guessing.

## Decision 2 — storage in schema 5

Two tables, added by a forward migration that leaves schema 4 rows untouched:

```sql
CREATE TABLE catalog_datasets (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL UNIQUE,
  source_url TEXT,
  commit_ref TEXT,
  license TEXT NOT NULL,
  retrieved_at TEXT,
  imported_at TEXT NOT NULL,
  version_label TEXT NOT NULL,
  row_count INTEGER NOT NULL,
  category_counts_json TEXT NOT NULL,
  active INTEGER NOT NULL
);
CREATE TABLE catalog_parts (
  id TEXT PRIMARY KEY,
  dataset_id TEXT NOT NULL REFERENCES catalog_datasets(id) ON DELETE CASCADE,
  position INTEGER NOT NULL,
  external_id TEXT,
  sku TEXT NOT NULL,
  category TEXT NOT NULL,
  model TEXT NOT NULL,
  model_normalized TEXT NOT NULL,
  manufacturer TEXT,
  specs_json TEXT NOT NULL,
  source_url TEXT,
  verified_at TEXT
);
```

`version_label` is `commit`, else `retrievedAt`, else the import date. The
provenance source is `catalog:<name>@<version_label>`. Part specs are stored as
the engine's `Spec` values with that provenance, so a stored part and an enriched
inventory field use exactly the same representation as a CSV import.

Re-import is one transaction: update the dataset row, delete its parts, insert the
new parts. The `active` flag survives a re-import. Another dataset is untouched.
Delete is one `DELETE` on the dataset; `ON DELETE CASCADE` removes only its parts.
No inventory table references a catalog part, so delete cannot touch inventory,
snapshots, checks, recommendations or diagnoses.

## Decision 3 — matching order and ambiguity

Only `active = 1` datasets participate. A row's category (already normalized by
the importer) scopes every lookup:

1. exact normalized `sku` (the existing `normalize_sku`);
2. exact normalized `externalId` (MPN) against the row's SKU;
3. exact normalized `model` (the existing `normalize_model`) inside the category.

The first basis with any candidate wins. One candidate is `matched`, more than one
is `ambiguous`, none is `unmatched`. `ambiguous` returns every candidate and no
proposals; the operator has to decide, exactly as the diagnosis matcher refuses to
pick a default.

## Decision 4 — proposals and conflicts

For a matched part, compare against the operator's row:

- A proposal exists only for a field the row leaves empty: an empty model, or a
  specification key the row does not have. Existing keys are never proposed.
- A conflict is a field both sides supply with different normalized values. It is
  reported and never applied. This includes the model and each specification key.
- The catalog never supplies `sku`, `quantity`, `price`, `currency` or `category`
  as a proposal: those are the operator's identity and stock facts, and the row's
  category already drove the lookup.

## Decision 5 — per-row opt-in on commit

`RowDecision` gains an optional `catalog: { "partId": "…" }`. The commit handler
loads those parts once and passes them into `build_records`, which applies only
the proposals for the row's effective values (after any operator edit). Applied
fields then carry:

- source `catalog:<dataset-name>@<version_label>`;
- reference: the part's `sourceUrl`, else the dataset's `sourceUrl`, else
  `catalog:<name>`;
- observed time: the part's `verifiedAt`, else the dataset's `retrievedAt`, else
  the import time.

The record's `match.confidence` becomes `catalog-declared` with a basis saying the
match was used for empty fields only and RigCraft did not verify the source. When
the row already had some specifications and only extra keys were added, the added
`Spec` values carry the catalog provenance and the field-level `specs` provenance
stays with the operator, because the field is mixed. A clean row (no specs
supplied) gets the catalog provenance for the `specs` field itself.

Operators win by construction: only missing fields are filled, and conflicts are
never written.

## Decision 6 — search ranking

`GET /api/catalog/search` ranks deterministically:

1. exact SKU or exact external ID match (`rank 0`);
2. model prefix match (`rank 1`);
3. token overlap > 0 between the query and SKU/external ID/model (`rank 2`).

Ties break on overlap count descending, then SKU, then model, then part ID.
Paging is `limit` (default 20, maximum 100) plus `offset`. Each hit names its
dataset, licence and version. Inactive datasets are excluded. `q` is required.

## Decision 7 — interface

`/catalog` is a new route in the same SPA. It owns upload, the validation report,
the registry and search. Removal is two-step: the first button opens an inline
confirmation naming the dataset and part count and stating that inventory is not
affected; the second performs `DELETE …?confirm=remove`.

`/inventory` fetches matches for the preview rows in one batch and renders a
catalog status per row. A matched row with proposals gets a checkbox; the commit
request includes the opt-in only when checked. Added fields are shown as
"will add" before commit and as catalog-sourced badges plus provenance after
commit.

## Rejected alternatives

- Vendoring a dataset, or fetching one at runtime: ownership and licence are the
  operator's, and an MIT repository must not redistribute CC BY-NC-SA data.
- Parsing the document with `serde` structs into the extractor: a missing row
  field would become one generic `invalid_json` instead of a row-scoped reason.
- Reusing the CSV `source` column for catalog application: one source for the
  whole row would mislabel a model the operator typed when only specifications
  were filled.
- Auto-resolving conflicts: the operator's value is the stock truth; a catalog is
  a claim.

## Verification

Unit tests for document validation, ranking and matching; HTTP tests for every
required refusal, replace, delete, activation, enrichment and schema migration;
Vitest for the catalog page and import opt-in; Playwright for upload, refusal,
search, enrichment and committed provenance; Lighthouse on `/catalog`.

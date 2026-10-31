# Operator-supplied catalog ingestion

Status: implemented in `user-catalog-ingestion` (2026-11-01).

RigCraft does not ship, fetch or redistribute a hardware catalog. This document
defines the one catalog document an operator may upload into an instance, how it is
validated and stored, and how search, matching and import enrichment use it. The
data and its licence stay the operator's.

The ten verified rows served by `/api/imports/sample` remain the only data shipped
with RigCraft. They are a sample, not a catalog.

## The document

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

| Field | Required | Rule |
| --- | --- | --- |
| `format` | yes | exactly `rigcraft-catalog` |
| `version` | yes | exactly `1` |
| `dataset.name` | yes | a token, `[A-Za-z0-9][A-Za-z0-9._@+/-]{0,119}`; it is embedded in provenance |
| `dataset.license` | yes | non-empty, stored and displayed verbatim |
| `dataset.sourceUrl` | no | absolute `http://` or `https://` URL |
| `dataset.commit` | no | stored verbatim; becomes the version label when present |
| `dataset.retrievedAt` | no | RFC 3339 timestamp |
| `parts` | yes | at least one part |
| `parts[].sku` | yes | 1–120 characters, no control characters |
| `parts[].category` | yes | one of `cpu`, `motherboard`, `ram`, `gpu`, `psu`, `case`, `cooler`, `storage` (the existing alias table is applied) |
| `parts[].model` | yes | 1–200 characters, no control characters |
| `parts[].externalId` | no | the MPN; 1–120 characters, unique within the document |
| `parts[].manufacturer` | no | 1–200 characters |
| `parts[].specs` | no | object of engine specification key to string value |
| `parts[].sourceUrl` | no | absolute `http://` or `https://` URL |
| `parts[].verifiedAt` | no | RFC 3339 timestamp |

Specification keys are the engine's own keys (`socket`, `tdp`, `length`, `wattage`,
`capacity`, `form_factor`, …), the same set the CSV importer accepts. An unknown
key is refused with the accepted list. Values are strings and are parsed by the
existing normalizer: a numeric key without its unit, or with the wrong unit, is
refused rather than guessed.

## Upload

```text
POST /api/catalog/datasets
Content-Type: application/json

{ "acknowledged": true, "catalog": { …the document… } }
```

`acknowledged` must be literally `true`: it is the operator's statement that they
have the right to use the data. A missing or false value is `400
catalog_acknowledgement_required` and nothing is read.

The whole document is validated before anything is written. Any invalid row refuses
the document with `422 catalog_invalid`; nothing is stored, and no partial import is
possible. The existing JSON envelope carries every problem:

```json
{
  "error": {
    "code": "catalog_invalid",
    "message": "The catalog document was refused…",
    "row": 1,
    "details": {
      "accepted": false,
      "rows": [
        { "row": 1, "field": "model", "code": "required", "message": "`model` is required." }
      ],
      "specKeys": ["tdp", "power_draw", "…"]
    }
  }
}
```

`row` is the zero-based array index; the message also names it in words. Duplicate
`sku` or `externalId` values produce `catalog_duplicate_sku` or
`catalog_duplicate_external_id` on **both** rows, each naming the other index. A
duplicate is refused rather than merged or dropped.

Accepted response:

```text
accepted, replaced, dataset { …registry entry… }, partCount, categoryCounts, specKeys
```

## Storage

Schema `PRAGMA user_version = 5` adds two tables; a schema 4 database migrates
forward with every existing row untouched.

- `catalog_datasets` — id, name (unique), source URL, commit, licence, retrieval
  date, import date, version label, row count, per-category counts, active flag.
- `catalog_parts` — id, dataset id, position, external ID, SKU, category, model and
  normalized model, manufacturer, parsed specifications, source URL, verification
  date.

The version label is `commit`, else `retrievedAt`, else the import date. Provenance
for a part's facts is:

```text
source:     catalog:<dataset-name>@<version>
reference:  the part's sourceUrl, else the dataset sourceUrl, else catalog:<name>
observedAt: the part's verifiedAt, else the dataset retrievedAt, else the import time
```

Re-importing the same dataset name replaces that dataset's rows in one transaction
and keeps its active flag. Another dataset is untouched. Removing a dataset deletes
only its registry row and its parts: inventory, snapshots, compatibility checks,
recommendations and diagnoses are not referenced by the catalog and are never
touched.

## Registry

| Route | Purpose |
| --- | --- |
| `GET /api/catalog/datasets` | registry with licence, version, counts and active flag |
| `PATCH /api/catalog/datasets/{id}` | `{ "active": false }` or `{ "active": true }`; only active datasets are used |
| `DELETE /api/catalog/datasets/{id}?confirm=remove` | remove a dataset; without `confirm=remove` the response is `400 catalog_confirm_required` |
| `GET /api/catalog/parts/{id}` | one stored part with its specs and field provenance |

Unknown IDs return `404 catalog_dataset_not_found` or `404 catalog_part_not_found`.

## Search

```text
GET /api/catalog/search?q=<text>&category=<category>&limit=20&offset=0
```

`q` is required (1–200 characters); `limit` is 1–100. Search covers active datasets
only. Ranking is deterministic:

1. exact SKU or exact external ID (MPN);
2. model prefix;
3. token overlap between the query and SKU/MPN/model.

Ties break on overlap count, then SKU, then model, then part ID. Each hit carries
the part and its dataset with licence and version. Paging uses `limit` and `offset`
over that fixed order.

## Matching and enrichment

```text
POST /api/catalog/match
{ "rows": [{ "row": 2, "sku": "…", "category": "cpu", "model": "…", "specs": "socket=AM5" }] }
```

One to five hundred rows. Matching is scoped to the row's normalized category and
uses active datasets only:

1. exact normalized SKU;
2. exact normalized external ID (MPN) against the row's SKU;
3. exact normalized model.

The first basis with candidates wins. The result is `matched`, `ambiguous` (all
candidates, no default) or `unmatched`. A matched part yields:

- **proposals** only for fields the row leaves empty — an empty model, or a
  specification key the row does not have;
- **conflicts** for fields both sides supply with different values, reported with
  the operator's value and the catalog's value.

`sku`, `quantity`, `price`, `currency` and `category` are never proposed. The
catalog does not supply stock facts, and the row's category already drove the
lookup.

## Applying proposals on commit

`POST /api/imports/{previewId}/commit` accepts an optional per-row opt-in:

```json
{ "row": 2, "action": "accept", "catalog": { "partId": "…" } }
```

The handler loads those parts once. Only proposals are applied, to the row's
effective values after any operator correction. Applied fields get the part's
catalog provenance; the record's `match.confidence` becomes `catalog-declared` with
a basis saying the catalog filled empty fields and RigCraft did not verify the
source.

- Operator values always win. A conflict is reported and never overwrites.
- A row that supplied no specifications gets field-level catalog provenance for
  `specs`. A row that supplied some keeps its operator field provenance, and each
  added specification carries the catalog provenance on its own `Spec`.
- `sku`, `quantity`, `price` and `currency` stay `user` facts.
- A part ID that no longer exists refuses the commit with `422
  catalog_part_not_found` before anything is written.

## The Web surface

`/catalog` uploads the document with the acknowledgement, shows the accepted report
or the refused rows with their field reasons, lists the registry with licence and
counts, activates or deactivates a dataset, removes one only after an inline
confirmation that states inventory is unaffected, and searches. A stored part opens
with its specifications and provenance.

`/inventory` shows each preview row's catalog status and, for a matched row with
proposals, a per-row "Apply catalog proposals" checkbox. Conflicts are named next
to it. After commit, added specifications show the `Catalog` badge and the
`catalog:<name>@<version>` provenance entry.

Both pages use labelled controls, a polite live region, `role="alert"` errors,
visible focus, non-colour-only badges, and a stacked narrow layout with no
horizontal overflow.

## Licensing position

The operator supplies the document and its licence. RigCraft does not verify the
licence, does not redistribute the catalog, and does not write it into the
repository, a fixture, an export template or another instance. The registry shows
the licence exactly as supplied. OpenDB remains CC BY-NC-SA 4.0 and is converted from
a local checkout by the operator; see [data-sources.md](data-sources.md).

## What never happens

- No dataset is bundled, generated from a similar model, or fetched at runtime.
- No unit is guessed: a numeric specification must carry its unit.
- No unknown category or specification key is stored as a claim.
- No partial import: one invalid row refuses the whole document.
- No duplicate is merged or silently dropped.
- No operator value is overwritten, and no conflict is auto-resolved.
- No unit, price, quantity or currency is proposed from a catalog.
- Removing or replacing a dataset never touches inventory, snapshots, checks,
  recommendations or diagnoses.

## Converting a local OpenDB checkout

The catalog converter is the companion of the CSV converter and stays an external,
operator-run step:

```sh
node scripts/opendb-to-catalog.mjs /tmp/opencode/buildcores-open-db \
  --name buildcores-opendb \
  --license "CC BY-NC-SA 4.0" \
  --out /tmp/opencode/catalog.json
```

It requires `--name` and `--license`, resolves the checkout commit unless
`--commit` is passed, and writes nothing when a part has no name or usable SKU, a
JSON file cannot be parsed, a category is unmapped, or two records share a SKU (the
catalog document refuses duplicates). It never adds a price or quantity. Upload the
result through `/catalog`; the operator is responsible for the licence obligations.

## Evidence

- `docs/evidence/catalog-ingestion-browser.md`
- `docs/assets/catalog-ingestion-*.png`
- `docs/evidence/catalog-ingestion-lighthouse/` and `-mobile/`

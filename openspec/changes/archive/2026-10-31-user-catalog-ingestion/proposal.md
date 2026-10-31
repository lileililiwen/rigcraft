# Proposal: Operator-supplied catalog ingestion and enrichment

## Why

RigCraft ships ten verified sample rows and can convert a local BuildCores OpenDB
checkout into an inventory CSV. Both stop short of the product need: a shop that
owns or is licensed to use a hardware catalog has no way to load it into this
instance, and the import preview cannot use it to fill facts the operator's row
leaves out. The catalog itself is not RigCraft's asset. Vendoring a third-party
dataset would make an MIT repository a redistribution channel for someone else's
licence, and the operator would still have no entry point.

This change ships the mechanism, not the data: a documented versioned document,
strict validation, storage in this instance's SQLite database with per-field
provenance, and use of that stored catalog by search, matching and import
enrichment. The operator supplies the document and its licence.

## What Changes

- Define the `rigcraft-catalog` version 1 JSON document with a dataset header
  (name, optional source URL, optional commit, licence, optional retrieval date)
  and parts (SKU, category, model, optional external ID/manufacturer/specs/source
  URL/verification date).
- Add `POST /api/catalog/datasets` with a required boolean acknowledgement. It
  validates the whole document before writing anything, refuses any structurally
  invalid row as a whole, names the row index and field, reports duplicate SKUs or
  external IDs by naming both rows, and rejects unknown specification keys with
  the accepted list.
- Add a dataset registry and parts storage in SQLite schema 5 through a forward
  migration that preserves schema 4 data. Re-importing the same dataset name
  replaces that dataset's rows atomically; another dataset coexists. Deleting a
  dataset never touches inventory, snapshots, checks, recommendations or
  diagnoses.
- Add `GET /api/catalog/datasets`, `PATCH /api/catalog/datasets/{id}`,
  `DELETE /api/catalog/datasets/{id}`, `GET /api/catalog/search`,
  `GET /api/catalog/parts/{id}` and `POST /api/catalog/match`.
- Only active datasets take part in matching. Matching tries exact SKU, then MPN
  (the part's external ID), then the exact normalized model inside the row's
  category. It returns `matched`, `ambiguous` (candidates, no default) or
  `unmatched`. A match proposes only the fields the operator's row is missing and
  reports conflicts without resolving them.
- Extend the commit contract with a per-row catalog opt-in. Applied proposals take
  provenance `catalog:<dataset-name>@<version>` and the part's source URL as the
  reference. Operator values always win.
- Add a `/catalog` route for upload, the validation report, the registry with
  licence and counts, activation, guarded removal and search. On `/inventory`,
  show each preview row's catalog status, let the operator apply proposals per row
  before commit, and mark the fields that came from the catalog.
- Add `docs/catalog-ingestion.md`, update the README routes and the import
  contract, and add `scripts/opendb-to-catalog.mjs` as an operator-run companion
  to the existing CSV converter. The conversion stays an external step.

## Package Boundary and Split Assessment

Single outcome: an operator can load a catalog they are entitled to use, and this
instance can search it and enrich an import from it. Ingestion, storage, search
and enrichment share one document, one registry and one storage transaction;
shipping them separately would ship a catalog that nothing consumes. The change
adds no dataset, no network fetch, no scraper and no new dependency. Compatibility,
recommendation, diagnosis and BYOK behaviour are unchanged.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence | Compatibility and ownership | Decision |
| --- | --- | --- | --- |
| `serde_json` and `uuid` | `Cargo.toml` | Already direct dependencies; the document is plain JSON and identifiers already use UUIDs. | reuse, no new crate |
| `rusqlite` | `Cargo.toml`, `src/store.rs` | Already owns the single connection and forward migrations; schema 5 is one more forward step. | extend the existing store |
| Existing normalizer | `src/domain/specs.rs`, `src/domain/mod.rs` | Owns the category aliases and specification keys/units; reusing it means no second vocabulary. | reuse |
| Forge Rust platform libraries | `../forge/modules/rust-platform-libs/Cargo.toml`, `forge.yaml` | The profile declares zero consumption and no crate owns catalog ingestion or matching. | keep local |
| Pinned design tokens | `apps/web/src/styles/vendor/tokens.css`, `.project.json` | Already vendored and pinned; the new page reuses measured tokens and adds no colour. | reuse |

## User Experience and Interface Impact

Actors: the shop operator who uploads and manages a catalog, and the same operator
reviewing an import. Surfaces: a new `/catalog` page (file choice, acknowledgement,
validation report, registry, activation, guarded removal, search, part detail) and
the existing `/inventory` review (catalog status per row, per-row opt-in, conflicts,
added fields labelled catalog-sourced). States: no catalog, uploading, accepted with
counts, refused with per-row reasons, upload error, registry empty, active,
inactive, removal confirmation, search empty and search results. Focus moves to
the report heading, the registry heading and the error alert; the live region
announces each transition; destructive removal needs an explicit confirmation step;
keyboard order reaches every control and the 390 px layout has no horizontal
overflow.

## BFS Impact Map

Rust: new `catalog` module (document validation, report, ranking, matching) and
catalog persistence beside `store` (schema 5 migration, registry, parts, atomic
replace, delete); `api` (seven catalog routes and catalog-aware commit); `commit`
(per-row opt-in and applied provenance); `domain::specs` (expose accepted keys).
TypeScript: `types.ts` catalog shapes, `api.ts` catalog helpers, new `CatalogPage`,
catalog status in `PreviewReview`, opt-in state in `InventoryPage`, route and header.
Tests: Rust unit plus HTTP suite, Vitest, Playwright. Docs: new contract, README,
import contract, data-source pointer, converter, capture plan, evidence. SQLite
schema moves from 4 to 5.

## Capabilities

- `catalog-ingestion` (added): the documented document, validation, storage,
  registry, search, matching and enrichment, and the licensing position.
- `catalog-seed` (modified): the data-source position now also states that an
  operator-supplied catalog is ingested at runtime and never redistributed.
- `inventory-import` (modified): the commit contract and provenance rules for
  catalog-applied proposals.

## Non-goals

Shipping or fetching any third-party dataset, scraping, runtime download, a
background import job, catalog editing in the browser, unit guessing, automatic
conflict resolution, overwriting operator values, partial import, changes to
compatibility rules, recommendation scoring, diagnosis advice or the BYOK panel.

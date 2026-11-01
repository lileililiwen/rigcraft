# catalog-ingestion Specification

## Purpose
Define the operator-supplied catalog document, its strict validation, local storage, registry, search, three-state matching and import enrichment, and the position that the data and its licence are never RigCraft's.
## Requirements
### Requirement: One documented, versioned catalog document
The system SHALL accept a JSON catalog document identified by
`"format": "rigcraft-catalog"` and `"version": 1`, with a `dataset` header
(`name`, optional `sourceUrl`, optional `commit`, required `license`, optional
`retrievedAt`) and a non-empty `parts` array. Each part SHALL carry required `sku`,
`category` and `model`, and optional `externalId`, `manufacturer`, `specs`,
`sourceUrl` and `verifiedAt`. `specs` SHALL be an object whose keys are the
engine's own specification keys and whose values are strings.

The upload request SHALL contain a boolean acknowledgement that the operator has
the right to use the data, and a false or missing acknowledgement SHALL be refused.

#### Scenario: Accept a valid document and report its counts
Given a document with a valid dataset header and parts, when it is uploaded with the acknowledgement, then the response is accepted and reports the dataset name, licence, row count and per-category counts.

#### Scenario: Refuse an unknown format or version
Given a document whose `format` is not `rigcraft-catalog` or whose `version` is not `1`, when it is uploaded, then the response is 400 `catalog_format_unsupported` or `catalog_version_unsupported` and no dataset is stored.

#### Scenario: Require the licence acknowledgement
Given a valid document uploaded without `acknowledged: true`, when it is submitted, then the response is 400 `catalog_acknowledgement_required` and no dataset is stored.

### Requirement: Validation refuses a bad document as a whole
The system SHALL validate the entire document before writing anything. A
structurally invalid row SHALL name the row index and field with a reason in the
existing JSON error envelope, and the document SHALL be refused with no partial
import. A duplicate `sku` or duplicate `externalId` within one document SHALL be an
error whose details name both row indexes. A `category` outside the supported set
SHALL be refused with the accepted categories. A specification key outside the
engine's key set SHALL be refused with the accepted keys, and a specification value
SHALL be parsed by the existing normalizer with no unit guessing.

#### Scenario: Refuse an invalid row and store nothing
Given a document whose second part has an empty model, when it is uploaded, then the response is 422 `catalog_invalid`, `details.rows` names row index 1 and field `model`, and `GET /api/catalog/datasets` is still empty.

#### Scenario: Name both duplicate rows
Given two parts that share a SKU, when the document is uploaded, then the refusal names both row indexes and no dataset is stored.

#### Scenario: Reject an unknown specification key with the accepted list
Given a part with `specs: { "socket_size": "AM5" }`, when it is uploaded, then the refusal names the row and `specs.socket_size` and lists the accepted keys.

#### Scenario: Reject a numeric specification without its unit
Given a part whose `tdp` value is `120` with no unit, when it is uploaded, then the row is refused with the normalizer's unit error rather than a guessed unit.

### Requirement: An operator-supplied catalog is stored in this instance with provenance
The system SHALL store accepted datasets and their parts in this instance's SQLite
database with per-field provenance, in a forward schema migration that preserves
existing data. It SHALL store the licence and source URL verbatim and expose them.
A dataset SHALL carry a name, optional source URL and commit, licence, optional
retrieval date, import date, row count, per-category counts, a version label and an
active flag. A part SHALL be keyed by its dataset and SHALL expose its SKU, category,
model, manufacturer, specifications, source URL and verification date.

#### Scenario: Re-import replaces one dataset atomically
Given two stored datasets, when one name is uploaded again with different rows, then that dataset's row count and parts are replaced in one transaction and the other dataset is unchanged.

#### Scenario: Removing a dataset leaves inventory untouched
Given a committed inventory snapshot and a stored dataset, when the dataset is removed, then the snapshot, its records and its provenance are unchanged.

#### Scenario: Schema 4 data migrates to schema 5 intact
Given a schema 4 database with a committed snapshot, when the service opens it, then `user_version` is 5 and the snapshot and its records are read back unchanged.

### Requirement: Only active datasets are used, and the registry is visible
The system SHALL list datasets with their licence, version and counts, allow a
dataset to be activated or deactivated, and allow removal only after an explicit
confirmation. Matching, search and enrichment SHALL use only active datasets.

#### Scenario: Deactivating removes a dataset from matching
Given an active dataset whose part matches an import row, when the dataset is deactivated, then the same row matches nothing and no proposals are offered.

#### Scenario: Removal requires confirmation
Given a stored dataset, when `DELETE` is called without the confirmation value, then the response is 400 `catalog_confirm_required` and the dataset remains.

### Requirement: Search ranks deterministically and names the source dataset
`GET /api/catalog/search` SHALL require a query, accept an optional category and
paging, and rank exact SKU or external-ID matches first, model-prefix matches
second, and token-overlap matches third, breaking ties deterministically. Every hit
SHALL name the dataset, its licence and its version.

#### Scenario: Exact identity ranks first
Given an active dataset containing a part whose SKU equals the query and another part that merely shares a token, when the search runs, then the exact-SKU hit is first and repeats identically on a second request.

#### Scenario: Paging is stable
Given more hits than the requested limit, when the next page is requested with an offset, then the pages do not overlap and are ordered by the same ranking.

### Requirement: Matching is three-state and enrichment never overwrites the operator
For an import row the system SHALL look up an active part by exact SKU, then by
external ID (MPN), then by exact normalized model inside the row's category. It
SHALL return `matched`, `ambiguous` with candidates and no default, or `unmatched`.
A match SHALL propose only fields the operator's row leaves empty and SHALL report
values that conflict with the operator's without resolving them. A per-row
commit opt-in SHALL apply only the proposals, and applied fields SHALL carry
`catalog:<dataset-name>@<version>` provenance with the part's source URL as the
reference. Operator values SHALL always win.

#### Scenario: Exact SKU is a match
Given an active part whose SKU equals the row's SKU in the same category, when the row is matched, then the status is `matched` and the candidate names its dataset.

#### Scenario: Two plausible parts are ambiguous
Given two active parts that match the same row model in the category, when the row is matched, then the status is `ambiguous`, both candidates are returned, and no proposals are offered.

#### Scenario: An unrelated model is unmatched
Given no active part shares the row's SKU, external ID or normalized model, when the row is matched, then the status is `unmatched` and no proposals are offered.

#### Scenario: Proposals fill only missing fields
Given a row that supplies a model but no specifications, when it is matched to a catalog part with specs, then only specification proposals are returned and the supplied model is not proposed.

#### Scenario: A conflict is reported, not applied
Given a row that supplies `socket=AM4` and a catalog part that records `socket=AM5`, when it is matched and committed with the opt-in, then the conflict is reported, the committed record still has `socket=AM4`, and its provenance remains the operator's.

#### Scenario: Applied fields carry catalog provenance
Given a row with no specifications and an opt-in for a matched part, when the import is committed, then the added specifications cite `catalog:<dataset-name>@<version>` with the part's source URL, while the row's SKU, quantity and price stay `user` facts.

### Requirement: The catalog page is usable, accessible and self-hosted
The `/catalog` route SHALL let the operator choose and upload the document with the
acknowledgement, show the accepted report or the refused rows with their reasons,
list the registry with licence and counts, activate or deactivate a dataset,
remove one only through a confirmation step, and search. The registry and the
search results SHALL be presented as dense console tables that name, per row, the
dataset and its provenance chip, the licence, the version, the part and category
counts, the source URL and the active state, and the registry SHALL summarise the
dataset, part and active counts. Search SHALL keep its rank order and the dataset
attribution of every hit and SHALL be disabled, with an accessible reason, while
the query is empty. The page SHALL use labelled controls, visible focus, live status
announcements, non-colour-only statuses, a desktop layout and a stacked narrow
layout. The `/inventory` review SHALL show each row's catalog status and the
per-row opt-in, and SHALL mark catalog-applied fields.

#### Scenario: Upload and see the registry
Given a small valid document, when it is uploaded through `/catalog`, then the page shows the accepted report and the dataset in the registry with its licence, version, counts, source URL and active state.

#### Scenario: Refused document shows the reason
Given a document with an invalid row, when it is uploaded, then the page shows the refused state with the row index and field reason and the registry is unchanged.

#### Scenario: Apply a proposal on the import review
Given a preview row that matches an active dataset and has a missing specification, when the operator checks the per-row opt-in and commits, then the committed snapshot shows the added specification with catalog provenance and the operator's own values unchanged.

#### Scenario: The empty query cannot be submitted
Given an empty search field, when the search panel renders, then `Search catalog` is disabled and its reason names the query, and a non-empty query enables it.


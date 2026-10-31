# Catalog seed deltas

## ADDED Requirements

### Requirement: An operator-supplied catalog is ingested locally and never redistributed
The repository SHALL NOT vendor any third-party hardware dataset to support catalog
ingestion. A catalog SHALL enter an instance only through an operator upload that
carries the operator's acknowledgement and the dataset's licence. `docs/catalog-ingestion.md`
SHALL state that the data and its licence are the operator's, that RigCraft does not
redistribute the catalog, and that the shipped ten-row sample remains the only
shipped data. A catalog supplied to one instance SHALL NOT be written to the
repository, a fixture, an export template or another instance automatically.

#### Scenario: No dataset is shipped by this change
Given the repository tree after this change, when it is inspected, then it contains no third-party catalog dataset and every test fixture is a small synthetic document.

#### Scenario: The licence is the operator's
Given an accepted dataset, when its registry entry is displayed, then it shows the licence exactly as the operator supplied it and a statement that RigCraft neither verified nor redistributed the data.

## MODIFIED Requirements

### Requirement: The OpenDB converter maps only recorded facts and fails loudly
The repository SHALL provide `scripts/opendb-to-csv.mjs`, a Node script with no third-party dependency, that reads a local BuildCores OpenDB checkout and writes the CSV accepted by `POST /api/imports`, and a companion `scripts/opendb-to-catalog.mjs` that converts the same checkout into the `rigcraft-catalog` version 1 document. Both SHALL map CPU, Motherboard, RAM, GPU, PSU, PCCase, CPUCooler and Storage to `cpu`, `motherboard`, `ram`, `gpu`, `psu`, `case`, `cooler` and `storage`. The CSV converter SHALL record `source=catalog:buildcores@<commit>` using the checkout's commit SHA, and `sourceUrl` from `general_product_information.manufacturer_url` when that field is present. The catalog converter SHALL require the operator's dataset name and licence.

Both converters SHALL emit a specification only when OpenDB records it. A set-valued field SHALL be reduced to a single required value only when the set has one unambiguous value; otherwise it SHALL be omitted. They SHALL exit non-zero and name the file and field when a requested category is unmapped, a part has no name, a part has no usable SKU, a JSON file cannot be parsed, or (for the CSV converter) the operator-supplied quantity or price is missing. They SHALL NOT invent a price or quantity from catalog data. Neither conversion is part of the running service: the operator runs it locally and uploads or imports the result.

For the CSV converter, two catalog records that share an SKU SHALL both be emitted, with a warning naming both files, so the import preview's explicit duplicate decision resolves them rather than the converter silently merging or dropping one. For the catalog converter, a shared SKU SHALL be refused with both files named and no document written, because the catalog document refuses duplicates rather than merging them. A manufacturer URL that is not an absolute HTTP(S) URL SHALL be omitted with a warning rather than emitted as an invalid value. An MPN longer than the contract's SKU limit SHALL be replaced by the record's `opendb_id` with a warning, so the row stays traceable. A model name longer than the contract allows SHALL be emitted unchanged with a warning by the CSV converter, where the preview reports the field error; the catalog converter SHALL refuse it, naming the file, because the catalog document cannot carry it.

#### Scenario: Convert a checkout
Given a local OpenDB checkout with a resolvable commit, when the CSV converter runs with a price and quantity, then the output CSV imports through `POST /api/imports` with `catalog:buildcores@<commit>` provenance and a manufacturer URL per row that has one.

#### Scenario: Refuse an unmapped category
Given a request that names a category with no mapping, when either converter runs, then it exits non-zero and names the category.

#### Scenario: Refuse to guess a singular fact
Given a cooler that records several supported sockets, when the converter emits its row, then no `cooler_socket` specification is emitted and the compatibility rule reports `unknown`.

#### Scenario: Refuse a part with no identity
Given a part with no name or no usable SKU, when the converter runs, then it exits non-zero naming the file and the missing field, and writes no partial CSV or catalog.

#### Scenario: Surface a shared SKU without merging it
Given two OpenDB records with the same MPN, when the CSV converter runs, then both rows are written, the warning names both files, and the import preview reports them as a duplicate group for an explicit decision.

#### Scenario: Do not emit an unusable source URL
Given a record whose manufacturer URL is not absolute HTTP(S), when the converter runs, then the row is written with an empty `sourceUrl` (CSV) or no `sourceUrl` (catalog) and a warning names the file.

#### Scenario: Convert a checkout into the catalog document
Given a local OpenDB checkout with a resolvable commit, when the catalog converter runs with a name and licence, then its output is a valid `rigcraft-catalog` document whose parts carry the mapped category, specifications and manufacturer URL.

#### Scenario: Refuse a shared SKU instead of writing an invalid document
Given two OpenDB records with the same SKU, when the catalog converter runs, then it exits non-zero naming both files and writes no document.

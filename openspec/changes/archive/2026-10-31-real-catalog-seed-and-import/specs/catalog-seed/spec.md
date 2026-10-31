# Catalog seed and import deltas

## ADDED Requirements

### Requirement: Only redistributable data is shipped; non-commercial catalog data is fetched by the operator
The repository SHALL keep its source code under MIT. It SHALL NOT vendor a third-party hardware dataset whose licence is incompatible with that statement or whose size makes the repository a redistribution channel. The shipped sample data SHALL be licensed CC BY 4.0 with attribution in `NOTICE`. A dataset under CC BY-NC-SA 4.0 SHALL be documented as not distributed, with its attribution, non-commercial and ShareAlike obligations stated for the operator who fetches it. `docs/data-sources.md` SHALL record, for each assessed source, its maintenance evidence, licence, commercial position, field fit and verdict.

#### Scenario: No non-commercial dataset is committed
Given the repository tree, when it is inspected, then no BuildCores OpenDB data file is present and `docs/data-sources.md` and `NOTICE` state its licence and that the operator fetches it.

#### Scenario: The shipped sample carries attribution
Given the sample rows or their documentation, when an operator reuses them, then `NOTICE` identifies the CC BY 4.0 licence, the source pages and the changes made.

#### Scenario: A rejected source is recorded
Given PCPartPicker's terms prohibiting collection and derivative use, when the data sources are assessed, then `docs/data-sources.md` records the rejection and no code, fixture or script reads that source.

### Requirement: The OpenDB converter maps only recorded facts and fails loudly
The repository SHALL provide `scripts/opendb-to-csv.mjs`, a Node script with no third-party dependency, that reads a local BuildCores OpenDB checkout and writes the CSV accepted by `POST /api/imports`. It SHALL map CPU, Motherboard, RAM, GPU, PSU, PCCase, CPUCooler and Storage to `cpu`, `motherboard`, `ram`, `gpu`, `psu`, `case`, `cooler` and `storage`. It SHALL record `source=catalog:buildcores@<commit>` using the checkout's commit SHA, and `sourceUrl` from `general_product_information.manufacturer_url` when that field is present.

It SHALL emit a specification only when OpenDB records it. A set-valued field SHALL be reduced to a single required value only when the set has one unambiguous value; otherwise it SHALL be omitted. It SHALL exit non-zero and name the file and field when a requested category is unmapped, a part has no name, a part has no usable SKU, a JSON file cannot be parsed, or the operator-supplied quantity or price is missing. It SHALL NOT invent a price or quantity from catalog data.

Two catalog records that share an SKU SHALL both be emitted, with a warning naming both files, so the import preview's explicit duplicate decision resolves them rather than the converter silently merging or dropping one. A manufacturer URL that is not an absolute HTTP(S) URL SHALL be omitted with a warning rather than emitted as an invalid value. An MPN longer than the import contract's SKU limit SHALL be replaced by the record's `opendb_id` with a warning, so the row stays importable and traceable. A model name longer than the import contract allows SHALL be emitted unchanged with a warning; the preview reports the field error and the operator corrects or excludes the row.

#### Scenario: Convert a checkout
Given a local OpenDB checkout with a resolvable commit, when the converter runs with a price and quantity, then the output CSV imports through `POST /api/imports` with `catalog:buildcores@<commit>` provenance and a manufacturer URL per row that has one.

#### Scenario: Refuse an unmapped category
Given a request that names a category with no mapping, when the converter runs, then it exits non-zero and names the category.

#### Scenario: Refuse to guess a singular fact
Given a cooler that records several supported sockets, when the converter emits its row, then no `cooler_socket` specification is emitted and the compatibility rule reports `unknown`.

#### Scenario: Refuse a part with no identity
Given a part with no name or no usable SKU, when the converter runs, then it exits non-zero naming the file and the missing field, and writes no partial CSV.

#### Scenario: Surface a shared SKU without merging it
Given two OpenDB records with the same MPN, when the converter runs, then both rows are written, the warning names both files, and the import preview reports them as a duplicate group for an explicit decision.

#### Scenario: Do not emit an unusable source URL
Given a record whose manufacturer URL is not absolute HTTP(S), when the converter runs, then the row is written with an empty `sourceUrl` and a warning names the file.

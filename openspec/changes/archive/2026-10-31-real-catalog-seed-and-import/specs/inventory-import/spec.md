# Inventory provenance deltas

## MODIFIED Requirements

### Requirement: Provenance and unknown specifications stay visible end to end
The system SHALL label each value by source (`user`, `rigcraft-normalizer`, or a declared catalog source), reference, and observation time. Normalization SHALL be lexical only; the system SHALL NOT invent Socket, dimensions, BIOS support, or connector values. Specifications absent from the import SHALL remain `Unknown` and SHALL NOT be filled in from a similar name.

Optional `source` and `sourceUrl` CSV columns SHALL let a row declare where its part facts came from. A `source` value SHALL be either `user` or `catalog:<id>`, and a `sourceUrl` value SHALL be an absolute `http://` or `https://` URL. When a catalog source is declared, the row's `model` and every parsed specification SHALL cite that source with `sourceUrl` (or the CSV reference when no URL is recorded) as the reference. `sku`, `quantity`, `price` and `currency` SHALL remain `user` facts. A missing or empty `source` or `sourceUrl` column SHALL leave provenance exactly as before. An invalid value SHALL be a field error with a stable code, and the row SHALL be excluded by default.

The committed snapshot and the Web UI SHALL show whether a fact is operator-entered, catalog-provided, normalized, or unknown without relying on colour alone. The preview and the snapshot SHALL show the catalog source when one is declared, and a correction to model or specifications SHALL replace the catalog claim with `user` provenance.

#### Scenario: Show an unverified match and unknown specs
Given an imported model with no catalog source and no specification values, when the snapshot is displayed, then the UI shows an `unverified` match and an `Unknown` specification state with the source reference.

#### Scenario: Distinguish user and normalized provenance
Given a row with a model and a quantity and no source column, when the snapshot is displayed, then the model normalization cites `rigcraft-normalizer` while the user-entered quantity cites `user` and the CSV reference.

#### Scenario: Keep operator stock facts out of a catalog claim
Given a row with `source=catalog:buildcores@a1b2c3d` and a manufacturer URL, when it is committed, then the model and every specification cite `catalog:buildcores@a1b2c3d` with that URL, while the SKU, quantity and price still cite `user` and the CSV reference.

#### Scenario: Fall back when the columns are absent
Given a CSV with no `source` or `sourceUrl` column, when it is previewed and committed, then its provenance is identical to the behaviour before these columns existed.

#### Scenario: Reject an invalid catalog claim
Given a row whose `source` is not `user` or `catalog:<id>`, or whose `sourceUrl` is not an absolute HTTP(S) URL, when it is previewed, then the row has a `source_format` or `source_url_format` field error and is excluded by default.

#### Scenario: An operator correction owns the edited facts
Given a catalog-sourced row, when the operator corrects its model or specifications before commit, then the corrected row cites `user` with the correction reference instead of the catalog source.

## ADDED Requirements

### Requirement: The shipped sample build is sourced, verified and labelled
`GET /api/imports/sample` SHALL serve a CSV whose rows form one physically possible, internally consistent build in the supported categories. Each compatibility fact SHALL be taken from the manufacturer's own product page, SHALL record that page in `sourceUrl`, and SHALL record the dataset revision and verification date in `source`. A fact the cited page does not state SHALL be omitted so that it stays unknown; it SHALL NOT be inferred. The sample SHALL declare `price` and `quantity` as operator-entered stock values rather than market or catalog data.

The `/inventory` download area SHALL state that the sample data is licensed CC BY 4.0, when its facts were checked, and that prices are operator-entered.

#### Scenario: The sample passes the compatibility engine without a conflict
Given the shipped sample committed to a snapshot and one unit of every category selected, when the compatibility check runs, then no rule reports a conflict, and every rule that lacks a sourced fact reports `unknown` with a follow-up.

#### Scenario: The impossible socket pairing is gone
Given the shipped sample, when its CPU and motherboard rows are compared, then the CPU Socket equals the motherboard Socket and no row declares an AM4 processor as AM5.

#### Scenario: A missing manufacturer fact stays unknown
Given a manufacturer page that does not state a value, when the sample is authored, then that specification is absent and the affected rule reports `unknown` rather than a guessed value.

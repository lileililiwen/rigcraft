# ui-foundation deltas

## ADDED Requirements

### Requirement: The inventory route is an operator console
`/inventory` SHALL open with the state of the snapshot it has open: one summary bar
naming the revision, record count, units in stock, stock value and whether that
revision is the latest, followed by its source file and commit time. The bar SHALL
carry a status chip of symbol, word and border and SHALL NOT replace the committed
snapshot panel, which SHALL keep every record, field, provenance chip, disclosure,
correction entry point and announcement it had. Snapshot history SHALL be a
readable card or table listing every revision with its revision number, timestamp,
record count, stock value and an explicit `View` action, marking the revision that
is currently open. Import, export and backup, and restore SHALL remain distinct
panels with the console header treatment, and the import panel SHALL stay reachable
when a snapshot is open.

#### Scenario: The summary names the snapshot before the records
Given a committed snapshot with ten records, when `/inventory` renders, then the summary bar states the revision, `10` records, the units in stock and the stock value, and the records table below it still lists every record with its provenance.

#### Scenario: History is a readable list of revisions with an action
Given three revisions, when the history panel renders, then each revision appears once with its revision number, ISO 8601 UTC timestamp, record count and stock value, and each has its own `View` action while the open revision is marked.

#### Scenario: The panels stay distinct
Given an open snapshot, when the page renders, then the import panel, the snapshot history panel and the export and backup panel are separate labelled regions, and the import file input is still on the page.

#### Scenario: Status is never colour alone
Given an open snapshot that is not the newest revision, when the summary bar renders, then its chip contains a symbol, a word and a border in addition to its colour.

### Requirement: The catalog route is a registry console
`/catalog` SHALL present the upload panel, the dataset registry and the search
results as console surfaces. The registry SHALL be a table of stored datasets with
the dataset name and a catalog provenance chip, the licence, the version, the part
count, the per-category counts, the source URL, the active state as a symbol plus
word plus border chip, and its actions, together with a count summary of datasets,
parts and active datasets. Search results SHALL stay in rank order and SHALL name
the rank, the dataset, its licence and its version for every hit, while keeping the
part detail view. The accepted report and the refusal alert SHALL keep their
headings, codes, row and field wording, and every state — no dataset, refused
document, accepted report and empty search — SHALL stay reachable and announced.

#### Scenario: The registry shows the facts that decide reuse
Given two stored datasets, when the registry renders, then each row shows its licence, version, part count, category counts, source URL and active state, and the summary states the dataset, part and active counts.

#### Scenario: Search keeps rank and attribution
Given a ranked search result, when the table renders, then each hit carries its position in the ranking, its dataset name, licence and version, and the `View part` action still opens the stored part with its provenance.

#### Scenario: Refusal still names the row
Given a refused document, when the page renders, then the alert keeps its `Catalog refused` heading, the documented code, and the row index and field, and no dataset is added.

### Requirement: Console actions are disabled until their input is valid
The four actions that need input — `Preview CSV`, `Search catalog`, `Match this
list` and `Diagnose this build` — SHALL be disabled while their input is missing,
and each SHALL reference an accessible reason that names what is required, so the
operator learns the requirement before the click rather than from an error
afterwards. `Preview CSV` SHALL require a chosen file and a three-letter currency
code; `Search catalog` SHALL require a non-empty query; `Match this list` SHALL
require a non-empty pasted list; `Diagnose this build` SHALL require at least one
reviewed or hand-added part. No action that currently succeeds SHALL be disabled,
and every request guard and server validation SHALL stay the source of truth.

#### Scenario: Preview waits for a file and a currency
Given no chosen file, when the import panel renders, then `Preview CSV` is disabled and its accessible reason says a CSV must be chosen; given a file and the currency `USD` it is enabled.

#### Scenario: Search waits for a query
Given an empty query, when the search panel renders, then `Search catalog` is disabled with a reason naming the query; typing one character enables it.

#### Scenario: Diagnosis waits for a part
Given a snapshot with no reviewed line and no hand-added part, then `Match this list` is disabled with an empty list and `Diagnose this build` is disabled with an empty build, and adding one part enables the diagnosis.

#### Scenario: A resolvable review is still submitted
Given a matched list whose review is unresolved, when the operator activates `Diagnose this build`, then it is enabled, the request is checked, and the existing review-complete refusal is announced.

### Requirement: Console counts and examples are real
Every count the inventory, catalog and diagnosis pages compose SHALL go through the
shared pluralisation helper, so a count of one reads as a singular noun and no
page-composed `(s)` pseudo-plural remains. The diagnosis examples — the pasted-list
placeholder, the parts-list help text and the manual-entry specification example —
SHALL use parts and specifications the shipped sample actually contains. Engine and
exported prose SHALL remain verbatim.

#### Scenario: Singular and plural forms follow the count
Given one record and two records, when the inventory history and the export panel render, then they read `1 record` and `2 records`.

#### Scenario: The examples match the shipped sample
Given the `/diagnose` page, when the pasted-list placeholder and the parts-list help text are read, then every example part is one of the shipped sample's parts and no example part is labelled `Fixture`.

#### Scenario: No page-composed pseudo-plural
Given the three pages at any count, when their rendered text is searched for `record(s)`, `part(s)`, `hit(s)` or `snapshot(s)`, then none is found in text the page composed.

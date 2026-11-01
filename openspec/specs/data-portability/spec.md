# data-portability Specification

## Purpose

Define how a shop takes its private catalog out and puts it back: deterministic snapshot export in importer-column CSV and authoritative JSON, a consistent standard-SQLite whole-database backup, and a restore that validates read-only, writes a permanent pre-restore copy and replaces live data atomically.
## Requirements
### Requirement: A snapshot can be exported as CSV in the importer's own columns
The system SHALL serve `GET /api/inventory/snapshots/{id}/export?format=csv` as a
UTF-8 CSV with exactly the header
`sku,category,model,quantity,price,currency,specs,source,sourceUrl`, one row per lot
in stored order. Each row SHALL reproduce the committed `sku`, canonical `category`,
original model text, integer quantity, two-decimal price, currency, raw
`key=value` specifications, the model's declared catalog source (or `user`), and an
absolute HTTP(S) `sourceUrl` when the provenance reference is one.

Re-importing that CSV through `POST /api/imports` SHALL reproduce the same records
and the same provenance sources: operator fields stay `user`, catalog part facts keep
their `catalog:<id>` value.

CSV SHALL be documented and header-announced as lossy: it carries no lot identity,
snapshot metadata, per-field provenance reference, or match confidence. The response
SHALL include `X-Rigcraft-Export-Lossy: true` and a human-readable lossiness note
header.

#### Scenario: CSV export re-imports to the same records and provenance sources
Given a committed snapshot with catalog and operator facts, when its CSV export is imported again, then the new snapshot has the same SKUs, categories, models, quantities, prices, currencies and specifications, the model facts still cite `catalog:<id>`, and the stock facts still cite `user`.

#### Scenario: CSV states its own limitation
Given a successful CSV export, when the response is inspected, then the content type is CSV, the lossiness headers are present, and the filename contains the snapshot revision.

#### Scenario: One row per lot
Given a snapshot with two lots that share an SKU, when it is exported as CSV, then the file has two data rows and the header appears once.

### Requirement: A snapshot can be exported as an authoritative JSON artifact
The system SHALL serve `GET /api/inventory/snapshots/{id}/export?format=json` as a
deterministic JSON artifact containing the snapshot ID, created time, revision,
parent snapshot ID, currency, source, note, totals, and every lot with its raw and
normalized model, quantity, price, currency, each specification's raw and normalized
value with provenance, the provenance of every field, and match basis and confidence.
It SHALL identify the format and a format version, and it SHALL be marked
authoritative and not lossy. It SHALL set a `Content-Disposition` filename
containing the snapshot revision.

#### Scenario: JSON round-trips lots and per-field provenance
Given a committed snapshot, when its JSON export is parsed, then every lot is present with its spec values and the provenance source of each field, and the snapshot revision and parent are retained.

#### Scenario: Export is deterministic
Given the same immutable snapshot, when it is exported twice as JSON, then both bodies are byte-identical and contain no export timestamp.

### Requirement: Export errors use the existing envelope
An unknown snapshot SHALL return `404 snapshot_not_found`. An unsupported or missing
format SHALL return `400 export_format_unsupported` with field `format`. Both SHALL
use the existing JSON error envelope.

#### Scenario: Unknown snapshot
Given no snapshot has the requested ID, when either export format is requested, then the response is 404 `snapshot_not_found` in the existing envelope.

#### Scenario: Unsupported format
Given a snapshot that exists, when `format=xml` is requested, then the response is 400 `export_format_unsupported` and names `format` as the field.

### Requirement: The whole database can be downloaded as a standard SQLite file
The system SHALL serve `GET /api/backup` as a consistent copy of the live database
produced with `VACUUM INTO`, not a hand-rolled container or archive. The response
SHALL have content type `application/vnd.sqlite3` and a
`Content-Disposition` filename `rigcraft-backup-<UTC>.sqlite3`. The file SHALL open
with a standard SQLite client and contain the same snapshot, record and lot counts as
the live service.

#### Scenario: Backup is a valid SQLite database with the same counts
Given a live catalog, when its backup is downloaded, then the file opens read-only as SQLite, passes `PRAGMA integrity_check`, and reports the same snapshot and record counts.

#### Scenario: Backup is named and typed as SQLite
Given a successful backup, then the content type is `application/vnd.sqlite3` and the filename matches `rigcraft-backup-<UTC>.sqlite3`.

### Requirement: Restore validates an upload before touching live data
The system SHALL accept an uploaded SQLite file and validate it read-only before any
write. It SHALL require a passing `PRAGMA integrity_check`, a `user_version` between
1 and the schema version this build supports, and the expected tables and columns.
Each failure class SHALL return a distinct documented code in the existing envelope:

- `backup_empty` — a zero-byte upload;
- `backup_not_sqlite` — no SQLite header or SQLite refuses to open it;
- `backup_corrupt` — integrity check does not pass;
- `backup_schema_too_new` — `user_version` exceeds the supported schema version;
- `backup_wrong_shape` — `user_version` below 1, a required table missing, or a required column absent.

In every rejection the live catalog SHALL be unchanged.

#### Scenario: Reject a corrupt file
Given a SQLite file whose integrity check fails, when restore is attempted, then the response is `backup_corrupt` and the live record count is unchanged.

#### Scenario: Reject a newer schema
Given a valid SQLite file with a `user_version` greater than the supported schema, when restore is attempted, then the response is `backup_schema_too_new` and the live catalog is unchanged.

#### Scenario: Reject a non-SQLite upload
Given a plain text file, when restore is attempted, then the response is `backup_not_sqlite` and the live catalog is unchanged.

#### Scenario: Reject an empty or wrong-shape upload
Given a zero-byte file, then restore returns `backup_empty`; given a SQLite file without the RigCraft tables, then restore returns `backup_wrong_shape`; in both cases the live catalog is unchanged.

### Requirement: Restore writes a pre-restore copy and replaces data atomically
`POST /api/backup/restore` SHALL require an explicit multipart confirmation whose
value is `restore`, and SHALL otherwise return `400 backup_confirm_required` before
validating or writing. After validation it SHALL write a safety copy of the current
database into the data directory as
`inventory.sqlite3.pre-restore-<UTC timestamp>` and SHALL never delete that copy
automatically. It SHALL then replace the live data with SQLite's backup API into the
live connection, so the destination is committed in one operation and a failure rolls
it back rather than leaving a half-restored database. A `user_version` older than the
current schema SHALL migrate forward through the existing migration path.

The response SHALL summarise the source schema version, the resulting schema version,
snapshot count, latest revision, record count, lot count and the pre-restore path. A
failure after the pre-restore copy SHALL return `500 backup_restore_failed` with
`details.preRestorePath` naming the file that holds the previous catalog.

#### Scenario: Restore reproduces counts
Given a backup of the live catalog, when it is restored with the confirmation, then the service reports the same snapshot, revision, record and lot counts and the live API returns those records.

#### Scenario: Pre-restore copy exists after a successful restore
Given a successful restore, then a file named `inventory.sqlite3.pre-restore-<UTC>` exists in the data directory, is reported in the response, and is not deleted automatically.

#### Scenario: An older schema migrates forward
Given a valid RigCraft database with `user_version` below the current schema, when it is restored, then the response reports the source version and the current version, and the restored data is readable.

#### Scenario: A single request cannot restore by accident
Given a restore request without the confirmation field, then the response is `400 backup_confirm_required` and no pre-restore copy or data change is made.

### Requirement: The inventory page offers export, backup and guarded restore
`/inventory` SHALL offer CSV and JSON export of the selected snapshot, a
whole-database backup download, and a restore flow that inspects an upload before
replacing anything. The export actions SHALL become usable as soon as a snapshot is
open, and the page SHALL open the newest revision on first load, so a shop with a
committed snapshot can export without first discovering a separate selection
control. While no snapshot is open the export actions SHALL be disabled and the
copy SHALL name the controls that open one — the history `View` action or a commit.
Inspection SHALL show the integrity result, schema version, created date and record
and lot counts. Replacement SHALL require an explicit confirmation typed by the
operator, so it cannot be triggered by one accidental click. Success SHALL show the
pre-restore path; failure SHALL show the stable code and, when present, the path that
holds the previous data. The panel SHALL keep the page's existing keyboard path,
focus moves, accessible announcements and 390 px layout.

#### Scenario: Inspect then confirm
Given a chosen backup file, when the operator inspects it, then the page shows integrity, schema version, created date and counts, and the replace button stays disabled until the confirmation word is typed.

#### Scenario: Successful restore is visible
Given a confirmed restore, then the page announces success, shows the pre-restore path, and refreshes the snapshot history.

#### Scenario: A rejected restore leaves the page usable
Given a rejected upload, then the page shows the documented code in an alert, keeps the current inventory visible, and allows another file to be chosen.

#### Scenario: Export is usable when a snapshot exists
Given a committed snapshot, when `/inventory` first loads, then the newest revision is open and both `Download CSV` and `Download JSON` are enabled without any other action.

#### Scenario: The disabled state names the control
Given no snapshot has ever been committed, when the export panel renders, then both export buttons are disabled and the copy names `View` in Snapshot history and committing an import as the ways to open one.

### Requirement: Portability documentation states formats, guarantees and privacy
`docs/data-portability.md` SHALL describe both export formats, the CSV lossiness and
round-trip guarantee, the backup format and its inspectability, restore validation
and error codes, the atomic restore mechanism, the pre-restore copy semantics, and the
fact that an export or backup contains the shop's entire private inventory. The README
SHALL list the new routes and runtime behaviour.

#### Scenario: The privacy consequence is documented
Given the portability document, then it states that a backup and the JSON export contain the complete private inventory and are produced only on an explicit local request.

#### Scenario: The README routes are current
Given the README Routes section, then it lists both snapshot export formats, the backup download, the inspection route and the restore route.


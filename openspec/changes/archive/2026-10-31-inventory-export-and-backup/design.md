# Design: Snapshot export, database backup and guarded restore

## Snapshot export

### CSV

The header is exactly the importer's column set, in the importer's order:

```text
sku,category,model,quantity,price,currency,specs,source,sourceUrl
```

One row is written per lot, in the snapshot's stored `position` order. Values are
taken from the normalized record, not re-normalized on export:

| Column | Source | Notes |
| --- | --- | --- |
| `sku` | `record.sku` | as committed |
| `category` | `record.category` | canonical category |
| `model` | `record.model_raw` | original model text, so import normalizes it back to the same `model_normalized` |
| `quantity` | `record.quantity` | integer |
| `price` | `record.price_minor` | rendered with exactly two decimals |
| `currency` | `record.currency` | batch currency |
| `specs` | `record.specs[*].raw` | joined with `;`; the raw `key=value` text the importer parsed |
| `source` | `record.provenance["model"].source` | `catalog:<id>` or `user` |
| `sourceUrl` | `record.provenance["model"].reference` | only when it is an absolute HTTP(S) URL |

CSV is deliberately lossy and the contract says so: it carries no lot identity, no
snapshot ID or revision, no per-field provenance reference for each field (only the
model's declared catalog source), no match basis or confidence, and no created
timestamp. Re-importing the CSV therefore reproduces the stock records and the
provenance *sources* (operator fields stay `user`, catalog part facts keep their
`catalog:<id>`), with new record IDs, lot numbers, file reference and timestamps.
The JSON export is the authoritative artifact. The CSV response carries
`X-Rigcraft-Export-Lossy: true` and `X-Rigcraft-Export-Note`, and the docs repeat
the limitation.

Writer: the `csv` crate with `QuoteStyle::Necessary`, `\n` terminator, so a model
containing a comma is quoted and the importer reads it back unchanged.

### JSON

```json
{
  "format": "rigcraft.inventory.snapshot",
  "formatVersion": 1,
  "authoritative": true,
  "lossy": false,
  "snapshot": { "...": "the complete Snapshot serialization" }
}
```

The embedded snapshot already contains the ID, created time, revision,
`parentSnapshotId`, currency, source file and checksum, note, totals, and every
record with its raw and normalized model, quantity, price, currency, specs (raw,
normalized value, unit, status and provenance) and the provenance map for every
field, plus match basis and confidence. The wrapper identifies the artifact and its
version. Serialization uses `serde_json::to_string_pretty`, so the bytes are
deterministic for the same immutable snapshot.

### Filenames

`Content-Disposition: attachment; filename="rigcraft-inventory-snapshot-r<revision>-<short id>.<csv|json>"`.
The revision is required; the short ID prevents two snapshots at the same revision in
different chains from sharing an obvious name.

### Errors

- unknown snapshot: `404 snapshot_not_found`, the existing envelope;
- unsupported or missing format: `400 export_format_unsupported` with field `format`;
- the existing storage failure path for anything else.

## Whole-database backup

`GET /api/backup`:

1. Under the store mutex, run `VACUUM INTO '<temporary path in the data directory>'`.
   SQLite reads a consistent snapshot of committed data, including WAL content, and
   writes a compact standalone database. No other rigcraft writer can interleave
   because the connection is locked for the duration.
2. Read the temporary file, delete it, and stream the bytes with
   `Content-Type: application/vnd.sqlite3` and
   `Content-Disposition: attachment; filename="rigcraft-backup-<UTC>.sqlite3"`.
   The timestamp is `YYYYMMDDTHHMMSSffffffZ` so two downloads in one second do not
   share a name.

A plain SQLite file is the point: `sqlite3`, DB Browser, or a Python `sqlite3` session
can inspect it without RigCraft. The file contains the shop's entire private
inventory, snapshots, previews, check reports, recommendations and diagnoses. It is
never uploaded anywhere by this change.

## Restore

### Validation before any write

An upload is written to a temporary file in the data directory, then opened
read-only. Checks run in this order and each maps to one stable code:

| Condition | Code | HTTP |
| --- | --- | --- |
| zero bytes | `backup_empty` | 400 |
| no `SQLite format 3\0` header, or SQLite refuses to open it | `backup_not_sqlite` | 400 |
| `PRAGMA integrity_check` does not return `ok` | `backup_corrupt` | 422 |
| `user_version` greater than this build's schema version | `backup_schema_too_new` | 422 |
| `user_version` below 1, a required table missing, or a required column absent | `backup_wrong_shape` | 422 |

Required tables are `snapshots`, `records` and `previews`, and the shape probe reads
the columns each of them must have. A valid database with zero snapshots is accepted;
"empty" in the table above means a zero-byte file. The live database is not opened
for writing until every check passes.

`POST /api/backup/inspect` runs exactly these checks and returns:

```text
fileName, sizeBytes, integrity, schemaVersion, snapshotCount,
latestRevision, createdAt, recordCount, lotCount
```

`lotCount` counts the records of the newest snapshot, which is what an operator
thinks of as "the lots currently on the page"; `recordCount` counts every lot in
every snapshot.

### Restore sequence

`POST /api/backup/restore` requires a multipart `file` and a `confirm` field whose
value is `restore`; anything else is `400 backup_confirm_required`. It then:

1. Validates the upload again (the file may have changed between inspect and
   restore) and computes the summary.
2. Writes a safety copy of the current live database with `VACUUM INTO` into the
   data directory as `inventory.sqlite3.pre-restore-<UTC timestamp>`. If this fails,
   the restore stops with `500 backup_pre_restore_failed` and the live database is
   untouched.
3. Opens the validated upload read-only and copies it into the live connection with
   `rusqlite::backup::Backup` (SQLite's online backup API). SQLite performs this as
   one destination transaction; if the copy does not reach `SQLITE_DONE`, finishing
   the backup handle rolls the destination back to its previous contents. There is no
   intermediate state in which half of the old and half of the new catalog is live.
4. Re-applies the connection pragmas (`foreign_keys=ON`, `journal_mode=WAL`) and runs
   the existing forward migration, so an upload with `user_version` 1–3 becomes the
   current schema without losing rows.
5. Counts the restored database and returns the summary, including
   `sourceSchemaVersion`, the final `schemaVersion` and `preRestorePath`.

If step 3 or 4 fails, the response is `500 backup_restore_failed` with
`details.preRestorePath` naming the file that holds the previous catalog. The copy is
never deleted automatically; cleanup is the operator's decision. A successful restore
also never deletes it.

### Why not a file swap

Replacing `inventory.sqlite3` on disk while a live connection and its WAL exist is
not atomic across the connection, the WAL and the server's open file descriptor;
a crash between rename and reopen can leave the process serving the old inode or a
mismatched WAL. Copying into the open connection with the SQLite backup API avoids
that class of failure and keeps one durable destination transaction.

## Web flow

A `DataPortability` panel on `/inventory`:

- **Export this snapshot**: two buttons, CSV and JSON, enabled only when a snapshot
  is selected. Each asks the service for the file and hands it to the browser with
  the service's filename. A status line announces the downloaded name.
- **Back up this database**: one button that downloads the SQLite file.
- **Restore from a backup**: a file input, an `Inspect backup` button, and then an
  inspection panel with integrity, schema version, created date, snapshot count,
  latest revision, record count and lot count. The confirmation control is a text
  input that must contain `RESTORE`; until it does, the `Replace this inventory`
  button is disabled. One accidental click cannot restore. A short warning names
  what will be replaced: the live catalog at the service's data directory, with the
  counts read from the current snapshot history.
- Success moves focus to a heading and shows the pre-restore path; failure shows the
  stable code and the path when one exists. Restored data refreshes history and
  selects the newest snapshot.

Accessibility: labelled controls, `aria-describedby` hints, `role="status"` live
announcements, `role="alert"` errors, visible focus, no colour-only distinction, and
the existing 390 px stacked layout.

## Privacy and scope

A backup and the JSON export contain every private inventory field, including
optional purchase costs if they were ever stored in a record. They are produced only
on an explicit request to the local service, are never logged, and are never sent to
a provider or catalog. The service binds to loopback by default. No cloud, schedule,
encryption or new file format is added.

## Compatibility

- SQLite schema stays `user_version = 4`.
- `rusqlite` keeps its version and gains the `backup` feature.
- Existing routes, error envelopes, inventory semantics, recommendation, diagnosis,
  compatibility and BYOK behaviour are unchanged.
- A v1–v3 upload is migrated by the existing `migrate` function after the copy.

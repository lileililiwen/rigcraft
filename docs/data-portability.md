# Data portability: export, backup and restore

Status: implemented in `inventory-export-and-backup` (2026-11-01).

The inventory catalog lives in SQLite outside the repository: `RIGCRAFT_DATA_DIR`
when set, otherwise `$XDG_DATA_HOME/rigcraft` or `$HOME/.local/share/rigcraft`.
That file is the only copy of a shop's stock, corrections, compatibility reports,
recommendations and diagnoses. This document defines the three ways to get it out
and the one guarded way to put it back.

## Snapshot export

Both formats are served from an immutable snapshot and are deterministic for that
snapshot: the same snapshot produces the same bytes, with no export timestamp.

```text
GET /api/inventory/snapshots/{id}/export?format=csv
GET /api/inventory/snapshots/{id}/export?format=json
```

The filename contains the snapshot revision, for example
`rigcraft-inventory-snapshot-r1-1b2c3d4e.csv`. An unknown snapshot returns
`404 snapshot_not_found`; a missing or unsupported format returns
`400 export_format_unsupported` with field `format`.

### CSV — the import columns

```text
sku,category,model,quantity,price,currency,specs,source,sourceUrl
```

One row per lot. The columns are exactly those `POST /api/imports` accepts, so the
file opens in a spreadsheet and feeds straight back into the import preview. The
values come from the committed record: SKU, canonical category, original model
text, quantity, price with two decimals, currency, the raw `key=value`
specifications, the declared catalog source (or `user`), and the manifest page URL
when the provenance reference is an absolute HTTP(S) URL.

Re-importing the file reproduces the stock records and the provenance **sources**:
operator fields (`sku`, `quantity`, `price`, `currency`) stay `user`, and catalog
part facts (`model`, specifications) keep their `catalog:<id>`. Record IDs, lot
numbers, the source filename reference and observation timestamps are new, because
they belong to the new batch.

**CSV is lossy and says so.** It cannot carry:

- lot identity or the snapshot's record order guarantee;
- the snapshot ID, revision, parent revision, creation time or totals;
- per-field provenance references and observation times (only the model's declared
  catalog source is exported);
- match basis or confidence;
- purchase cost, which the import contract does not accept.

The response carries `X-Rigcraft-Export-Lossy: true` and
`X-Rigcraft-Export-Note` repeating this. Use JSON when the artifact must be
complete.

### JSON — the authoritative artifact

```json
{
  "format": "rigcraft.inventory.snapshot",
  "formatVersion": 1,
  "authoritative": true,
  "lossy": false,
  "snapshot": { "...": "complete snapshot" }
}
```

The embedded snapshot contains the ID, creation time, revision, parent snapshot ID,
currency, source file and SHA-256, note, totals, and every lot with its raw and
normalized model, quantity, price, currency, every specification's raw and
normalized value, unit, status and provenance, the provenance of every field, and
the match basis and confidence. JSON is the artifact to keep or hand to another
RigCraft instance's importer or archive.

## Whole-database backup

```text
GET /api/backup
```

The service runs `VACUUM INTO` under its connection lock, so the copy is a
consistent snapshot of committed data including WAL content, and writes no
half-copied file. The response is the file itself:

- `Content-Type: application/vnd.sqlite3`
- `Content-Disposition: attachment; filename="rigcraft-backup-<UTC>.sqlite3"`
- timestamp `YYYYMMDDTHHMMSSffffffZ`

It is a plain SQLite file, deliberately inspectable with `sqlite3`, DB Browser or a
Python `sqlite3` session. It contains the entire private catalog: snapshots,
records, previews, check reports, recommendations and diagnoses.

## Restore

### Preview first

```text
POST /api/backup/inspect        multipart file
POST /api/backup/restore        multipart file + confirm=restore
```

`inspect` opens the upload read-only, validates it and returns its integrity result,
schema version, created date, snapshot count, latest revision, record count, lot
count and size. It writes nothing. The `/inventory` page shows this summary before
any replacement is possible.

### Validation and error codes

Every check runs before the live database is opened for writing. Each failure has one
stable code in the existing error envelope:

| Condition | Code | HTTP |
| --- | --- | --- |
| zero-byte upload | `backup_empty` | 400 |
| no `SQLite format 3\0` header, or SQLite refuses to open it | `backup_not_sqlite` | 400 |
| `PRAGMA integrity_check` does not return `ok` | `backup_corrupt` | 422 |
| `user_version` greater than this build's schema version | `backup_schema_too_new` | 422 |
| `user_version` below 1, a required table missing, or a required column absent | `backup_wrong_shape` | 422 |
| restore without `confirm=restore` | `backup_confirm_required` | 400 |
| no file part | `backup_file_required` | 400 |

The required tables are `snapshots`, `records` and `previews`; the shape probe reads
the columns each must have. A valid RigCraft database with zero snapshots is
accepted. A rejected upload leaves live data untouched and writes no safety copy.

### The pre-restore copy

Before anything is replaced, the service writes a safety copy of the current
database into the data directory:

```text
inventory.sqlite3.pre-restore-<UTC timestamp>
```

It is never deleted automatically, on success or failure. The response names the
exact path. If the restore later fails, that path is where the previous catalog is,
and the failure response repeats it in `error.details.preRestorePath`.

### How the replacement is atomic

After validation, the upload is copied into the **live connection** with SQLite's
online backup API (`rusqlite::backup::Backup`). SQLite performs the copy as one
destination transaction: if it does not reach completion, finishing the backup
handle rolls the destination back to its previous contents. There is no window in
which half of the old catalog and half of the new one are live, and there is no
rename that can leave the process serving an old inode or a mismatched WAL.

The connection then re-applies `foreign_keys=ON` and `journal_mode=WAL`, and runs the
existing forward migration. A `user_version` from 1 to the current version therefore
migrates forward on restore. `sourceSchemaVersion` in the response records what the
upload declared; `counts.schemaVersion` records the resulting version.

A successful response also reports snapshot count, latest revision, record count,
lot count and the pre-restore path.

### Why not swap the file on disk

Replacing `inventory.sqlite3` while a connection, a WAL and an open file descriptor
exist is not atomic across all three. A crash between rename and reopen can leave
the process serving the old file while the new one sits on disk. Copying into the
open connection avoids that whole failure class.

## Web flow

`/inventory` has an **Export and backup** panel:

1. **Download CSV** and **Download JSON** export the snapshot currently shown. The
   newest revision opens automatically on first load, so the buttons are enabled
   without an extra step; with no snapshot they stay disabled and the copy names
   the control (`No snapshot is open. Choose View on a revision in Snapshot
   history, or commit an import.`).
2. **Download backup** downloads the whole SQLite file.
3. **Restore from a backup** takes a file, then **Inspect backup** shows integrity,
   schema version, created date and counts, and names what will be replaced.
4. The **Replace this inventory** button stays disabled until the operator types
   `RESTORE`. One accidental click cannot restore.
5. Success shows the pre-restore path and refreshes the history; failure shows the
   code and, when a safety copy exists, its path. The current inventory stays visible
   throughout a rejected attempt.

The panel keeps the page's keyboard order, visible focus, `role="status"` and
`role="alert"` announcements, and the 390 px stacked layout.

## Guarantees and limits

- Snapshot export is deterministic and requires no network.
- CSV round-trips records and provenance sources, not identity or metadata.
- A backup is one consistent, standard SQLite file.
- A rejected restore changes nothing.
- A completed restore is atomic; a failed restore rolls back and names the safety
  copy.
- No cloud upload, scheduling, encryption, archive container or background job is
  part of this change. Moving a backup to another machine is the operator's file
  transfer, not a service feature.
- Automatic use of an old pre-restore copy, retention limits and deletion are not
  provided. The operator owns those files.

## Privacy

An export, a backup and an inspection all happen inside the local service, on an
explicit request. A backup and the JSON export contain the shop's **entire private
inventory**, including optional purchase costs if they were stored. They are never
sent to a provider or catalog and their contents are not logged. The service binds to
loopback by default. Treat the downloaded files with the same care as the original
database.

## Evidence

- `docs/evidence/data-portability-browser.md` — browser journeys, the confirmation
  guard, the rejected restore, responsive widths and the privacy review.
- `docs/assets/data-portability-*.png` — capture set.
- `docs/evidence/data-portability-lighthouse/` and `-mobile/` — Lighthouse reports.

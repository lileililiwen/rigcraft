# Proposal: Export the inventory catalog and back up the database

## Why

The SQLite catalog that holds a shop's inventory lives outside the repository
(`RIGCRAFT_DATA_DIR`, otherwise the XDG/HOME application data directory). It is the
only copy of that shop's data. A disk loss, a mistaken upgrade or a failed move to
another machine destroys real stock records, and there is no supported way to move a
catalog between machines. The application can import a CSV but cannot emit the
catalog it already holds, and it has no whole-database backup or restore path.

This change closes that defect with three narrow, inspectable operations: a
snapshot export in two formats, a consistent whole-database backup, and a guarded
restore that validates before it replaces anything and keeps a pre-restore copy.

## What Changes

- Add `GET /api/inventory/snapshots/{id}/export?format=csv|json`.
  - `csv` emits one row per lot with exactly the columns the importer accepts
    (`sku,category,model,quantity,price,currency,specs,source,sourceUrl`) so the
    file can be opened in a spreadsheet and fed straight back into
    `POST /api/imports`. CSV is lossy: it cannot carry lot identity, per-field
    provenance references or snapshot metadata. The response says so in headers and
    `docs/data-portability.md` says so in prose.
  - `json` is the authoritative artifact: snapshot metadata, revision, parent,
    totals, every lot, every normalized field and every field's provenance.
  - Both are deterministic for the same immutable snapshot, and both set a
    `Content-Disposition` filename containing the snapshot revision.
- Add `GET /api/backup`, which streams a consistent copy of the live database as a
  plain SQLite file produced with `VACUUM INTO`, named
  `rigcraft-backup-<UTC>.sqlite3` with `application/vnd.sqlite3`. A standard SQLite
  file is intentionally inspectable by any SQLite tool.
- Add `POST /api/backup/inspect` and `POST /api/backup/restore`.
  - `inspect` validates an upload without writing anything and returns its
    integrity result, schema version, created date and counts for the UI preview.
  - `restore` validates before touching live data, writes a pre-restore safety copy
    named `inventory.sqlite3.pre-restore-<UTC>` into the data directory, and then
    copies the upload into the live connection with the SQLite backup API. That API
    writes the destination in one atomic operation: if it does not complete, SQLite
    rolls the destination back, so a half-restored catalog is not a possible
    outcome. A `user_version` older than the supported schema is migrated forward
    through the existing migration path.
  - Corrupt, non-SQLite, empty, too-new and wrong-shape uploads each return a
    distinct, documented error code and leave live data untouched.
- Add a `/inventory` data-portability panel: export the selected snapshot as CSV or
  JSON, download a backup, and restore only after an inspect step that shows the
  upload's integrity, schema version, created date and counts, followed by typing an
  explicit confirmation word. The pre-restore path is shown after success.
- Add `docs/data-portability.md`, update the README routes and runtime notes, and
  record the guarantee that a backup contains the shop's entire private inventory.

## Package Boundary and Split Assessment

Single outcome: a shop can leave with its catalog, put it back, and survive a bad
upgrade. Export, backup and restore share one artifact family, one data directory
and one acceptance gate; splitting them would ship an export that no restore can
consume. No recommendation, diagnosis, compatibility or BYOK behaviour changes.
Adding the `backup` feature to the existing `rusqlite` dependency is a feature flag,
not a new dependency; no archive format, cloud service, scheduler or encryption is
added.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence | Compatibility and ownership | Decision |
| --- | --- | --- | --- |
| `rusqlite` `backup` feature | `Cargo.toml`, vendored crate `backup.rs` | Already a direct dependency at the same version; the feature exposes SQLite's own online backup API and adds no crate. | enable the feature |
| Forge Rust platform libraries | `../forge/modules/rust-platform-libs/Cargo.toml`, `forge.yaml` | Profile declares zero consumption; no crate owns SQLite backup or CSV export. | keep local |
| Pinned design tokens | `apps/web/src/styles/vendor/tokens.css`, `.project.json` | Already vendored and pinned; the portability panel reuses measured tokens. | reuse, no new token |

## User Experience and Interface Impact

Actors: the shop operator who must move, preserve or recover a catalog. Surface:
`/inventory`, in a new data-portability panel beside the committed snapshot and
history. States: nothing selected, export ready, backup downloading, restore
file chosen, inspecting, inspection failed with a code, inspection ready, confirmation
required, restore pending, restored with the pre-restore path, and a restore failure
that names the safety copy. Existing empty, parsing, review, correction, committed,
retry, earlier-revision and 390 px states stay intact. Focus moves to the inspection
heading, the success heading and the error alert; the live region announces each
transition; keyboard order reaches every control.

## BFS Impact Map

Rust: new `portability` module (CSV/JSON serialization, filenames, inspection and
restore types), `store` (VACUUM INTO, read-only validation, backup-API restore,
pre-restore copy, migrate on restore), `api` (three routes plus the snapshot export
route), `error` (existing envelope only). TypeScript: `api.ts` download helpers,
`types.ts` inspection/result shapes, a new `DataPortability` component wired into
`InventoryPage`, styles that reuse existing tokens. Tests: new Rust unit and HTTP
suites, Vitest component tests, new Playwright journeys. Docs: new
`docs/data-portability.md`, README routes/runtime, `docs/inventory-import.md` cross
reference, `docs/assets/capture-plan.md`, evidence. SQLite schema stays at
`user_version = 4`; a restore of an older database migrates on the live connection.

## Capabilities

- `data-portability` (added): snapshot export, whole-database backup, guarded
  restore, the pre-restore copy and the CSV lossiness contract.
- `inventory-import` (modified): the export routes that reuse the importer's columns
  and a cross-reference from the import contract.

## Non-goals

Cloud storage, scheduling, encryption, archives or zips, a background job, export of
recommendations or diagnoses, changes to inventory semantics, compatibility rules,
recommendation scoring, diagnosis advice, or the BYOK panel.

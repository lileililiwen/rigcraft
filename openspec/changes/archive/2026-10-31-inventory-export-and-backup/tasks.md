# Tasks: Inventory export, backup and restore

## 1. BFS — Baseline and impact coverage
- [x] 1.1 Read `AGENTS.md`, `.ai-rules/`, `HANDOFF.md`, `ROADMAP.md`, `requirement.md`, `docs/inventory-import.md`, `docs/data-sources.md`, `src/store.rs` and the `/inventory` UI; confirm no other active change and set `current_spec`.
- [x] 1.2 Map impact across the store, API routes, error envelope, snapshot UI, styles, Rust tests, Vitest, Playwright and docs.
- [x] 1.3 Assess shared infrastructure by name (`rusqlite` backup feature, Forge Rust libraries, pinned tokens) and record that no new crate is added.

## 2. DFS — Snapshot export
- [x] 2.1 Add a `portability` module that renders one CSV row per lot with the importer's exact columns and a deterministic JSON artifact with full snapshot fidelity.
- [x] 2.2 Derive `source`/`sourceUrl` from model provenance, render price and specs so re-import reproduces records and provenance sources, and quote values with the `csv` crate.
- [x] 2.3 Add `GET /api/inventory/snapshots/{id}/export?format=csv|json` with revision-bearing filenames, lossiness headers on CSV, `404 snapshot_not_found` and `400 export_format_unsupported`.
- [x] 2.4 Test CSV re-import equivalence, JSON round-trip of lots and per-field provenance, determinism, headers, 404 and 400.

## 3. DFS — Backup and restore
- [x] 3.1 Enable rusqlite's `backup` feature; keep the schema at `user_version = 4`.
- [x] 3.2 Add `VACUUM INTO` backup to a temporary file in the data directory and `GET /api/backup` with `application/vnd.sqlite3` and the UTC filename.
- [x] 3.3 Add read-only validation (`backup_empty`, `backup_not_sqlite`, `backup_corrupt`, `backup_schema_too_new`, `backup_wrong_shape`) and `POST /api/backup/inspect`.
- [x] 3.4 Add `POST /api/backup/restore` with the confirm guard, pre-restore `VACUUM INTO` copy, SQLite backup-API copy into the live connection, forward migration and a summary that names the pre-restore path on success and on failure.
- [x] 3.5 Test that a backup opens as SQLite with matching counts, a restore reproduces counts, each rejection class leaves data intact, a pre-restore copy exists, and a v1 database migrates forward.

## 4. DFS — Interface
- [x] 4.1 Add API helpers and types for snapshot export, backup download, inspection and restore.
- [x] 4.2 Add the `DataPortability` panel to `/inventory`: export CSV/JSON, download backup, inspect, preview details, typed `RESTORE` confirmation, success path and error alerts.
- [x] 4.3 Keep existing states, focus moves, live announcements, keyboard order and 390 px behaviour; refresh history and the selected snapshot after a restore.
- [x] 4.4 Add Vitest coverage for export enablement, inspection preview, the confirmation guard and success/error states.

## 5. BFS — Cross-surface regression and docs
- [x] 5.1 Recheck callers, error envelopes, privacy (no logging of file content), responsive layout and accessibility.
- [x] 5.2 Write `docs/data-portability.md`; update README routes/runtime, `docs/inventory-import.md`, `docs/assets/capture-plan.md` and any stale claim.
- [x] 5.3 Extend the browser suite with an export/re-import journey, a backup→restore journey and the restore-confirmation guard.

## 6. Verification and evidence
- [x] 6.1 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test`, `npm run check`, `npm test`, `npm run build`, `npm run test:e2e`, `./verify-local.sh`.
- [x] 6.2 Run `node scripts/check-openspec-change-names.mjs`, `openspec validate inventory-export-and-backup --strict` and `openspec validate --all --strict`.
- [x] 6.3 Capture browser evidence with Chrome DevTools MCP at 1440×900 and 390×844: both exports, backup download, successful restore with the pre-restore path and the rejected-restore state; write `docs/evidence/data-portability-browser.md`; re-run Lighthouse on `/inventory`.
- [x] 6.4 Run the Forge full Gate and record its aggregate verdict.
- [x] 6.5 Archive without `--skip-specs`; commit everything except `HANDOFF.md`, then commit `HANDOFF.md` only; do not push.

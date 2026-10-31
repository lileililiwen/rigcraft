# Data portability browser and accessibility evidence

Captured: 2026-11-01, against the working tree based on revision `3006b9a`
(`Record use-case integrity evidence`). The implementation commit is recorded in
`HANDOFF.md`.

- Build under test: `npm run build` → `apps/web/dist`, served by `target/debug/rigcraft`.
- Service: `RIGCRAFT_ADDR=127.0.0.1:8791 RIGCRAFT_DATA_DIR=/tmp/opencode/rigcraft-evidence ./target/debug/rigcraft`
  (port 8080 is occupied on this host and was not used).
- Browser: Google Chrome through Chrome DevTools MCP (HeadlessChrome/154), not the Vite dev server.
- Data: the shipped sample (`GET /api/imports/sample`), previewed and committed through the
  documented API into a fresh data directory. Real product data, 10 lots, one revision.

## Captures

| File | Viewport | State proved |
| --- | --- | --- |
| `docs/assets/data-portability-export-desktop.png` | 1440×900 | Export and backup panel with revision 1 selected: CSV and JSON buttons enabled, backup button, restore form. |
| `docs/assets/data-portability-inspected-desktop.png` | 1440×900 | Inspected upload: integrity `ok`, schema 4, created 2026-10-31T20:19:34.474Z, 1 snapshot, revision 1, 10 records, 10 lots; “What will be replaced”; replace button disabled. |
| `docs/assets/data-portability-confirmed-desktop.png` | 1440×900 | After typing `RESTORE`: replace button enabled. |
| `docs/assets/data-portability-restored-desktop.png` | 1440×900 | Restored summary and the pre-restore path. |
| `docs/assets/data-portability-rejected-desktop.png` | 1440×900 | Non-SQLite upload rejected as `backup_not_sqlite`; committed revision 1 and its 10 records remain visible. |
| `docs/assets/data-portability-export-mobile.png` | 390×844 | The portability panel stacked at phone width. |
| `docs/assets/data-portability-rejected-mobile.png` | 390×844 | The rejected state stacked in the alert, no overflow. |

## Journey exercised

1. Seeded the shipped sample: preview `10 rows · 0 errors · 0 duplicates`, commit
   revision 1, totals `10 records`, `10 units`, `US$2,120.90`.
2. Opened the snapshot from history; the export panel reported
   `Ready: revision 1, 10 records.`
3. Clicked **Download CSV**. Status announced
   `Downloaded rigcraft-inventory-snapshot-r1-c43c1187.csv. CSV export is lossy: it
   omits lot identity, snapshot metadata and per-field provenance references. Use
   format=json for the authoritative artifact.`
   The served file had the exact import header
   `sku,category,model,quantity,price,currency,specs,source,sourceUrl`, 10 data rows and
   `catalog:manufacturer@2026-10-31` provenance.
4. Clicked **Download JSON**. Status announced
   `Downloaded rigcraft-inventory-snapshot-r1-c43c1187.json. This file contains the
   complete snapshot.` The file parsed as `format=rigcraft.inventory.snapshot`,
   `formatVersion=1`, `authoritative=true`, `lossy=false`, 10 records, and the CPU's
   model and first specification cited `catalog:manufacturer@2026-10-31` while its
   quantity cited `user`.
5. Clicked **Download backup**. Status announced
   `Downloaded rigcraft-backup-20261031T202047717937Z.sqlite3. It is a standard SQLite
   file containing the entire private inventory.` The file was 139,264 bytes; a
   standard `sqlite3` client reported `integrity_check = ok`, `user_version = 4`,
   1 snapshot and 10 records.
6. Uploaded that backup and clicked **Inspect backup**. The page announced
   `Inspected evidence-backup.sqlite3: integrity ok, schema 4, 10 records. Nothing has
   been replaced.` and showed the counts above. The replace button was disabled.
7. Typed `RESTORE`; the button became enabled. Clicked **Replace this inventory**.
   The page announced `Restored 1 snapshots and 10 records`, focused the
   `Inventory restored` heading, refreshed history to revision 1, and printed the
   pre-restore path
   `/tmp/opencode/rigcraft-evidence/inventory.sqlite3.pre-restore-20261031T202155709950Z`.
   That file existed on disk, passed `integrity_check`, and held 1 snapshot and 10
   records.
8. Uploaded `not-a-database.sqlite3` (23 bytes of text) and clicked **Inspect
   backup**. The `role="alert"` focused and showed
   `This backup cannot be restored`, code `backup_not_sqlite`; the committed revision
   and its 10 records stayed visible.
9. Checked the 390×844 viewport:
   `document.documentElement.scrollWidth === clientWidth === 390`.

## Keyboard and focus

- Tab order reached the snapshot history action, the backup download and the restore
  file input. After selecting a snapshot, focus moved to the
  `Committed snapshot · revision 1` heading, as the existing flow already does.
- The focused file input had a computed outline of `rgb(29, 78, 216) solid 3px`.
- After a rejected inspection, focus moved to the `role="alert"` region.
- After inspection, focus moved to `Backup inspected — nothing replaced yet`.
- After a successful restore, focus moved to `Inventory restored`.
- The replace action is disabled until the confirmation input matches `RESTORE`, so a
  single pointer click cannot restore; the check is repeated on submit.
- No control in the panel relies on colour alone: counts and states are text, the
  lossiness note is prose, and the error carries its code.

## Measured contrast

`npm run check` re-resolved the vendored tokens; `docs/evidence/token-contrast.json`
keeps its 16 measured pairs and its lowest ratio of 5.46:1. The panel reuses existing
tokens: body/secondary text on surface, primary and secondary buttons, the danger
button label on surface, the focus ring (7.29:1 on surface) and error text on surface.
No new colour token was added.

## Lighthouse

`chrome-devtools` Lighthouse, navigation mode, against the production build on
`http://127.0.0.1:8791/inventory`:

| Device | Accessibility | Best practices | SEO | Agentic browsing | Audits | Report |
| --- | --- | --- | --- | --- | --- | --- |
| Desktop | 100 | 100 | 100 | 100 | 51 passed, 0 failed | `docs/evidence/data-portability-lighthouse/report.json` |
| Mobile | 100 | 100 | 100 | 100 | 51 passed, 0 failed | `docs/evidence/data-portability-lighthouse-mobile/report.json` |

## Privacy review

- The service log contained the listen line and one restore line with aggregate
  counts (`snapshots=1`, `records=10`). No record, price, file content or backup bytes
  were logged.
- CSV and JSON exports and backup bytes are produced only by an explicit request to
  the local service; none was sent to an external host.
- The page states that the JSON export and the backup contain the complete private
  inventory, and that the pre-restore copy is permanent until the operator removes it.
  `docs/data-portability.md` states the same.

## Limits

- Lighthouse performance was not measured; this change adds no rendering work beyond
  the panel.
- Only Chrome was used. There are no visual-diff baselines.
- The browser suite covers the same journeys on every `npm run test:e2e` run; this
  document records the manual Chrome DevTools pass and its captures.

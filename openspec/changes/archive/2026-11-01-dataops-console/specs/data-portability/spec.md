# data-portability deltas

## MODIFIED Requirements

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

# inventory-import deltas

## MODIFIED Requirements

### Requirement: The Web flow is keyboard usable, responsive, and self-hosted
The `/inventory` route SHALL cover empty, selecting, parsing, review, row
correction, validation error, commit pending, committed, retry, earlier-revision
and restored states. When snapshot history is not empty, the route SHALL open the
newest revision automatically on first load, announcing it through the polite
status region and without moving focus, so the export and backup actions are
usable without a prior selection step. `Start a new import` SHALL close the open
snapshot and return the route to the empty import state, and it SHALL NOT be undone
by a later history refresh. The route SHALL use labelled controls, visible focus,
status announcements, non-color-only outcomes, a desktop layout and a stacked
mobile layout, and SHALL preserve entered form values on validation or network
failure. Inventory SHALL be stored outside the repository in a local data directory
chosen by `RIGCRAFT_DATA_DIR`, and CSV content and cost fields SHALL be excluded
from logs.

#### Scenario: Complete the journey without a pointer at narrow width
Given a phone-width viewport and keyboard-only operation, when the operator previews, corrects, resolves duplicates, and commits, then every action is reachable, the focus order is visible, and no content overflows horizontally.

#### Scenario: Preserve work and offer retry after a network failure
Given a selected file or a parsed preview, when the local service becomes unreachable, then the UI announces the failure, keeps the current values, and offers a retry action.

#### Scenario: The newest revision opens on arrival
Given at least one committed snapshot, when `/inventory` is loaded with no preview open, then the newest revision's summary, records and export actions are available without activating a history control, and the first Tab stop is still the skip link.

#### Scenario: Starting a new import closes the open snapshot
Given an automatically opened snapshot, when the operator activates `Start a new import`, then the snapshot view closes, the import panel is ready for a new file, and the view does not reopen on its own.

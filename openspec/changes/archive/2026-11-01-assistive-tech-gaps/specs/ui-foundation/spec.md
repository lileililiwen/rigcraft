# ui-foundation deltas

## ADDED Requirements

### Requirement: Every form is a named landmark
Every `<form>` the Web application renders SHALL have an accessible name. Where a
heading introduces the form, the form SHALL point at that heading's `id` with
`aria-labelledby`; where no heading introduces it, the form SHALL carry a label
that is not displayed. The inventory import controls SHALL be a `<form>` element
named by the "Import a CSV" heading, not a bare `<section>`. Naming a form SHALL
NOT add visible copy, and the form's class, children, position and submit
behaviour SHALL be unchanged.

#### Scenario: The inventory import is a named form
Given `/inventory`, when the accessibility tree is read, then a `form` landmark named "Import a CSV" contains the file, currency and preview controls.

#### Scenario: The catalog forms are distinguishable
Given `/catalog`, when landmark navigation lists the forms, then one is named "Upload a catalog" and one is named "Search".

#### Scenario: The recommendation and check forms are named
Given a selected snapshot on `/recommend`, when the forms are listed, then the budget form is named "3. Budget and constraints" and the assistant transport form is named "Assistant provider settings"; given a selected snapshot on `/builds/check`, then the parts form is named "2. Select parts".

#### Scenario: A form that appears after an interaction is named too
Given an inspected backup, a previewed row being corrected or an open record correction, when its form is inspected, then it has an accessible name derived from its own heading or from a label that is not displayed.

#### Scenario: No form is left unnamed
Given every component that renders a form, when the rendered tree is searched for `form` landmarks, then each one has a non-empty accessible name.

### Requirement: A static hint is not a live region
A sentence that describes the product rather than a change SHALL NOT be exposed as
a live region. The export and backup hint "Exports and backups are created by this
local service only." SHALL be exposed as a note with its text, styling and
position unchanged. Results of real actions — a download, an inspection, a
restore — SHALL remain announced through a polite live region, and failures SHALL
remain `role="alert"` with their focus move.

#### Scenario: The hint is a note on load
Given `/inventory` on load, when the accessibility tree is read, then "Exports and backups are created by this local service only." is a note and is not inside a status or other live region.

#### Scenario: An action result is still announced
Given the export panel, when a download completes, then the downloaded filename is the text of a polite status region and the visible line shows the same sentence.

### Requirement: Every route's main region has an accessible name
Each of the six client routes SHALL render exactly one `main` landmark, and that
landmark SHALL be named by the route's visible `h1` through `aria-labelledby` to a
stable `id` on that heading. The name SHALL appear in the accessibility tree on
the real page, `id="main"` SHALL remain so the skip link still targets it, and no
second `main` or additional landmark SHALL be introduced.

#### Scenario: The landmark carries the page name
Given each client route, when the accessibility tree is read, then `main` is announced with the same text as its `h1`: "Inventory", "Catalog", "Compatibility check", "Build recommendation" or "Diagnose an existing build".

#### Scenario: One main per route
Given any client route, when the document is queried for `main`, then exactly one element matches and it is focused after the skip link is activated.

### Requirement: Result headings that take focus are addressable
Every heading that receives programmatic focus after an asynchronous result SHALL
carry a stable `id`. The catalogue upload result heading SHALL carry
`id="catalog-result-title"`, matching the existing `#result-title`,
`#recommend-result-title`, `#diagnosis-result-title`, `#review-title` and
`#snapshot-title`. The heading's text and its focus move SHALL NOT change.

#### Scenario: The accepted catalog heading is addressable
Given an accepted catalog upload, when focus settles, then the focused element is the `h3` with `id="catalog-result-title"` and the text "Catalog accepted".

#### Scenario: A test can address it without a ref
Given the same page, when the result heading is looked up by `id`, then exactly one element is found and it is the same element that received focus.

### Requirement: The silent selection count stays a recorded human decision
The `/builds/check` selection count SHALL remain outside every live region
(`aria-live="off"`) so that changing a selection does not queue an announcement on
top of the check result. The assistive-technology evidence SHALL record this as a
deliberate decision with its reason and SHALL leave it to the human
screen-reader pass to confirm, rather than claiming the count is correctly
handled.

#### Scenario: The counter is not a live region
Given a selected part on `/builds/check`, when the selection count element is inspected, then it carries `aria-live="off"` and no `role` that announces changes.

#### Scenario: The decision is recorded, not resolved
Given `docs/evidence/assistive-technology-review.md`, when the counter is read about, then it is listed as a deliberate decision still open to the human pass, with the reason it is silent.

### Requirement: The assistive-technology gap evidence names the fix
`docs/evidence/assistive-tech-gaps.md` SHALL record, from the running production
build, an `ariaSnapshot()` of `/inventory` and `/recommend` that shows the named
`form` and `main` in the accessibility tree, and SHALL name the accessible names
that were added. `docs/evidence/assistive-technology-review.md` SHALL mark the
role and name gaps that this change fixed, keep the open ones, and keep the
statement that it is not a screen-reader pass together with the human script.

#### Scenario: The proof is an accessibility tree, not a selector
Given `docs/evidence/assistive-tech-gaps.md`, when a capture is read, then it shows the form and main landmarks with their names inside an `ariaSnapshot()` extract rather than describing a CSS selector.

#### Scenario: The review separates fixed from open
Given the review document, when the role and name gaps are read, then each gap states whether it is fixed and with which name, and the screen-reader refusal and human script are still present.

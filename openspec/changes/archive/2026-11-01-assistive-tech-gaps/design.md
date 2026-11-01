# Design: Name the forms, the main regions and the result heading

## Context

Every route is a client-rendered page inside one shell: `SiteHeader` renders the
skip link, the `banner` and `navigation "Primary"`, and the page component renders
`main#main` (`tabIndex={-1}`) with one `h1.page-title`. Each page also renders a
`div.status-line` with `role="status"` for its own announcements, plus a
`DataPortability` panel on `/inventory` and `/` whose status line starts with a
static sentence.

Playwright drives the built bundle in Chrome, Firefox, WebKit (smoke subset), and
twelve committed pixel baselines are captured from `/inventory`, `/catalog`,
`/builds/check`, `/recommend`, `/diagnose` and the 404 at 1440×900 and 390×844.
Vitest renders each component with mocked API calls.

## Decision 1 — names come from the heading that already introduces the control

An accessible name is not copy. Each form points at the `id` of the heading that
already sits above it, so the name matches what an operator reads and no string is
added:

| Route | Form | Accessible name | Source |
| --- | --- | --- | --- |
| `/inventory` | import panel | `Import a CSV` | `h2#import-title` |
| `/inventory` | restore inspection | `Restore from a backup` | new `id` on the existing `h3` |
| `/inventory` | restore confirmation | `Confirm replacing the inventory` | label that is not displayed |
| `/inventory` (after a preview) | row correction | `Correct row 3` | new `id` on the existing `h3` |
| `/inventory` (after a commit) | record correction | `Create a new revision from <SKU>` | new `id` on the existing `h3` |
| `/catalog` | upload | `Upload a catalog` | `h2#upload-title` |
| `/catalog` | search | `Search` | `h2#search-title` |
| `/builds/check` (after a snapshot loads) | parts and check | `2. Select parts` | `h2#parts-title` |
| `/recommend` (after a snapshot loads) | budget and constraints | `3. Budget and constraints` | `h2#request-form-title` |
| `/recommend` (assistant opened) | transport | `Assistant provider settings` | label that is not displayed |

The four headings that introduce a form but had no `id` gain one on themselves;
nothing is renamed. Where no heading names the form — the restore confirmation
and the assistant transport — the name is supplied by a label that is not
displayed, because inventing a visible heading would change the page.

The inventory import panel is a `<section aria-labelledby="import-title">` today
with no `<form>` element inside it. It becomes a `<form className="panel"
aria-labelledby="import-title">` with the same class, children and submit handler
(which already calls `preventDefault` through its guarded button). A `form` and a
`section` are both block boxes with no user-agent margin, and `.panel` owns the
padding, border and background, so the pixels do not move; what changes is that
the region landmark becomes a named form landmark.

## Decision 2 — the static hint and the action results are two elements

The export/backup panel keeps one visible line. Its initial sentence is static:
it describes the product, not a change, so it is exposed as `role="note"` and is
not announced on load. The same line also reports real outcomes ("Downloaded
…", "Inspected …", "Restored …"), which must stay announced.

Two properties are required at once: an element that is a live region when an
action result arrives must already exist in the accessibility tree, and the
static sentence must not be one. One element cannot do both by swapping its
`role`, because a region that only becomes `role="status"` and receives its text
in the same commit is not reliably announced.

So the visible `.status-line` becomes `role="note"` and keeps its text, class,
`data-busy` and position; a sibling `<p className="visually-hidden"
role="status" aria-live="polite">` carries the latest action result. Both are
updated by one helper, so the visible line and the announcement never disagree.
`.visually-hidden` is already used by the preview table's legend and is
`position: absolute` at 1×1 px, so it cannot move a pixel or a baseline.

`aria-busy` moves off the visible line (it only means something on a live region)
and `data-busy` stays for the spinner. The error path keeps `role="alert"` and
its focus move.

## Decision 3 — `main` is named by the heading it already contains

Each `h1.page-title` gains an `id` (`inventory-page-title`, `catalog-page-title`,
`check-page-title`, `recommend-page-title`, `diagnose-page-title`) and the route's
existing `main#main` gains `aria-labelledby` pointing at it. The name is therefore
the visible heading — `Inventory`, `Catalog`, `Compatibility check`,
`Build recommendation`, `Diagnose an existing build` — and cannot drift from it.
`id="main"` stays, because the skip link targets it and `e2e/smoke.spec.ts` and
`e2e/keyboard-responsive` assert that activating the skip link focuses that
element. Exactly one `main` is rendered per route; no new landmark is added.

## Decision 4 — one stable id for the catalogue result heading

The upload result heading is focused through a ref today, so it is reachable but
not addressable. It gains `id="catalog-result-title"` beside the ref, matching
the naming of `#result-title`, `#recommend-result-title`,
`#diagnosis-result-title`, `#review-title` and `#snapshot-title`. The focus move
and the heading text are unchanged.

## Decision 5 — the silent counter stays a human decision

`/builds/check` renders its selection count in a `span.small.muted` with
`aria-live="off"`. Announcing every checkbox change on top of the check result
would be noisy, and whether the silence costs a keyboard operator context is
exactly what a screen-reader pass decides. Nothing in this change touches the
element; the review keeps the item open and says why.

## Tests

- Vitest, per component: `getByRole("form", { name })` for every form, the
  catalogue result heading's `id`, `getByRole("main", { name })` for each page,
  and the static hint as `note` and not `status` on the export panel.
- Playwright, `e2e/landmarks.spec.ts` in the chrome project: each of the six
  routes exposes `main` with its expected name and no unnamed `form`; the named
  forms resolve after the interactions that render them; the skip link still
  focuses `main`.
- `e2e/visual.spec.ts` runs unchanged. No baseline is regenerated; if a naming
  change moved a pixel, the suite fails and the change is wrong.

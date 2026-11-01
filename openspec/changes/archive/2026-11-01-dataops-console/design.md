# Design: Present inventory and catalog as an operator console

## Context

`InventoryPage` keeps its whole state machine in one component: a file picker, a
review, a committed `SnapshotPanel`, a one-line history list and the
`DataPortability` panel. The snapshot is only set by `viewSnapshot`, which a
`View {shortId}` button in the history list triggers, or by a commit. Nothing opens
it on load, so `DataPortability` receives `snapshot === null` after a reload and
disables both export buttons; its only explanation is "Open or commit a snapshot
first."

`CatalogPage` renders its registry and search hits with `table.responsive-table`,
which is already the shell's responsive card layout, but the registry row buries
the licence, version and counts between a name cell and an action cell, and the
search result omits the dataset version that provenance will use.

Neither page adds a colour: both consume the measured dark token layer through
semantic aliases, and `app.css` already owns the table, badge, panel and
announcement surfaces this change reuses.

The reference (`docs/design-program.md`, package 4) shows a dense hardware table
under a count strip, with a status chip per row and a monospaced numeric column.
Only that structure is taken. Its scraping, ETL, GitOps and anti-crawl panels are
product scope RigCraft has declined, and none of their wording, markup, class
names or hex values appear here.

## Decision 1 — the open snapshot is announced by one bar

A new `section.console-bar` renders after the live status line whenever a snapshot
is open, and only then. It carries:

- a decorative caps label `Open snapshot` (`aria-hidden`), repeated by the
  section's heading, and a status chip: `✓ Latest revision` or `? Earlier revision`
  — symbol, word and border, never colour alone;
- a heading that names the surface;
- a `dl.console-stats` strip with four cells: `Revision`, `Records`,
  `Units in stock`, `Stock value`. Values render in the monospace stack with
  tabular numerals, so a revision number and a money amount do not jitter;
- the source file name and the commit time as `<time datetime>`.

The bar deliberately does **not** reuse `dl.summary-grid`. `e2e/inventory.spec.ts`
reads `dl.summary-grid dd` positionally for the preview and the committed
snapshot, so a second grid before `SnapshotPanel` would silently shift every
assertion. It also does not repeat the record table: `SnapshotPanel` remains the
one place records, provenance and corrections live.

`SnapshotPanel` itself is unchanged apart from replacing a leftover inline style
with a class, so every record field, `factsLabel` chip, `Show provenance`
disclosure, correction form, stale-revision note and focus target survives.

## Decision 2 — the newest revision opens on first load

After the first `GET /api/inventory/snapshots`, if the list is non-empty the page
calls the same `viewSnapshot(id)` path it uses for a `View` click, with an
`initial` flag. This is what makes export truthful: `DataPortability` receives a
real snapshot immediately, so `Download CSV` and `Download JSON` are enabled
without the operator discovering the history button first.

Two rules keep it honest:

- It never steals focus on load. The `phase === "committed"` effect that focuses
  `#snapshot-title` after a user action is suppressed for the initial open, so the
  first Tab stop on `/inventory` is still the skip link. It announces through the
  existing polite status region instead.
- It runs once. `Start a new import` closes the view and the flag is not reset, so
  a later history refresh cannot reopen what the operator just closed.

Rejected: keeping the explicit open and only rewriting the copy. It leaves a
first-time user with a committed snapshot scrolling past the record table and the
history list to find a button, and the design program asks for export in one
obvious action. The copy is fixed anyway, because a page with no snapshot still
has to explain the control.

## Decision 3 — import, export and restore stay separate panels

The import panel keeps its own `<section class="panel">` with the console header
treatment (caps label + heading on one line) and stays rendered whenever no
preview exists. That means it is present before the first snapshot, after a commit
and after `Start a new import`; a committed import clears the selected file so the
panel shows its empty state rather than the file just committed.

`DataPortability` keeps its export, backup and guarded restore sections and gains
the same header treatment. Its disabled copy names the controls:
"No snapshot is open. Choose View on a revision in Snapshot history, or commit an
import." — and the enabled line reports `plural(records, "record")` instead of
`1 records`.

Panel order on `/inventory`: summary bar → alert → open snapshot → import →
snapshot history → export and backup. The state is first, the two action panels are
distinct, and nothing that used to be above the fold moves below a table except
the history that was already there.

## Decision 4 — snapshot history is a card grid

`ul.history-list` becomes a responsive grid of `li.history-card`, one card per
revision, each with a `Revision N` heading, an optional `✓ Viewing now` chip, a
`dl` of `Committed` (ISO 8601 UTC `<time>`), `Records` and `Stock value`, the
revision note when present, and the existing `View {shortId}` button. The current
revision gets a primary border and `aria-current="true"` on its button, exactly as
before.

The list stays a list, not a `<table>`. Two tables on `/inventory` would break
`page.locator("table.responsive-table")` in `e2e/inventory.spec.ts` and
`e2e/portability.spec.ts`, which read the record table positionally; a card grid
carries the same revision, timestamp, count, value and action without that
collision.

## Decision 5 — the catalog registry and search are dense console tables

The registry keeps its `table.responsive-table` and caption, and gains:

- a count strip above the table — `Datasets`, `Parts`, `Active` — computed only
  from the datasets already fetched;
- a catalog provenance chip (`▣ Catalog`) beside the dataset name, its source URL
  as a wrapping `tag`, and its import time;
- explicit `Licence`, `Version`, `Parts`, `Categories`, `Active` and `Actions`
  columns, so the facts that decide reuse are column-aligned instead of buried in
  prose. `Active`/`Inactive` keeps its symbol + word + border chip.

Search results keep the caption, rank order, dataset attribution, licence and the
`View part` action, and add an ordinal `Rank` column plus the dataset `Version`
that applied provenance will record. Paging is still `limit=20, offset=0`; rank is
the position in the returned order, which is the ranking the API defines.

The upload panel, the accepted report and the refusal alert are unchanged in code,
code strings and wording; the report gains the same caps header and keeps
`Catalog accepted`, the dataset name, `replaced`/`added`, `partCount`, the
per-category counts, licence, version, source URL and the
`catalog:<name>@<version>` provenance line. Every state — no dataset, refused
document, accepted report, empty search — stays reachable and announced.

## Decision 6 — four actions are guarded, and the server stays authoritative

| Control | Disabled while | Reason shown |
| --- | --- | --- |
| `Preview CSV` | no file, or the currency is not three ASCII letters | `Choose a CSV file to enable Preview CSV.` / `Enter a three-letter ISO 4217 currency code to enable Preview CSV.` |
| `Search catalog` | the query is empty after trimming | `Enter a SKU, MPN, model or word to enable search.` |
| `Match this list` | the pasted list is empty after trimming | `Paste at least one part above to enable matching.` |
| `Diagnose this build` | no reviewed line and no hand-added part | `Add a matched line or a part by hand to enable the diagnosis.` |

Each disabled control points at its reason with `aria-describedby`, so the reason
is announced when the control is reached rather than rendered as an after-the-fact
alert. The handler guards stay in place: the disabled state is an affordance, and
the request (or the client-side review check) is still what decides. No action that
currently succeeds is disabled — a file and a valid currency still enable Preview,
a one-character query still searches, and an unresolved review still runs
`Diagnose this build` so the existing `review_incomplete` refusal keeps working.

`isValidCurrencyCode` joins `inventory.ts` so the form and the preview handler
agree on one predicate; the server's `currency_format`/`currency_required` checks
are untouched.

## Decision 7 — real examples and real plurals

The `/diagnose` textarea placeholder and the parts-list help text use the shipped
sample's own parts — `AMD Ryzen 7 7800X3D`, `NVIDIA GeForce RTX 4070 Founders
Edition`, `Corsair RM750e` — and the manual-entry specification example uses the
sample's `socket=AM5`, so an operator can paste the example against the sample and
match it. No example claims a part the sample does not ship.

Counts the pages compose go through `plural`/`pluralWord`: catalog
`searchResult.total` ranked hits, `DataPortability`'s record and snapshot counts,
and the inventory history card counts. Engine and export prose is untouched, and
so is `continuationSummary`'s asserted `10 rows · 0 errors · 0 duplicates` line.

## Decision 8 — no new visual vocabulary

The change adds classes, not tokens: `.console-bar`, `.console-bar__head`,
`.console-bar__title`, `.console-stats`, `.history-card*`, `.panel__head`,
`.catalog-table`. Every property resolves an existing semantic alias
(`--color-surface`, `--color-surface-raised`, `--color-muted`, `--color-border`,
`--color-primary`, `--color-muted-foreground`, `--font-mono`, `--radius-md`,
`--radius-lg`). No colour literal, no inline style, no `!important` outside the
reduced-motion block, no new dependency, no webfont and no animation are added;
the existing transitions stay ≤160 ms and reduced-motion safe.

## Shared infrastructure assessed

| Candidate | Evidence | Decision |
| --- | --- | --- |
| Shared token kit | `vendor/tokens.css`, pin `7ca6b795…` | consume unchanged; no edit, no re-pin |
| Brand layer | `brand.css`, 40 measured pairs | consume semantic aliases only |
| Shared plural helper | `apps/web/src/plural.ts` | reuse for every new count; no second helper |
| Responsive table + badge shell | `app.css` `.responsive-table`, `.badge` | reuse; extend with two table classes |
| Playwright suite | `e2e/inventory.spec.ts`, `e2e/catalog.spec.ts`, `e2e/portability.spec.ts` | extend, no assertion removed |
| Inventory/catalog/backup APIs | `GET /api/inventory/snapshots[/{id}]`, `/api/catalog/*`, `/api/backup` | unchanged; the page only calls what already exists |

## Alternatives considered

- **A dedicated `/inventory/{id}` route for a snapshot.** Rejected: it is a route
  and a server-shape change for a presentation problem, and it would split the
  import review from the state it produces.
- **Auto-opening by picking `history[0]` without a `GET`.** Rejected: the summary
  bar needs real totals and the record table needs real records; the summary row
  does not carry per-record provenance.
- **A second `dl.summary-grid` for the bar.** Rejected: it would shift the
  positional `dd` assertions the existing suite relies on.
- **Converting history into a `<table>`.** Rejected: it collides with the
  positional `table.responsive-table` locators.
- **Disabling `Diagnose this build` whenever the review is unresolved.** Rejected:
  the existing `review_incomplete` alert is asserted text and the operator must be
  able to press it to learn what is missing.
- **A browser catalog editor.** Rejected: out of scope, and the document is the
  contract.

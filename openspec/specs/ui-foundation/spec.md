# ui-foundation Specification

## Purpose

The Web application owns exactly one brand token layer on top of the generated,
hash-pinned shared kit, and its contrast claims are measured rather than
asserted: the effective theme resolves across both stylesheets, every text and
UI pair that the interface uses is scored against WCAG 2.2 AA, and the record
names both source hashes. This capability also keeps the shell honest — native
controls follow the declared colour scheme, focus stays visible on every
surface, a table caption never collapses inside the responsive card layout, the
skip link never peeks above the header, and the built application serves an
icon.
## Requirements
### Requirement: One app-owned brand token layer
The Web application SHALL declare its palette and typography in exactly one
app-owned stylesheet that is imported after the vendored shared tokens and before
the component stylesheet. That stylesheet SHALL declare custom properties only,
SHALL NOT redefine a vendored primitive, and the vendored
`apps/web/src/styles/vendor/tokens.css` SHALL remain byte-identical to the
`.project.json` `pin_sha256`. Component rules SHALL reference semantic or
component variables and SHALL NOT contain a colour literal.

#### Scenario: The vendored kit is untouched
Given the repository after this change, when `apps/web/src/styles/vendor/tokens.css` is hashed, then it matches the pinned value and the conformance scope still resolves.

#### Scenario: No colour literal in a component
Given the component stylesheet and the component sources, when they are searched for a hex, `rgb()` or `hsl()` literal, then none is found outside the brand token file.

#### Scenario: Brand values win by cascade order
Given the brand file declares `--color-surface`, when the page renders, then a component using `var(--color-surface)` resolves the brand value and not the vendored light value.

### Requirement: The effective theme is measured, not asserted
`scripts/check-contrast.mjs` SHALL resolve the ordered token sources (vendored
then brand) so that the later declaration wins, SHALL report every source with
its `sha256`, and SHALL measure the effective dark pairs: body and muted text on
each of the three surfaces, primary/success/warning/danger on the raised
surfaces, chip text on each chip tint, and the border and focus ring on each
surface. The check SHALL exit non-zero when any pair misses its WCAG 2.2 AA
minimum, and the generated `docs/evidence/token-contrast.json` SHALL be the
record the UI conformance gate reads.

#### Scenario: A regression fails the build
Given a brand value that drops a text pair below 4.5:1, when `npm run check` runs, then the checker reports the failing pair and the command exits non-zero.

#### Scenario: The evidence names both sources
Given a passing run, when the generated report is read, then it lists the vendored and brand files with their hashes and the measured pairs.

#### Scenario: Decoration is not measured as text
Given a decorative border that uses `color-mix()` or alpha, when the report is generated, then that value is not presented as a text pair and no pair is claimed for it.

### Requirement: The brand mode is legible in native UI and focus
The root SHALL declare `color-scheme: dark` and set `accent-color` for native
controls. Focus SHALL remain a visible 3 px ring with a 2 px offset and SHALL
meet 3:1 against the page, surface, raised surface and chip tints. Status SHALL
continue to be expressed as icon plus word plus border, never colour alone.

#### Scenario: Native controls follow the theme
Given a dark brand mode, when a select, checkbox, radio or scrollbar renders, then it is legible against its surface and not a light-theme native widget.

#### Scenario: Keyboard focus is visible on every surface
Given the keyboard-only journey, when focus lands on the skip link, a nav item, a control and a chip, then each ring is visible at 3:1 or better against its own background.

### Requirement: Shell chrome never leaks or collapses
A table caption inside the responsive card layout SHALL occupy the full width of
its table block and SHALL NOT shrink to a word-per-line column. The skip link
SHALL be entirely outside the viewport while unfocused and SHALL be fully
visible when focused.

#### Scenario: Captions at phone width
Given a table with a caption at 390 px, when the page renders, then the caption's rendered width exceeds 150 px and its text does not stack one word per line.

#### Scenario: The skip link does not peek
Given any route at rest, when the page renders, then the skip link's bottom edge is at or above the top of the viewport and no fragment of it is painted above the header.

#### Scenario: The skip link is usable
Given the page at rest, when the user presses Tab once, then the skip link becomes fully visible and activating it moves focus to the main region.

### Requirement: The application serves an icon
The built bundle SHALL contain a favicon, `index.html` SHALL link it, and a
request for `/favicon.ico` SHALL return it rather than 404.

#### Scenario: No icon 404 in the console
Given a fresh page load, when the network log is inspected, then no request for the icon fails.

### Requirement: The recommendation route is a three-region workbench
The `/recommend` route SHALL render its request surface, its candidate options and
its verdict into one workbench with three named regions — constraints, options and
verdict — in that order. At a viewport of 1100 px or wider the regions SHALL be
laid out as a named-area grid with those three columns and a maximum content width
of 88 rem. Below 1100 px the regions SHALL stack in one column while keeping the
same source order. The document SHALL NOT have horizontal overflow at 320, 390, 768
or 1440 px.

#### Scenario: Three regions share one screen at desktop width
Given a generated recommendation at 1440 px, when the page renders, then the constraints region, the options region and the verdict region are all laid out and their grid areas are `constraints`, `options` and `verdict`.

#### Scenario: One column below the breakpoint
Given the same report at 390 px, when the page renders, then the three regions stack vertically in source order and each spans the full content width.

#### Scenario: No page overflow at any audited width
Given `/recommend` at 320, 390, 768 and 1440 px, when the document is measured, then `scrollWidth` equals `clientWidth` at every width.

### Requirement: An option is presented as a specification module
Each candidate SHALL be presented as a specification module containing a
decorative uppercase label that is hidden from assistive technology and repeated
in the module's accessible heading, a heading that names the option and its rank,
a status chip carrying a symbol, a word and a border, and the total, budget delta,
stock units and score rendered in the monospace stack with tabular numerals. The
module SHALL keep every part, price, stock line, score dimension, reason sentence,
follow-up, alternative, rule verdict, check report identifier and version string
the engine emits.

#### Scenario: The caps label adds no duplicate reading
Given a rendered option, when the accessibility tree is inspected, then the decorative caps label is not exposed and the module heading names the option.

#### Scenario: Numerals are monospaced and tabular
Given a rendered option, when the total, budget delta, stock units and score are measured, then they resolve to the monospace font stack with `font-variant-numeric: tabular-nums`.

#### Scenario: Status is never colour alone
Given an option whose verdict is unknown, when its chip is inspected, then the chip contains the unknown symbol, the word `Unknown` and a border.

#### Scenario: No engine fact is dropped
Given a generated option, when the module is read, then every part row, score dimension, reason, unknown follow-up, rule verdict and the check report identifier that the report carries for that option are present.

### Requirement: The decision is visible beside the options
The workbench SHALL show a verdict region carrying the report status, the top
option's budget delta, stock units and score, and the rule-matrix summary of
compatible, conflict and unknown counts. It SHALL state the use-case completeness
rule that applied, keep the exact sentence `Gaming requires a discrete graphics
card` as a heading when gaming required one, and keep the exact statement that the
reported builds were not padded to three. Report counts SHALL be labelled so
`distinct builds` and `valid candidates` are distinguishable.

#### Scenario: The verdict explains the count
Given a report with two valid builds and one distinct option, when the verdict region renders, then it shows a labelled counter line reading `2 valid candidates → 1 distinct build`.

#### Scenario: The rule heading keeps its contract text
Given a gaming report, when the verdict region renders, then the heading `Gaming requires a discrete graphics card` and the no-padding sentence are present with their existing text.

#### Scenario: No option to score
Given a no-solution report, when the verdict region renders, then it states the status and that there is no option to score, and the binding constraint remains readable in the options region.

### Requirement: The optional assistant is collapsed and its key field has a form owner
The optional assistant SHALL be a native `<details>` element that is closed on
first render, so its expansion state is exposed without script. Opening it SHALL
reveal the no-key state, the consent copy, the transport fields and the
`Continue without AI` action. The provider key, base URL and model inputs SHALL
reside inside a `<form>` that prevents its own submission, so a password input is
never outside a form, and the key SHALL remain request-scoped and never persisted.
`Continue without AI` SHALL move focus to the budget field.

#### Scenario: Closed by default and expandable
Given the recommendation page with a snapshot loaded, when it first renders, then the assistant `<details>` is closed, and activating its summary opens it and exposes the no-key explanation.

#### Scenario: The key field is inside a form
Given the opened assistant panel, when the provider key input is inspected, then its form owner is the panel's transport form and that form does not navigate.

#### Scenario: Continuing without AI reaches the budget
Given the assistant panel, when `Continue without AI` is activated, then focus is on the total budget field and no provider request was made.

### Requirement: The compatibility check is presented as a rule console
The `/builds/check` result SHALL open with one summary bar carrying the overall
status, the count of rules passed, the count of conflicts, the count of unknown
rules and the count of selected parts that no evaluated rule covered. The summary
SHALL remain visible above the rule matrix while the result is read at desktop
width and SHALL become static at phone width. The rule matrix SHALL present every
rule result in one row or one labelled card containing a verdict chip of symbol,
word and border; every compared value with its field; the provenance source,
reference and observation time for each value; and the rule's follow-up. Every
rule, value, explanation, method, rule identifier and rule version the report
carries SHALL remain rendered, and the part selector SHALL keep its source order.

#### Scenario: The summary names all five counts
Given a completed check, when the result renders, then the summary bar names the overall status, rules passed, conflicts, unknown rules and parts not evaluated, each as a number or a labelled status.

#### Scenario: A rule row carries its verdict, values and source
Given any rule result, when its matrix row is read, then it contains a chip with the status symbol and word, every compared field and value, the provenance source and reference for each value, and the follow-up sentence when one exists.

#### Scenario: Status is never colour alone
Given an incompatible, an unknown and a compatible rule, when their chips render, then each contains its symbol, its word and a border in addition to its colour.

#### Scenario: No rule or value is dropped
Given a report with a rule whose evidence value is absent, when the matrix renders, then the row still states `Not provided`, keeps the rule identifier and version, and the total row count equals the report's rule count.

### Requirement: The diagnosis uses the same console language
The `/diagnose` result SHALL present the four advice groups — must replace,
should upgrade and verify first, fine as is, and not verified — as console group
cards, each carrying a status chip of symbol, word and border beside its existing
heading and count. The described-build evidence table and the upgrade plan SHALL
keep every part, quantity, source, rule identifier, price, follow-up, rejected
candidate and validation state. The stable numbered skeleton, its section
headings, ids and statuses SHALL NOT be renumbered or reordered.

#### Scenario: Each group is a card with a status chip
Given a diagnosis with all four advice groups, when the result renders, then each group is a card whose header contains the unchanged heading and count plus a symbol-and-word chip.

#### Scenario: Every diagnosis fact survives
Given a diagnosis with steps, rejected candidates, follow-ups and an unverified part, when the result renders, then every step, price, rule identifier, rejected candidate, follow-up and unverified part is still present.

#### Scenario: The skeleton does not move
Given the diagnosis page before and after a result, when the numbered headings are read, then they are still `1. Choose an inventory snapshot` through `5. Read the diagnosis` in that order with the same ids.

### Requirement: Console counts use real plural forms
The check and diagnosis interfaces SHALL render every count they compose through
one shared pluralisation helper that yields the singular noun for a count of one
and the plural noun otherwise. No check-composed or diagnosis-composed string
SHALL contain an `(s)` pseudo-plural. Recommendation prose and exported report
text SHALL be rendered verbatim; the service owns their grammar and the page
SHALL NOT paraphrase them.

#### Scenario: Singular and plural forms follow the count
Given the shared helper, when it is called with one unit and with two units, then it returns `1 unit` and `2 units`.

#### Scenario: No page-composed pseudo-plural remains
Given a completed check and a completed diagnosis at any count, when the rendered page text is searched for `line(s)`, `part(s)`, `unit(s)`, `rule(s)` or `conflict(s)`, then none is found in text the page composed.

### Requirement: The inventory route is an operator console
`/inventory` SHALL open with the state of the snapshot it has open: one summary bar
naming the revision, record count, units in stock, stock value and whether that
revision is the latest, followed by its source file and commit time. The bar SHALL
carry a status chip of symbol, word and border and SHALL NOT replace the committed
snapshot panel, which SHALL keep every record, field, provenance chip, disclosure,
correction entry point and announcement it had. Snapshot history SHALL be a
readable card or table listing every revision with its revision number, timestamp,
record count, stock value and an explicit `View` action, marking the revision that
is currently open. Import, export and backup, and restore SHALL remain distinct
panels with the console header treatment, and the import panel SHALL stay reachable
when a snapshot is open.

#### Scenario: The summary names the snapshot before the records
Given a committed snapshot with ten records, when `/inventory` renders, then the summary bar states the revision, `10` records, the units in stock and the stock value, and the records table below it still lists every record with its provenance.

#### Scenario: History is a readable list of revisions with an action
Given three revisions, when the history panel renders, then each revision appears once with its revision number, ISO 8601 UTC timestamp, record count and stock value, and each has its own `View` action while the open revision is marked.

#### Scenario: The panels stay distinct
Given an open snapshot, when the page renders, then the import panel, the snapshot history panel and the export and backup panel are separate labelled regions, and the import file input is still on the page.

#### Scenario: Status is never colour alone
Given an open snapshot that is not the newest revision, when the summary bar renders, then its chip contains a symbol, a word and a border in addition to its colour.

### Requirement: The catalog route is a registry console
`/catalog` SHALL present the upload panel, the dataset registry and the search
results as console surfaces. The registry SHALL be a table of stored datasets with
the dataset name and a catalog provenance chip, the licence, the version, the part
count, the per-category counts, the source URL, the active state as a symbol plus
word plus border chip, and its actions, together with a count summary of datasets,
parts and active datasets. Search results SHALL stay in rank order and SHALL name
the rank, the dataset, its licence and its version for every hit, while keeping the
part detail view. The accepted report and the refusal alert SHALL keep their
headings, codes, row and field wording, and every state — no dataset, refused
document, accepted report and empty search — SHALL stay reachable and announced.

#### Scenario: The registry shows the facts that decide reuse
Given two stored datasets, when the registry renders, then each row shows its licence, version, part count, category counts, source URL and active state, and the summary states the dataset, part and active counts.

#### Scenario: Search keeps rank and attribution
Given a ranked search result, when the table renders, then each hit carries its position in the ranking, its dataset name, licence and version, and the `View part` action still opens the stored part with its provenance.

#### Scenario: Refusal still names the row
Given a refused document, when the page renders, then the alert keeps its `Catalog refused` heading, the documented code, and the row index and field, and no dataset is added.

### Requirement: Console actions are disabled until their input is valid
The four actions that need input — `Preview CSV`, `Search catalog`, `Match this
list` and `Diagnose this build` — SHALL be disabled while their input is missing,
and each SHALL reference an accessible reason that names what is required, so the
operator learns the requirement before the click rather than from an error
afterwards. `Preview CSV` SHALL require a chosen file and a three-letter currency
code; `Search catalog` SHALL require a non-empty query; `Match this list` SHALL
require a non-empty pasted list; `Diagnose this build` SHALL require at least one
reviewed or hand-added part. No action that currently succeeds SHALL be disabled,
and every request guard and server validation SHALL stay the source of truth.

#### Scenario: Preview waits for a file and a currency
Given no chosen file, when the import panel renders, then `Preview CSV` is disabled and its accessible reason says a CSV must be chosen; given a file and the currency `USD` it is enabled.

#### Scenario: Search waits for a query
Given an empty query, when the search panel renders, then `Search catalog` is disabled with a reason naming the query; typing one character enables it.

#### Scenario: Diagnosis waits for a part
Given a snapshot with no reviewed line and no hand-added part, then `Match this list` is disabled with an empty list and `Diagnose this build` is disabled with an empty build, and adding one part enables the diagnosis.

#### Scenario: A resolvable review is still submitted
Given a matched list whose review is unresolved, when the operator activates `Diagnose this build`, then it is enabled, the request is checked, and the existing review-complete refusal is announced.

### Requirement: Console counts and examples are real
Every count the inventory, catalog and diagnosis pages compose SHALL go through the
shared pluralisation helper, so a count of one reads as a singular noun and no
page-composed `(s)` pseudo-plural remains. The diagnosis examples — the pasted-list
placeholder, the parts-list help text and the manual-entry specification example —
SHALL use parts and specifications the shipped sample actually contains. Engine and
exported prose SHALL remain verbatim.

#### Scenario: Singular and plural forms follow the count
Given one record and two records, when the inventory history and the export panel render, then they read `1 record` and `2 records`.

#### Scenario: The examples match the shipped sample
Given the `/diagnose` page, when the pasted-list placeholder and the parts-list help text are read, then every example part is one of the shipped sample's parts and no example part is labelled `Fixture`.

#### Scenario: No page-composed pseudo-plural
Given the three pages at any count, when their rendered text is searched for `record(s)`, `part(s)`, `hit(s)` or `snapshot(s)`, then none is found in text the page composed.

### Requirement: A source scan rejects the `(s)` pseudo-plural
`tests/copy_style.rs` SHALL scan every Rust source under `src/` and every
TypeScript source under `apps/web/src/` and fail when a lowercase word is
immediately followed by `(s)`, naming the file and line. The scan SHALL cover a
source file added later without a per-file allowlist. The shared Rust helper
SHALL render `0 units`, `1 unit` and `2 units` for the counts it is given, and
zero SHALL take the plural noun.

#### Scenario: The scan names an offender
Given a source line containing `2 unit(s)`, when the guard test runs, then it fails and prints that file path and line number.

#### Scenario: The shipped sources pass
Given the shipped `src/` and `apps/web/src/` trees, when the guard test runs, then it passes and finds at least one source file in each tree.

#### Scenario: The helper distinguishes zero, one and many
Given the shared Rust helper, when it is called with 0, 1 and 2 units, then it returns `0 units`, `1 unit` and `2 units`.

### Requirement: The production build is measured on every client route
A Lighthouse performance-only report SHALL exist for `/`, `/inventory`, `/catalog`,
`/builds/check`, `/recommend` and `/diagnose`, measured against the production build
with the shipped sample snapshot and an active catalog dataset, on the mobile form
factor and with the desktop preset, and SHALL be committed under
`docs/evidence/perf-lighthouse/`. `docs/evidence/performance-evidence.md` SHALL
tabulate, per route and device, the performance score, first contentful paint,
largest contentful paint, total blocking time, cumulative layout shift, speed index
and transferred bytes; SHALL name the measured asset file names and their hashes;
and SHALL record the exact command and report path. The record SHALL state that
these are localhost lab measurements, not field data.

#### Scenario: Every route has both reports
Given the six client routes, when the evidence directory is read, then each route has one desktop report and one mobile report, each a Lighthouse JSON document for that URL with `only-categories` performance.

#### Scenario: The table matches the shipped build
Given the reports and the built `apps/web/dist/assets` files, when the evidence document is read, then the asset file names and hashes it names are the ones the reports measured.

#### Scenario: A finding is fixed or explained
Given a non-perfect performance audit, when the evidence document is read, then either the change that fixes it and its re-measured number are recorded, or the measured number and the reason it is not fixed are recorded.

### Requirement: The built client assets have an enforced byte ceiling
`scripts/check-bundle-size.mjs` SHALL read every built `apps/web/dist/assets/*.js`
and `*.css` file, compare each against an explicit per-type ceiling declared in the
script, and exit non-zero when a file exceeds its ceiling, naming the file, its size,
the ceiling and the delta. A missing or empty build directory SHALL fail the check
rather than pass it. `npm run build` and `npm run check` SHALL run the check, and the
declared verification script SHALL build the Web app before it runs the check, so the
check reads the fresh build. `scripts/check-bundle-size.test.mjs` SHALL prove with a
temporary directory that a within-budget fixture passes and an over-budget fixture
fails, without writing to the repository.

#### Scenario: A build within the ceiling passes
Given a build whose every JavaScript and CSS asset is at or below its type's ceiling, when the check runs, then it exits zero and reports each file with its size and ceiling.

#### Scenario: An over-budget file fails and is named
Given one asset larger than its ceiling by one byte, when the check runs, then it exits non-zero and names the file, its size, the ceiling and the delta.

#### Scenario: No build is a failure, not a pass
Given a checkout with no `apps/web/dist/assets` directory, when the check runs, then it exits non-zero and says the Web build must run first.

#### Scenario: The gate reads the fresh build
Given `./verify-local.sh`, when the Node stages run, then `npm run build` completes before `npm run check`, and a bundle over its ceiling fails that script.


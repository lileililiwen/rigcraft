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


# UI foundation deltas

## ADDED Requirements

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

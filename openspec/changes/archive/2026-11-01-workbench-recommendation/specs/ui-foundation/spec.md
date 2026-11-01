# ui-foundation deltas

## ADDED Requirements

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

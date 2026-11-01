# Design: Present compatibility and diagnosis as a verdict console

## Context

`CheckPage.tsx` renders the result as a verdict line, a three-cell summary grid,
an incomplete-build block, conflict and unknown diagnostic blocks, and a table
whose Evidence cell hides every compared value in a `<details>`. `DiagnosePage`
renders the result through `DiagnosisReportPanel`, which already groups advice
into four sections but styles each as a plain bordered list. Both consume the
measured dark token layer and the shared `app.css`; neither adds a colour.

The reference (`docs/design-program.md`, package 3) shows a rule matrix as the
place the decision is read: each rule has a state, the values it compared, the
source of those values and the next action. That is the structure the engine
already emits; this change stops hiding it.

## Decision 1 — the summary is one bar, and the matrix is a table

The check result opens with `div.console-summary` containing the existing
verdict line and a `dl.summary-grid.result-summary` with five cells: status
(the chip), rules passed, conflicts, unknown and parts not evaluated. The
existing `dd` order for compatible/conflict/unknown is kept because
`e2e/compatibility.spec.ts` reads it; parts-not-evaluated is appended.

Parts not evaluated is a presentation-side count: the selected records whose
`recordId` appears in no rule's `parts[]`. It adds no request field and no engine
value, and it is named rather than inferred.

`div.console-summary` is `position: sticky; top: 0` while the result is read, so
the overall state stays visible beside the matrix. It is static below 760 px
where a sticky block would consume most of a phone viewport.

The matrix stays a real `<table class="responsive-table rule-matrix">` with a
caption, because it is tabular data and the existing responsive card layout at
≤760 px already gives each row its labels. Columns are Rule, Verdict, Compared
values and Follow-up:

- Rule: title, `ruleId`, category and the method disclosure.
- Verdict: the existing `RuleBadge` — symbol + word + border.
- Compared values: the explanation plus one `dt`/`dd` per evidence entry with
  the SKU, field, value (or `Not provided`), source label, reference and
  observation time. Every value is visible; nothing is behind a disclosure.
- Follow-up: the rule's `followUp`, or a plain statement that the rule reached a
  verdict.

The conflict-diagnostics and facts-to-confirm blocks stay: they are the summary
of the same matrix and the e2e suite asserts their headings and text.

Rejected: replacing the table with a div grid. It would lose the row/column
relationships that screen readers use and duplicate the responsive card CSS the
shell already proves.

## Decision 2 — diagnosis keeps its skeleton and gains the same surfaces

`DiagnosisReportPanel` keeps the `diagnosis-result-title` heading, the verdict
line, the summary grid and every diagnostic block. The four `section.advice-group`
elements keep their exact headings and counts, so the existing `(2)` assertions
still match; a status chip becomes a sibling of the heading in a
`div.advice-group__head`, and the group gets a status-coloured left border. The
heading's accessible name is unchanged because the chip is outside it.

The described-build table and the upgrade plan keep every column, step, price,
rule id, rejected candidate and follow-up. The plan's `validated`,
`needs_evidence`, `no_option` and `no_valid_plan` labels are untouched. The
evaluated-substitution count now uses the plural helper.

No step is renumbered, reordered or renamed.

## Decision 3 — coverage is one sentence

The old paragraph always printed the checked fraction and then, when nothing was
unverified, repeated the fact in words. The replacement branches once:

- Some unverified: `{evaluated} of {total} described parts were checked by at
  least one rule. {n} parts had no rule that could run and remain unverified.`
- None unverified: `Every described part was checked by at least one rule.`

Plural forms come from the shared helper, so one part reads `1 of 1 described
part was checked …` and `1 part had no rule …`.

## Decision 4 — gap sentences at the source

`completeness()` currently stores a lowercase reason and concatenates it into
the follow-up. The table becomes `(category, present, reason, select_noun)`:

| Category | Reason | Follow-up |
| --- | --- | --- |
| cpu | `A CPU is required.` | `Select a CPU from this snapshot.` |
| motherboard | `A motherboard is required.` | `Select a motherboard from this snapshot.` |
| ram | `Memory is required.` | `Select memory from this snapshot.` |
| psu | `A power supply is required.` | `Select a power supply from this snapshot.` |
| case | `A case is required.` | `Select a case from this snapshot.` |
| storage | `Startup storage is required.` | `Select startup storage from this snapshot.` |
| gpu | `No graphics card is selected and sourced integrated-graphics evidence is absent.` | `Select a graphics card, or record \`igpu=yes\` on the CPU with its source.` |
| cooler | `No cooler is selected and sourced bundled-cooling evidence is absent.` | `Select a cooler, or record \`bundled_cooler=yes\` on the CPU with its source.` |

A unit test asserts the exact motherboard pair, that every reason ends in a
period and that no follow-up contains `is required`. Because the reason now ends
in a period, the two renderers print `reason` and `followUp` with a space rather
than appending a second period.

This is the only service text change. The check API shape, the missing-entry
schema (`category`, `reason`, `followUp`) and every rule result are unchanged.

## Decision 5 — exact matches stay quiet

In the match review, `candidate.extraTokens` and `candidate.missingTokens` are
rendered only when `candidate.exact` is false. An exact token set adds nothing by
definition, so the empty list is noise. A partial or ambiguous candidate keeps
both lists, which is the evidence that disambiguates it. `exact` matching,
candidate ordering, the `exact tokens` / `partial tokens` label and the request
payload are unchanged.

## Decision 6 — one plural helper, two pages

`apps/web/src/plural.ts` exports `pluralWord(count, singular, pluralForm?)` and
`plural(count, singular, pluralForm?)`. `recommend.ts` imports and re-exports
`plural`, so the workbench imports and tested helper behaviour are unchanged.

The check and diagnosis pages use it for units, lines, parts, rules and
conflicts they compose. Engine sentences are still rendered verbatim: the plan
summary (`1 step(s) add …`), the check's `unit(s)` error from the API and the
recommendation prose are exported text, and this package is explicitly not the
place to rewrite them.

## Decision 7 — no new visual vocabulary

The change adds classes, not tokens: `.console-summary`, `.rule-matrix`,
`.advice-group__head`, `.advice-group--{action}`. Every property resolves an
existing semantic alias (`--color-surface`, `--color-border`, `--color-danger`,
`--color-warning`, `--color-success`, `--color-muted`, `--focus-ring`). No colour
literal, no inline style, no `!important` outside the reduced-motion block, no
dependency, no webfont and no animation are added; the only transitions are the
existing reduced-motion-safe ones.

## Shared infrastructure assessed

| Candidate | Evidence | Decision |
| --- | --- | --- |
| Shared token kit | `vendor/tokens.css`, pin `7ca6b795…` | consume unchanged; no edit, no re-pin |
| Brand layer | `brand.css`, 40 measured pairs | consume semantic aliases only |
| Compatibility engine | `src/compatibility/`, `compat-rules-v1` | only the gap sentence changes |
| Diagnosis advice | `src/diagnosis/`, `diagnosis-advice-v1` | unchanged |
| Playwright suite | `e2e/compatibility.spec.ts`, `e2e/diagnosis.spec.ts` | extend, no assertion removed |
| Plural helper | `apps/web/src/recommend.ts` | moved to `plural.ts` and re-exported, not duplicated |

## Alternatives considered

- **Colour the matrix cells by verdict only.** Rejected: status must be symbol +
  word + border; colour alone is not a verdict.
- **Put evidence behind a disclosure to keep rows short.** Rejected: the package
  exists to stop hiding the compared values.
- **Rename the diagnosis group headings to console labels.** Rejected: the
  headings and counts are asserted contract text and the skeleton must not move.
- **Fix `unit(s)` in the API error too.** Rejected: that is service prose beyond
  the authorised gap-copy change and is carried by exports; it is recorded as
  residual.
- **A shared component library for the console.** Rejected: two pages, one table
  and four group cards do not justify a new abstraction or a dependency.

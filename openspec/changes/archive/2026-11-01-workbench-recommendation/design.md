# Design: Rebuild the recommendation page as a workbench

## Context

`/recommend` is one 899-line component plus `CandidateCard`, `AssistantPanel`,
`recommend.ts` and `inventory.ts`, styled by the shared `app.css`. The archived
`ui-foundation` change gave the app a measured dark token layer; this change
consumes it and adds no colour.

The page currently emits one long column: snapshot picker, assistant panel,
budget form, then a result panel holding the summary, the use-case rule, the
candidates and the comparison. The engine emits every fact the page shows; the
page must not compute or drop one.

## Decision 1 — a named-area grid, source order everywhere

The three regions are siblings inside `<main>` and are never reordered by CSS:

```
.workbench {
  display: grid;
  grid-template-columns: minmax(0, 21rem) minmax(0, 1fr) minmax(0, 19rem);
  grid-template-areas: "constraints options verdict";
}
@media (max-width: 1099px) { one column, same three areas in order }
```

Source order is constraints → options → verdict at every width, so keyboard
order, screen-reader order and the single-column order never disagree (WCAG
2.4.3). The page and the header inner both widen to `88rem` on this route only,
through a `wide` prop on `SiteHeader`; other routes keep `76rem`.

Rejected: placing the verdict first in the DOM and painting it last. It looks
fine and breaks the relationship between focus order and visual order, which is
exactly the kind of defect the last two packages fixed.

## Decision 2 — the spec module keeps every fact

`CandidateCard` becomes a module: a decorative caps label (`BUILD SPEC · OPTION
1`, `aria-hidden`, repeated by the `<h3>` "1. Best overall"), the heading, the
status chip (`✓ / ✕ / ?` + word + border, never colour alone), and a definition
list of Total, Budget delta, Stock units and Score rendered in the mono stack
with `font-variant-numeric: tabular-nums`. The parts table, score-dimension
table, reasons, alternatives, unknown follow-ups, rule evidence and check/version
strings are unchanged. Nothing the engine emits is summarised away; the module is
a presentation layer over the same `Candidate`.

The `dl.summary-grid` class is kept on the candidate metrics because
`e2e/recommendation.spec.ts` reads the first `dd` of it as the option total; the
`result-summary` class moves to the verdict rail only, so "the first report
summary in the DOM" is unambiguous.

## Decision 3 — the verdict rail states the decision once

The rail carries the report summary (`Distinct builds`, `Budget`, `Valid
candidates`, `Status`), the top option's status chip, budget delta, stock units
and score, and the rule-matrix summary from `candidate.checkSummary` as
"12 compatible · 0 conflicts · 2 unknown". The use-case rule block keeps its
exact heading and `data-use-case-rule` attribute; the no-padding statement keeps
its exact sentence; a labelled counter line (`2 valid candidates → 1 distinct
build`) explains the difference between the two counts.

The rail holds no engine-computed value of its own: every number is read from
`RecommendationReport` or the top `Candidate`.

The no-solution explanation, candidate modules, comparison, exclusions and
exports stay in the options column, where there is room for the blockers and the
adjustable list; the rail then shows the status and says there is no option to
score.

## Decision 4 — budget grammar

`parseBudgetMinor` accepts:

| Input | Minor units |
| --- | --- |
| `1,800` | 180000 |
| `1 800` | 180000 |
| `1,800.50` | 180050 |
| `1 234 567.89` | 123456789 |

It refuses `1.800,50` (decimal-comma layout), `1,80` and `1,8000` (a group that is
not three digits), `1.234` (dotted grouping is not accepted), and any other shape,
returning `null`. A separator is only accepted when the layout is unambiguous: a
comma or a single space between groups of exactly three digits after the first
group. The parser stays pure and integer-safe (`Number.isSafeInteger`).

`Generate recommendations` is disabled while the parsed value is `null`, and the
button carries `aria-describedby` naming the persistent reason. When the field is
non-empty and unparsable, the inline `budget_invalid` guidance appears immediately
instead of waiting for a submit, so the guidance is still reachable even though
the button cannot submit. `generate()` keeps its defensive `budget_invalid`
branch.

## Decision 5 — one money form, engine prose untouched

`formatMoney` pins `Intl.NumberFormat("en-001", …)`: the neutral English locale
renders `US$1,672.90` (and `€1,234.56`), never consults the operator's browser
locale, and matches the amount already quoted in the evidence documents. The
`try/catch` fallback stays for a runtime without that locale.

The engine's own prose (`Total 1,672.90 USD leaves 127.10 under the budget …`)
is rendered verbatim, because e2e asserts `1,672.90 USD` in the no-solution text
and the exported JSON/Markdown carries the same sentence. So the page shows
`US$1,672.90` in its own readouts and the engine's sentence keeps `1,672.90 USD`;
the same amount, two documented spellings. Changing the engine's `format_money`
would break the asserted contract, so it is not done here.

The same reasoning applies to the engine's `unit(s)`/`conflict(s)` words: they
are the engine's sentences and this change keeps them verbatim. The shared
pluralisation helper governs every string the page composes.

## Decision 6 — collapsed assistant, credential field in a form

`AssistantPanel` returns `<details className="panel assistant">` with the panel
title as the `<summary>` heading, closed on first render. Native `<details>`
gives the expansion state to assistive technology without script; there is no
animation. The summary is styled as the panel title and keeps a visible focus
ring.

The key, base URL and model fields are wrapped in
`<form onSubmit={(event) => event.preventDefault()}>`. That gives the password
input a form owner — which is the condition Chrome's "password field is not
contained in a form" warning tests — without submitting anything: the interpret
action stays a `type="button"` click and the key never leaves component state.
It is a sibling of the budget `<form>`, never nested.

`Continue without AI` stays outside the interpretation flow and still calls the
page's `skipAssistant`, which moves focus to `#recommend-budget`.

## Decision 7 — shared files stay under the size gate

`RecommendPage.tsx` is 899 lines and the source-file-size gate blocks at 1000.
The results block and the verdict rail move into `RecommendOptions.tsx` and
`RecommendVerdict.tsx`; `RecommendPage` keeps state, effects and the request
form. `recommend.ts` gains `plural` and the parser; `inventory.ts` only pins the
locale.

## Shared infrastructure assessed

| Candidate | Evidence | Decision |
| --- | --- | --- |
| Shared token kit | `vendor/tokens.css`, pin `7ca6b795…` | consume unchanged; no token added |
| Brand layer | `brand.css`, 40 measured pairs | consume semantic aliases only; no colour literal |
| Recommendation engine | `src/recommend/`, `recommend-score-v1` | unchanged |
| BYOK contract | `docs/ai-planning.md`, `tests/ai_secret_hygiene.rs` | unchanged; presentation only |
| Playwright suite | `e2e/recommendation.spec.ts`, `e2e/fixtures.ts` | extend, no assertion removed |
| Vendor of a layout system | none in the app; CSS grid is in the platform | no dependency added |

## Alternatives considered

- **A CSS-only restyle of the existing single column.** Rejected: the requirement
  is that constraints, options and verdict are visible together, which needs a
  grid and a re-parented result block.
- **Fixed pixel columns copied from the reference.** Rejected: no fixed canvas,
  320 px must still work, and the reference's 2560 px four-column frame cannot
  collapse.
- **Accept `1.800,50` by guessing the locale.** Rejected: it is ambiguous without
  the operator's locale, and a wrong guess silently changes the budget by three
  orders of magnitude.
- **Rewriting engine sentences in the client.** Rejected: the export prints the
  engine sentence and the page would then disagree with the file the operator
  downloads.
- **A React details component.** Rejected: the platform element already exposes
  the state and keyboard behaviour with no dependency.

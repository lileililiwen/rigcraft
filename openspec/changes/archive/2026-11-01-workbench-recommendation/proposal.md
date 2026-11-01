# Proposal: Rebuild the recommendation page as a workbench

## Why

`/recommend` answers the one question the product exists for, and today it reads
like a long form followed by a stack of cards. The constraints, the options and
the verdict are never visible at the same time: at 1440 px the operator scrolls
from the snapshot picker through the assistant panel and the request form before
the first option appears, and the score and rule summary sit inside each card
with no single place that states the decision. The reference workbench
(`docs/design-program.md`, package 2) shows the idea worth taking: a narrow
constraints column, a column of specification modules with monospaced numerals,
and a verdict rail carrying the score and the rule matrix.

Two smaller defects live in the same page and make it lie about itself. The
budget parser rejects `1,800`, a form in which a price is normally written, and
`Generate recommendations` is enabled with an empty field, so the operator only
learns the field is invalid after a submit. The optional assistant panel is
expanded by default and pushes the budget field below the fold, and its provider
key field sits outside any form, which makes Chrome log a warning on every page
load. Page-owned counts still use `(s)` pseudo-plurals and money formatting
follows the browser locale.

This change rebuilds the page's presentation around the workbench layout and
fixes those input and copy defects. It changes no number the engine produces, no
request field, no route and no API.

## What Changes

- **Three-region workbench.** At 1100 px and wider, `/recommend` renders one
  named-area grid — constraints, options, verdict — capped at 88 rem; below
  1100 px the same regions stack in source order. The wide header aligns with the
  wide page.
- **Options as spec modules.** Each candidate becomes a specification module: a
  decorative caps label (repeated in the accessible heading), the option name, a
  status chip of symbol + word + border, and monospaced tabular price, stock and
  score rows. Every table, reason sentence, follow-up, alternative and version
  string the engine emits is kept.
- **A verdict rail.** Status, budget delta, stock units, score and the
  compatible/conflict/unknown rule-matrix summary are visible beside the options,
  together with the use-case rule, the labelled counters
  (`2 valid candidates → 1 distinct build`) and the existing no-padding sentence.
- **A collapsed assistant.** The optional BYOK panel is a native `<details>`,
  closed by default; its no-key state, consent copy and `Continue without AI`
  action remain, and that action still moves focus to the budget field. The
  transport fields sit inside a non-submitting `<form>` so the browser stops
  warning that a password field is outside a form. The key stays request-scoped.
- **An honest budget field.** `1,800`, `1 800` and `1,800.50` parse to minor
  units; ambiguous forms (`1.800,50`, `1,80`, `1,8000`, `1.234`) are refused with
  the existing `budget_invalid` guidance. Generate is disabled while the field is
  empty or unparsable, with a persistent description that says so.
- **One agreed form for counts and money.** A shared pluralisation helper
  replaces page-owned `(s)`; money renders as `US$1,672.90` in a fixed locale;
  the result counters are labelled so the no-padding rule explains itself.
- **Tests and evidence.** Vitest covers grouping, refusals, the disabled
  Generate, the collapsed panel, the plural helper and the workbench regions;
  Playwright adds the 1440 px three-column and 390 px single-column measurements
  with `scrollWidth === clientWidth` while every existing assertion stays.

## Impact

- Affected specs: `ui-foundation` (presentation requirements added),
  `build-recommendation` (budget parsing, disabled Generate and copy added).
- Affected code: `apps/web/src/components/RecommendPage.tsx`,
  `CandidateCard.tsx`, `AssistantPanel.tsx`, new
  `RecommendVerdict.tsx`/`RecommendOptions.tsx`, `apps/web/src/recommend.ts`,
  `apps/web/src/inventory.ts` (fixed money locale), `apps/web/src/styles/app.css`,
  `apps/web/src/components/SiteHeader.tsx` (wide variant), the Vitest page tests,
  `e2e/recommendation.spec.ts`, `docs/`.
- Unchanged: every API, the recommendation engine, the request contract, the
  persisted schema, the scoring version, the rule set, the diagnosis skeleton,
  the vendored and brand token files, and every copy string the e2e suite
  asserts.

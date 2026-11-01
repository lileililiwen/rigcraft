# Proposal: Present compatibility and diagnosis as a verdict console

## Why

`/builds/check` and `/diagnose` answer the two questions the compatibility engine
exists for, but they still read like a long form followed by prose. The check
result is a summary grid, two diagnostic blocks and a four-column evidence table
whose values are hidden inside a disclosure per row; the rule a person came to
read — its verdict, the two facts it compared, where those facts came from and
what to do next — is spread across three of those places. The diagnosis result
groups advice correctly but presents each group as a heading and a list, so the
console language the workbench already uses stops at the page boundary.

Two copy defects make both pages lie about themselves. `completeness()` in
`src/compatibility/mod.rs` builds a follow-up as `Select {reason} from this
snapshot.`, which renders "Motherboard: a motherboard is required. Select a
motherboard is required from this snapshot." The diagnosis report also prints
"X of X … were checked by at least one rule." and then "Every part was checked by
at least one rule." in the same paragraph. Both are visible in the shipped
sample and both are fixed at their source or in the sentence that renders them.

A third defect is noise: an `exact` match already proves the token sets are
equal, yet the match review prints empty added/missing token lists for it. Page
composed counts still use `line(s)`, `part(s)`, `unit(s)` and `rule(s)`.

This change presents rules as a console on both pages and fixes those copy
defects. It changes no rule, no score, no request, no route, no schema and no
matching behaviour; the gap sentences are the only service text change.

## What Changes

- **Check page as a rule console.** A summary bar carries the overall status,
  rules passed, conflicts, unknown and parts not evaluated, and stays above the
  matrix while the result is read. The matrix gives every rule one row with its
  verdict chip (symbol + word + border), the compared values, the provenance
  reference for each value and its follow-up. Every rule, value, explanation and
  live announcement is kept, and the part selector keeps its source order.
- **Diagnosis in the same language.** The existing 1–5 skeleton keeps its
  headings, ids and statuses. The four advice groups become console cards with a
  status chip and a group border, while the evidence table and upgrade plan keep
  every part, price, rule, follow-up and validation state.
- **Grammatical gap sentences.** `completeness()` returns a complete reason
  sentence and an actionable, non-duplicating follow-up for CPU, motherboard,
  memory, power supply, case, startup storage, graphics card and cooler. A test
  fails if any follow-up contains `is required` again.
- **One coverage sentence.** When every described part was evaluated the report
  states that once; when some were not, it reports the unverified count instead
  of printing both sentences.
- **Quiet exact matches.** Added and missing token lists are hidden for an
  `exact` candidate and kept for a partial or ambiguous one. Matching is
  unchanged.
- **Real plural forms.** A small shared helper replaces `line(s)`, `part(s)`,
  `unit(s)` and `rule(s)` on these two pages. Recommendation prose and exported
  text pinned by the workbench package are untouched.
- **Tests and evidence.** Rust covers every gap sentence and the
  no-`is required` guard; Vitest covers the summary/matrix, diagnosis grouping,
  the single coverage sentence, hidden exact token lists and plural forms;
  Playwright keeps every existing journey and adds the grammatical gap text and
  the absence of the duplicated coverage sentence. Lighthouse, captures and a
  browser evidence document are added.

## Impact

- Affected specs: `ui-foundation` (three presentation requirements added),
  `compatibility-evidence` (gap-sentence requirement added), `build-diagnosis`
  (coverage sentence and exact-token display added to the Web-flow requirement).
- Affected code: `src/compatibility/mod.rs` (gap copy and unit tests),
  `apps/web/src/compatibility.ts`, `apps/web/src/diagnosis.ts`, new
  `apps/web/src/plural.ts`, `CheckPage.tsx`, `DiagnosePage.tsx`,
  `DiagnosisReportPanel.tsx`, `apps/web/src/styles/app.css`, the Vitest page
  tests, `e2e/compatibility.spec.ts`, `e2e/diagnosis.spec.ts`, `docs/`.
- Unchanged: the rule set `compat-rules-v1`, matching, scoring, every API shape,
  every route, the SQLite schema, the `/diagnose` 1–5 skeleton, the stale-report
  notices, focus management, live regions, keyboard order, Markdown/JSON exports,
  the vendored and brand token files, and every existing e2e assertion.

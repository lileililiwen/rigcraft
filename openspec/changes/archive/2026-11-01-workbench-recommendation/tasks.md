# Tasks: Rebuild the recommendation page as a workbench

## 1. BFS — Baseline, impact and reproduction
- [x] 1.1 Read `AGENTS.md`, `.ai-rules/`, `HANDOFF.md`, `docs/design-program.md` (package 2), `docs/branding.md`, `RecommendPage.tsx`, `CandidateCard.tsx`, `AssistantPanel.tsx`, `recommend.ts`, `inventory.ts`, `app.css`, `types.ts`, the Vitest page tests and the Playwright recommendation/keyboard journeys; confirm no other active change and set `current_spec`.
- [x] 1.2 Reproduce against the shipped seed at 1800 (gaming): one materially different GPU-inclusive build at 1,672.90, 2 valid builds, the rule heading, `1 materially different build found`, and the `(s)` strings the engine emits.
- [x] 1.3 Map impact: the page's state and effects, the result summary and its `dd` order consumed by e2e, the candidate `dl.summary-grid`, the assistant panel's fields, the budget parser's callers (page and assistant), `formatMoney`'s callers, and the strings e2e asserts.
- [x] 1.4 Assess shared infrastructure by name (pinned token kit `7ca6b795…`, brand layer, recommendation engine, BYOK contract, Playwright suite) and record that all are consumed unchanged.

## 2. DFS — Workbench presentation
- [x] 2.1 Add the wide page and header variant and the named-area `constraints | options | verdict` grid, capped at 88 rem, with a single-column layout below 1100 px and no reordering of source order.
- [x] 2.2 Rebuild `CandidateCard` as a spec module: decorative caps label, option heading, status chip, monospaced tabular price/stock/score rows; keep every table, reason, follow-up, alternative and version string.
- [x] 2.3 Extract `RecommendOptions` (heading, stale notes, no-solution detail, candidates, comparison, exports, exclusions) and `RecommendVerdict` (summary, top-option metrics, rule matrix, use-case rule, labelled counters) so both files stay under the size gate.
- [x] 2.4 Keep the stale-results notice, exports, locked/excluded selectors, the `resultsRef` focus target, live announcements and the skip link unchanged.

## 3. DFS — Assistant panel
- [x] 3.1 Turn the panel into a native `<details>` closed on first render, with the panel title as its summary and no animation.
- [x] 3.2 Wrap the key/base URL/model inputs in a non-submitting `<form>` so the password field has a form owner.
- [x] 3.3 Keep the no-key state, consent copy, retry and `Continue without AI`, and prove the latter still moves focus to the budget field.

## 4. DFS — Budget, Generate and copy
- [x] 4.1 Extend `parseBudgetMinor` to accept comma and single-space grouping only in an unambiguous layout, keeping integer safety and refusing ambiguous forms.
- [x] 4.2 Disable `Generate recommendations` while the budget is empty or unparsable; keep the invalid guidance reachable inline and associated with the field and the button.
- [x] 4.3 Add the shared `plural` helper, replace page-composed `(s)` and singular/plural ternaries with it, label the result counters, and pin the money locale so money renders `US$1,672.90`.

## 5. BFS — Tests and evidence
- [x] 5.1 Extend `RecommendPage.test.tsx`: comma and space budget → 180000/180050, ambiguous refusals, disabled Generate, collapsed assistant with accessible expansion and `Continue without AI` focus, plural helper, workbench regions and spec-module labels.
- [x] 5.2 Extend `AssistantPanel.test.tsx` for the collapsed default and the form owner of the key field; keep every existing assistant assertion.
- [x] 5.3 Extend Playwright: three-region layout at 1440, single column at 390, `scrollWidth === clientWidth` at 320/390/768/1440, and no regression in the existing recommendation and keyboard journeys.
- [x] 5.4 Run the real flow on a free port with a scratch data directory, import the shipped sample, generate gaming at 1800, capture 1440×900 and 390×844 into `docs/assets/workbench-*.png`, and run Lighthouse desktop and mobile on `/recommend`.
- [x] 5.5 Write `docs/evidence/workbench-recommendation-browser.md` with the real scores, three-region measurements and overflow results; update `docs/design-program.md`, `docs/branding.md` if the type/role table moved, `docs/assets/capture-plan.md` and README claims.

## 6. Verification and close-out
- [x] 6.1 `cargo fmt --all -- --check`; `cargo clippy --all-targets --all-features -- -D warnings`; `cargo test`; `npm run check`; `npm test`; `npm run build`; `npm run test:e2e`; `./verify-local.sh`.
- [x] 6.2 `node scripts/check-openspec-change-names.mjs`; `openspec validate workbench-recommendation --strict`; `openspec validate --all --strict`.
- [x] 6.3 Run the Forge full Gate and record its aggregate verdict and labelled revision.
- [x] 6.4 Archive with `openspec archive workbench-recommendation --yes` (no `--skip-specs`), commit everything except `HANDOFF.md`, then commit `HANDOFF.md` only; do not push.

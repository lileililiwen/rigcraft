# Tasks: Present compatibility and diagnosis as a verdict console

## 1. BFS — Baseline, impact and reproduction
- [x] 1.1 Read `AGENTS.md`, `.ai-rules/`, `HANDOFF.md`, `docs/design-program.md` (package 3), `docs/branding.md`, `docs/build-diagnosis.md`, `docs/compatibility-evidence.md`, `src/compatibility/mod.rs`, `src/diagnosis/*`, `CheckPage.tsx`, `DiagnosePage.tsx`, `DiagnosisReportPanel.tsx`, `compatibility.ts`, `diagnosis.ts`, `app.css`, the Vitest page tests and the Playwright compatibility/diagnosis journeys; confirm no other active change and set `current_spec`.
- [x] 1.2 Reproduce against the shipped seed: a CPU-only check prints `a motherboard is required. Select a motherboard is required from this snapshot.`; a diagnosis on an all-evaluated build prints both the fraction and the all-checked sentence; an exact match prints empty token lists; the pages render `line(s)`, `part(s)` and `unit(s)`.
- [x] 1.3 Map impact: the summary `dd` order e2e reads, the rule table and its disclosure, the diagnostic blocks e2e asserts, the four group headings and counts, the described-build table, the plan block, the match-review candidate tokens, both live regions, focus targets and every string the e2e suite asserts.
- [x] 1.4 Assess shared infrastructure by name (pinned token kit `7ca6b795…`, brand layer, compatibility engine, diagnosis advice, Playwright suite, existing plural helper) and record that all are consumed unchanged except the moved helper.

## 2. DFS — Gap sentences and shared plural helper
- [x] 2.1 Rewrite `completeness()` reasons and follow-ups so each reason is a complete sentence and each follow-up is an actionable instruction with no `is required`; cover CPU, motherboard, memory, PSU, case, startup storage, GPU and cooler.
- [x] 2.2 Add Rust unit tests for the exact motherboard pair, the sentence punctuation and the no-`is required` follow-up guard.
- [x] 2.3 Print `reason` and `followUp` once in `CheckPage` and `DiagnosisReportPanel` without appending a second period.
- [x] 2.4 Extract `plural`/`pluralWord` into `apps/web/src/plural.ts`, re-export `plural` from `recommend.ts`, and replace page-composed `line(s)`, `part(s)`, `unit(s)`, `rule(s)` and `conflict(s)` in the check and diagnosis UI and the sentences they render.

## 3. DFS — Check page console
- [x] 3.1 Add the summary bar with status, rules passed, conflicts, unknown and parts not evaluated, sticky above the matrix at desktop width and static at phone width.
- [x] 3.2 Replace the evidence-in-a-disclosure table with a rule matrix: one row per rule with verdict chip, compared values, provenance reference per value and the follow-up; keep every rule, value, explanation, method, rule id and version.
- [x] 3.3 Keep the incomplete-build, conflict and facts-to-confirm blocks, the stale notice, live region, focus target and part selector source order.

## 4. DFS — Diagnosis console and match review
- [x] 4.1 Present the four advice groups as console cards with a status chip beside the unchanged heading/count, keeping every advice card, fact and cost.
- [x] 4.2 State the coverage fact once: the fraction plus unverified count when some parts were not evaluated, otherwise the all-checked sentence alone.
- [x] 4.3 Hide added/missing token lists for `exact` candidates and keep them for partial/ambiguous candidates without changing matching.
- [x] 4.4 Keep the `/diagnose` 1–5 headings, ids, statuses, evidence table, upgrade plan, stale notice, live region and focus management unchanged.

## 5. BFS — Tests and evidence
- [x] 5.1 Extend Vitest: check summary/matrix rendering and parts-not-evaluated, diagnosis grouping and the single coverage sentence, exact-match token lists hidden, and singular/plural forms.
- [x] 5.2 Extend Playwright: a CPU-only check asserting `A motherboard is required.` and `Select a motherboard from this snapshot.`, and a diagnosis on an all-evaluated build asserting the all-checked sentence exactly once; keep every existing journey green.
- [x] 5.3 Run the app on a free port with a scratch data directory, import the shipped sample, produce a real CPU-only check and a real diagnosis, capture 1440×900 and 390×844 into `docs/assets/verdict-*.png`.
- [x] 5.4 Run Chrome DevTools MCP `lighthouse_audit` desktop and mobile on `/builds/check` and `/diagnose`, measure focus rings, caption widths and `scrollWidth === clientWidth`, and write `docs/evidence/verdict-console-browser.md` with the real numbers.
- [x] 5.5 Update `docs/design-program.md`, `docs/build-diagnosis.md`, `docs/compatibility-evidence.md`, `docs/assets/capture-plan.md` and README claims touched by the new presentation.

## 6. Verification and close-out
- [x] 6.1 `cargo fmt --all -- --check`; `cargo clippy --all-targets --all-features -- -D warnings`; `cargo test`; `npm run check`; `npm test`; `npm run build`; `npm run test:e2e`; `./verify-local.sh`.
- [x] 6.2 `node scripts/check-openspec-change-names.mjs`; `openspec validate verdict-console --strict`; `openspec validate --all --strict`.
- [x] 6.3 Run the Forge full Gate and record its aggregate verdict and labelled revision.
- [x] 6.4 Archive with `openspec archive verdict-console --yes` (no `--skip-specs`), commit everything except `HANDOFF.md`, then commit `HANDOFF.md` only; do not push.

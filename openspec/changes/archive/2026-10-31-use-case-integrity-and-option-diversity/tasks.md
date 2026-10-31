# Tasks: Use-case integrity and option diversity

## 1. BFS — Baseline, impact and reproduction
- [x] 1.1 Read `AGENTS.md`, `.ai-rules/workflow.md`, `HANDOFF.md`, `docs/build-recommendation.md`, `src/recommend/*`, the UI recommendation files and the existing Rust/Vitest/Playwright tests; confirm `use-case-integrity-and-option-diversity` is the only active change and set `current_spec`.
- [x] 1.2 Reproduce the defect against the shipped seed: gaming at 1,800.00 USD returns three GPU-less storage variants; record the parts and totals.
- [x] 1.3 Map the impact: `build_input` required pools, `minimum_complete_cost`, the no-solution branches, `Walker` retention, candidate assembly, the report shape, `RecommendPage`, `CandidateCard`, `types.ts`, `recommend.ts` and the export.
- [x] 1.4 Assess shared infrastructure by name (recommendation module, compatibility completeness, AI contract, Playwright, vendor tokens) and record the decisions.

## 2. DFS — Use-case completeness and no-solution
- [x] 2.1 Mark `gpu` required for `gaming` in `build_input`; leave `office` and `creation` optional.
- [x] 2.2 Restructure `minimum_complete_cost` so a required GPU is priced once and a gaming budget below it returns `budget` with the GPU named, the GPU-inclusive minimum and the shortfall.
- [x] 2.3 Make the stock no-solution state the gaming GPU requirement when no card is usable.
- [x] 2.4 Add the report-level `useCaseRule` and per-candidate reasons for gaming, office and creation.

## 3. DFS — Material diversity
- [x] 3.1 Add a material signature over chosen parts excluding `storage`.
- [x] 3.2 Retain one cheapest representative per material signature in the bounded top pool, keeping cheapest and highest-performance tracking.
- [x] 3.3 Assemble at most three candidates with distinct material signatures and distinct objectives; return one when only one exists; add `materialSignature` to the candidate.
- [x] 3.4 Prove determinism, three distinct builds, storage-only collapse, one-build no-padding, gaming exclusion of iGPU-only, office acceptance of `igpu=yes`, and the named GPU no-solution with Rust tests and fixtures.

## 4. DFS — Web flow
- [x] 4.1 Extend the TypeScript types and render the use-case rule, the GPU requirement and an accurate materially-different option count.
- [x] 4.2 Update the Vitest page fixtures and add assertions for the rule sentence and the no-padding statement.
- [x] 4.3 Update `e2e/recommendation.spec.ts` and `e2e/keyboard-responsive.spec.ts`; add a gaming–GPU journey and a no-padding journey; keep the 390 px assertion.

## 5. BFS — Contract, docs and evidence
- [x] 5.1 Update `docs/build-recommendation.md`: completeness per use case, the material-signature definition, the non-material storage exclusion, the no-solution wording and the option-count statement.
- [x] 5.2 Add the one-line note to `docs/ai-planning.md` that a confirmed proposal is still subject to the gaming GPU rule.
- [x] 5.3 Write `docs/evidence/use-case-diversity-browser.md` and capture the gaming-with-GPU, GPU-bound no-solution and no-padding states at 1440×900 and 390×844; re-run Lighthouse on `/recommend`.
- [x] 5.4 Confirm no change to the request contract, schema, scoring weights, work limit, BYOK panel, provider adapters or tokens.

## 6. Verification and close-out
- [x] 6.1 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test`, `npm run check`, `npm test`, `npm run build`, `npm run test:e2e`, `./verify-local.sh`.
- [x] 6.2 Run `node scripts/check-openspec-change-names.mjs`, `openspec validate use-case-integrity-and-option-diversity --strict` and `openspec validate --all --strict`.
- [x] 6.3 Run the Forge full Gate and record its aggregate verdict and revision.
- [x] 6.4 Archive with `openspec archive use-case-integrity-and-option-diversity --yes` (no `--skip-specs`), then commit everything except `HANDOFF.md`, then commit `HANDOFF.md` only; do not push.

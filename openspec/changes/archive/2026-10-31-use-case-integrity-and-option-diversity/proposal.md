# Proposal: Make use cases and option diversity honest

## Why

The sourced catalog seed exposed two defects that the earlier synthetic fixtures hid.

A gaming request returns three builds with no graphics card. The Ryzen 7 7800X3D
publishes `igpu=yes`, and the current completeness rule treats that sourced fact as a
substitute for a discrete card in every use case. At a 1,800.00 USD budget the sample
answers a gaming request with three iGPU-only machines at 1,123.90, 1,183.90 and
1,333.90 that differ only in storage capacity. That is not a gaming build, and the
presence of one storage SKU does not make the second and third options different
choices.

REQ-R02 asks for up to three *materially different* options — value, performance and
upgrade room — and says not to pad when only one or two real options exist. The current
search deduplicates on the full SKU set, so three storage capacities of one machine
count as three options.

## What Changes

- **Use-case completeness.** `gaming` requires a discrete graphics card in every
  candidate. A CPU's sourced `igpu=yes` no longer completes a gaming build. `office`
  keeps accepting a sourced `igpu=yes`, and `creation` keeps today's behaviour while
  its explanation says a discrete card was not required. No request field is added and
  the AI parse contract is untouched.
- **GPU-bound no-solution.** When no GPU-inclusive build fits, the result is
  `no_solution` with the existing `budget`, `stock`, `compatibility` or `search_limit`
  binding, and the graphics card is named in the headline, blockers and explanation
  with the cheapest GPU-inclusive total and the shortfall.
- **Material diversity.** A candidate gets a structural signature over its chosen
  parts (category, SKU and quantity) excluding storage. Candidates with the same
  signature collapse to one representative, so a larger drive is not a different
  option. At most three representatives are shown, each with a distinct objective; if
  only one or two materially different builds exist, exactly that many are returned
  with a statement that nothing was padded. Ordering stays deterministic.
- **Explainability.** The report carries the use-case rule that applied
  (`requiresDiscreteGpu`, whether `igpu=yes` is accepted, and a sentence of text), and
  each candidate states how the rule affected completeness. The `/recommend` page
  renders the rule when a GPU was required and states the real option count.
- **Tests, docs and browser evidence.** Rust and Vitest tests pin the use-case rule,
  storage-only collapse, no padding, three distinct builds and determinism. The
  Playwright suite gains gaming–GPU and no-padding cases. `docs/build-recommendation.md`
  defines completeness per use case and what "material" excludes.

## Package Boundary and Split Assessment

Single outcome: a recommendation's parts and its option count tell the truth about the
requested use case. The use-case rule, the material signature and the report field all
travel through the same search result; splitting them would ship a report whose rule
text disagrees with the candidates it labels. The change touches one domain module,
one report shape, one page, its tests and its contract document, so it is one package.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence | Compatibility and ownership | Decision |
| --- | --- | --- | --- |
| Existing recommendation search (`src/recommend/`) | `search.rs`, `explain.rs`, `candidate.rs`, `score.rs` | Owns completeness, scoring, no-solution and determinism; the defect is in this module. | change in place |
| Compatibility completeness (`src/compatibility/`) | `completeness.status` / `missing` | Already models "GPU unless `igpu=yes`"; the use-case rule is a recommendation policy, not a hardware rule. | unchanged; the search enforces the stricter use-case pool |
| AI planning contract | `docs/ai-planning.md`, `tests/ai_planning_api.rs` | Proposes only the seven existing request fields and is explicitly out of scope. | unchanged; one-line note only |
| Playwright suite | `e2e/recommendation.spec.ts`, `e2e/keyboard-responsive.spec.ts` | Already drives the shipped seed against the real service. | update assertions, add two journeys |
| Vendor design tokens | `apps/web/src/styles/vendor/tokens.css` | The new callout reuses an existing measured token pair. | no token change |

## User Experience and Interface Impact

`/recommend` gains one rule sentence when `useCaseRule.requiresDiscreteGpu` is true, a
plain sentence for the other use cases, and an accurate option-count statement
("1 materially different build found; no option was padded to reach three"). The
existing heading, live region, focus target, comparison table, keyboard order and
390 px stacking stay as they are. A gaming no-solution names the graphics card in the
same block that already carries the binding constraint.

## Out of Scope

- No new request field and no change to `POST /api/ai/parse`, `POST /api/ai/revise`,
  the BYOK panel or any provider adapter.
- No licensed catalog, live pricing, purchase or reservation.
- No change to scoring weights, the work limit or the persisted schema.

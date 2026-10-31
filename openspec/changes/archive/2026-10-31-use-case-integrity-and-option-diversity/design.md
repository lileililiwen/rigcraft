# Design: Use-case integrity and option diversity

## Problem shape

Two independent truths are conflated in one place. Completeness is a hardware question
— is there a graphics path and a cooling path? — and the search currently answers it
with `igpu=yes` regardless of why the machine is being built. Diversity is a
choice question, and the search currently answers it with the full SKU set, which
counts a 1 TB, 2 TB and 4 TB drive as three different machines.

The fix separates them:

1. The use case adds a required category before the search starts.
2. A material signature collapses options after the search, before ranking is
   presented.

Both are pure functions of the validated request and the snapshot, so determinism is
preserved.

## Use-case completeness rule

`build_input` marks `gpu` as a required pool when `settings.use_case == "gaming"`.
That one flag reaches every place completeness is decided:

- `Walker::walk` cannot skip the pool, so every leaf contains a GPU.
- `suffix_min` prices the cheapest GPU into the pruning bound.
- `missing_required` reports `stock` with category `gpu` when no card is usable.
- `required_cheapest_blockers` names the cheapest card.
- `minimum_complete_cost` adds the card to the cheapest complete build.

`office` and `creation` leave the pool optional, which is today's behaviour: the
compatibility engine's completeness rule still requires a GPU unless the CPU has
sourced `igpu=yes`.

`minimum_complete_cost` is restructured so the GPU is counted once. The CPU minimum
adds a GPU when the use case requires one or when that CPU has no `igpu=yes`; the rest
of the required pools excludes `cpu`, `gpu` and `cooler`, because those three are
priced per CPU.

## No-solution wording

No new `binding` value is introduced. The vocabulary stays `budget`, `stock`,
`compatibility`, `search_limit`.

- Budget below the cheapest GPU-inclusive build → `budget`, with a headline that names
  a gaming build with a discrete graphics card, a `use_case_requirement` blocker naming
  the cheapest card, and `minimumBuildMinor` / `budgetShortfallMinor` computed from the
  GPU-inclusive minimum.
- No usable card in stock for gaming → `stock`, with the `gpu` blocker saying the
  snapshot has no card and gaming requires one.
- Cards exist but every complete gaming combination fails a rule → `compatibility`,
  unchanged.
- Work limit → `search_limit`, unchanged.

## Material signature

```text
material_signature = sorted(category:sku:quantity for every chosen part
                            where category != "storage") joined by "|"
```

Storage is deliberately excluded. Completeness already requires one storage part, and
its capacity is the one difference REQ-R02 says is not a different option. Everything
else — CPU, motherboard, memory, GPU, PSU, case, cooler and their quantities — counts.

The walker retains one representative per material signature in its top pool, choosing
the cheapest variant (equivalently the highest-scoring one, because the cost dimension
falls as the total rises) and breaking ties on score and full signature. The global
cheapest and highest-performance builds are still tracked separately so they can be
offered even when the top pool is full of other signatures.

Assembly keeps the existing objective order and deduplicates on the material signature:

1. the highest-scoring representative is `best_overall`;
2. the cheapest representative, if materially different, is `lowest_cost`;
3. the highest-performance representative, if materially different, is
   `highest_performance`;
4. remaining representatives fill `alternative` until three are shown.

If one signature exists, one candidate is returned. No placeholder is invented.

## Report shape

`RecommendationReport` gains:

```json
"useCaseRule": {
  "useCase": "gaming",
  "requiresDiscreteGpu": true,
  "acceptsIgpu": false,
  "explanation": "Gaming requires a discrete graphics card ..."
}
```

`Candidate` gains `materialSignature`, so a test or export can prove two options are
not the same machine with a different drive. Neither field changes the request
contract, the schema or the AI parse contract.

## Why the search stays bounded and deterministic

The material signature does not change the traversal, only which evaluated builds are
retained and which are presented. Retention is capped at the existing `TOP_POOL` (32)
after collapsing, so memory is bounded. Every comparison has a final tie-break on the
full signature, and the representative choice is a total order (total, score,
signature), so an identical request against an unchanged snapshot reproduces the same
candidates, labels and order.

## Rejected alternatives

- **Add a `requireDiscreteGpu` request field.** Rejected: the use case already means
  this, the change would grow the AI contract, and BYOK is out of scope.
- **Make `igpu=yes` a compatibility conflict for gaming.** Rejected: the compatibility
  engine answers hardware questions over a selection and does not know the use case.
  Enforcing it there would make an office check fail for a fact that is not a conflict.
- **Signature over the total price or the score.** Rejected: the requirement says
  structural, not price, and price changes with stock and budget while the machine does
  not.
- **Substitute a cheaper drive in the representative instead of excluding storage from
  the signature.** Rejected: it would still present the same machine three times with
  different drives, which is exactly the padding REQ-R02 forbids.

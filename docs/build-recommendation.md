# Budget and stock constrained build recommendation

Status: implemented in `inventory-build-recommendation` (2026-10-31). Updated by
`use-case-integrity-and-option-diversity` (2026-11-01): gaming requires a discrete
graphics card, and options are distinct material builds rather than storage variants.

This package delivers the recommendation, comparison and export outcome required by
`requirement.md` (REQ-D05, REQ-I01–I03, REQ-R01–R05). It reads one immutable
inventory snapshot, searches complete compatible builds inside a budget and the
available stock, and explains either the ranked options or the binding constraint.
AI planning, licensed catalogs, live pricing, purchase, reservation and build
diagnosis remain out of scope.

## Runtime

| Target | Command | Notes |
| --- | --- | --- |
| Rust API | `cargo run` | Same process and SQLite file as inventory and compatibility. |
| Web build | `npm run build` | `/recommend` is served by the Rust SPA fallback. |
| Tests | `cargo test`, `npm test` | Domain/API tests and Vitest component/state tests. |

The route is `/recommend`. It needs an inventory snapshot; when none exists it links
to `/inventory`. No AI key, catalog or network service is consulted.

## HTTP contract

All JSON uses camelCase. Every rejection, including a malformed or wrongly typed JSON body, is `application/json` in this shape (`invalid_json` / `invalid_body` for extractor failures):

```json
{ "error": { "code": "budget_required", "message": "...", "field": "budgetMinor" } }
```

### `POST /api/recommendations`

```json
{
  "snapshotId": "uuid",
  "budgetMinor": 200000,
  "currency": "USD",
  "useCase": "gaming",
  "inventoryOnly": true,
  "lockedSkus": ["FIX-CPU-AM5"],
  "excludedSkus": ["FIX-GPU-LONG"]
}
```

- `budgetMinor` is a positive integer in minor units (400 `budget_required`, `budget_invalid`).
- `currency` must be the snapshot currency (400 `currency_required`, `currency_format`, `currency_mismatch`). It is case-insensitive and normalized to three uppercase ASCII letters (`usd` becomes `USD`; `us` is rejected).
- `useCase` is `gaming`, `office` or `creation` (400 `use_case_required`, `use_case_unsupported`).
- `inventoryOnly` may be omitted or true. False is rejected with 400 `catalog_unavailable` because no licensed catalog exists; inventory-only is never silently bypassed.
- A SKU may appear once in each list (400 `duplicate_sku`, `locked_sku_invalid`, `excluded_sku_invalid`).
- A SKU cannot be both locked and excluded (400 `locked_excluded_conflict`).
- One lock per single-instance category (400 `locked_category_conflict`).
- Locked and excluded SKUs must exist in the snapshot (422 `locked_sku_not_found`, `excluded_sku_not_found`).
- A locked SKU with zero stock cannot enter a checked build (422 `locked_sku_out_of_stock`).
- Unknown snapshot: 404 `snapshot_not_found`.

Returned report:

```text
id, createdAt, snapshotId, snapshotRevision, currency, budgetMinor,
ruleSetVersion = compat-rules-v1, scoringVersion = recommend-score-v1,
status(ok|no_solution|search_limited),
request { budgetMinor, currency, useCase, inventoryOnly, lockedSkus[], excludedSkus[] },
useCaseRule { useCase, requiresDiscreteGpu, acceptsIgpu, explanation },
candidates[] {
  rank, objective, label, materialSignature,
  parts[] { recordId, sku, category, model, quantity, owned, unitPriceMinor,
            lineTotalMinor, available },
  totalMinor, budgetDeltaMinor,
  stock[] { sku, category, used, available, remaining, owned },
  unitsFromStock, ownedUnits,
  overallStatus, completeness, checkReportId, checkSummary,
  unknownFollowUps[], scores { total, confidence, dimensions[] }, reasons[],
  alternatives[] { category, sku, model, lineTotalMinor, reason, detail },
  check { ...the full compatibility CheckReport... }
},
noSolution { binding, headline, explanation, blockers[], adjustable[],
             minimumBuildMinor, budgetShortfallMinor } | null,
search { limit, nodesExamined, leavesEvaluated, validBuilds, limited, method },
exclusions[] { sku, category, model, reason, detail },
stale, currentSnapshotRevision, staleReason?
```

`useCaseRule` states the completeness rule the request imposed. For `gaming`,
`requiresDiscreteGpu` is true and `acceptsIgpu` false; for `office` and `creation`
both are the reverse. `materialSignature` is the structural identity described under
"Materially different options".

### `GET /api/recommendations/{id}`

Returns the stored report and marks it `stale` with `currentSnapshotRevision` and a
reason when the correction chain has advanced or the snapshot no longer exists.
Unknown id: 404 `recommendation_not_found`.

### `GET /api/recommendations/{id}/export?format=json|markdown`

- `json` (default): `application/json` with `exportVersion = recommend-export-v1`,
  `exportedAt`, the snapshot id/revision/currency/source filename/checksum, the exact
  stored `report` object, and a `checks[]` entry per candidate with its check report
  id, rule-set version, status, summary and unknown follow-ups.
- `markdown`: `text/markdown` plan with the same options, parts, prices, stock and
  verdicts, reasons, unselected alternatives, unknown follow-ups, evidence source
  references and a manual-confirmation notice. Every dynamic table cell escapes
  backslash and pipe, replaces line breaks with a space, and renders `<` and `>` as
  `&lt;` and `&gt;`, so operator text cannot add a column or inject HTML. The JSON
  export stays exact data and is never escaped.
- Both set `Content-Disposition: attachment` with a `.json` or `.md` filename.
- Any other format: 400 `export_format_unsupported`.

Export never recomputes prices or verdicts, so it matches what the UI showed for
that report identifier.

## Hard constraints and stock

- Only records in the selected snapshot are eligible. The current package has no catalog,
  so every recommendation is inventory-only.
- A candidate must be complete: CPU, motherboard, memory, power supply, case and storage,
  plus a cooler unless a CPU carries sourced `bundled_cooler=yes`.
- Completeness for graphics depends on the use case:
  - `gaming` requires a discrete graphics card in every candidate. A CPU's sourced
    `igpu=yes` fact does not complete a gaming build, and a budget that only covers an
    integrated-graphics machine is a named no-solution rather than a gaming plan.
  - `office` accepts a CPU with sourced `igpu=yes` when no card is selected.
  - `creation` keeps the optional-graphics behaviour and its explanation states that a
    discrete graphics card was not required.
- CPU, motherboard, GPU, PSU, case and cooler are single-instance categories. Memory may
  be selected at one or two kits of the same SKU when stock allows.
- Quantity used never exceeds snapshot stock. A lock is an owned part: zero incremental
  price and zero stock consumption, but the SKU must exist and have stock so the
  compatibility check can run.
- Excluded, out-of-stock, over-budget, mixed-currency and conflicting-duplicate-lot SKUs
  are removed before search and listed in `exclusions` with a reason. They are never
  silently dropped.

## Ranking and scoring

Weights are fixed and published (`recommend-score-v1`):

| Use case | Budget headroom | Performance proxy | Upgrade room |
| --- | --- | --- | --- |
| `gaming` | 0.30 | 0.55 | 0.15 |
| `office` | 0.60 | 0.20 | 0.20 |
| `creation` | 0.25 | 0.50 | 0.25 |

- Budget headroom is derived from the total price and the requested budget.
- The performance proxy is the mean of confirmed `perf_score` values (unit `pt`) recorded
  on the CPU and graphics card. It is user-provided evidence; it is never inferred.
- Upgrade room is free memory slots derived from the board's confirmed `ram_slots` and the
  memory's confirmed `modules` facts.
- A dimension with missing evidence is `available: false`, `value: null` and weight `0`.
  It is never scored as zero, and `confidence` becomes `partial`. Remaining weights are
  renormalised over the available dimensions.
- Each option states its objective (`best_overall`, `lowest_cost`, `highest_performance`
  or `alternative`), why it was chosen, and which same-category alternatives were not.
- At most three options with distinct material signatures are shown. Incompatible or
  incomplete builds are never candidates. An `unknown` verdict is labelled and counted,
  never presented as verified compatibility.

## Materially different options

REQ-R02 asks for up to three materially different options, not three prices for one
machine. The report therefore defines a structural signature:

```text
sorted(category:sku:quantity for every chosen part where category != "storage")
```

- Storage is excluded deliberately. Completeness already requires one drive, and a
  larger drive is a capacity, not a different machine: a 1 TB, 2 TB and 4 TB variant of
  otherwise identical builds are one option. The representative is the cheapest variant,
  which is also the highest-scoring one because the budget-headroom dimension rises as
  the total falls.
- Every other chosen part and quantity is material: a different CPU, board, memory kit,
  graphics card, PSU, case or cooler is a different build.
- Candidates with the same signature collapse. The remaining representatives are offered
  in the fixed objective order: best overall, lowest cost, performance first, then
  alternative.
- When only one or two representatives exist, exactly that many are returned. The
  report does not invent a third, and the Web page says no option was padded. The
  candidate count is always the real count.
- The signature is not price- or score-derived, so a budget change cannot rename an
  option into a different machine.

## Bounded search

The search is a deterministic depth-first enumeration over the fixed category order
`cpu, motherboard, ram, gpu, psu, case, cooler, storage`, with over-budget partial builds
pruned. Complete combinations are checked with the compatibility engine before they can
become candidates. Expansion stops at **100,000 nodes** (`search.limit`). When the limit
is reached, `search.limited` is true and `status` is `search_limited`: the best options
found so far are returned, the UI says so, and no global optimum is claimed. An identical
request against an unchanged snapshot revision, rule set and scoring version reproduces
the same ordering, part sets, totals and scores; only report and check identifiers and
timestamps differ because each request persists new records.

## No solution

`noSolution.binding` is one of:

- `budget` — the cheapest structurally complete build exceeds the budget; the report names
  the minimum and the shortfall plus the cheapest part per required category.
- `stock` — a required category has no usable part; blockers name the category and why
  (not recorded, zero quantity, excluded, over budget, ambiguous).
- `compatibility` — complete combinations exist but every one was rejected; blockers count
  the conflicting rule ids or missing categories. No invalid build is shown.
- `search_limit` — the limit was hit before a valid build was confirmed.

No new binding value is added for the gaming rule. A gaming request whose cheapest
GPU-inclusive build exceeds the budget returns `budget`, names the discrete graphics card
in the headline, explanation and a `use_case_requirement` blocker, and states the
GPU-inclusive `minimumBuildMinor` and `budgetShortfallMinor`. A gaming snapshot with no
usable card returns `stock` and says gaming requires a discrete card. Cards that exist but
cannot form a compatible build return `compatibility`.

`adjustable[]` states what the operator can change. `minimumBuildMinor` and
`budgetShortfallMinor` are present when they are known.

## Persistence

Schema `PRAGMA user_version = 3` adds:

```sql
CREATE TABLE recommendations (
  id TEXT PRIMARY KEY,
  created_at TEXT NOT NULL,
  snapshot_id TEXT NOT NULL REFERENCES snapshots(id),
  snapshot_revision INTEGER NOT NULL,
  scoring_version TEXT NOT NULL,
  rule_set_version TEXT NOT NULL,
  status TEXT NOT NULL,
  request_json TEXT NOT NULL,
  report_json TEXT NOT NULL
);
```

Opening a version-2 database runs a forward migration that creates this table and leaves
snapshots, records, previews and check reports untouched. A recommendation and the check
reports of its candidates are written in one transaction; a failure writes neither.
Generating a plan never mutates a snapshot and never reserves stock.

## Web flow

States covered on `/recommend`: loading history, empty inventory, snapshot load error,
empty or invalid budget, generating, results, unknown warning, search-limited warning,
no solution, stale after an edit or a correction, export success and export error.
The results state the `useCaseRule` that applied (including that gaming required a
discrete graphics card) and the real number of materially different options, so a
one- or two-option report never implies three. The budget and use case are labelled,
lock/exclude choices are native checkboxes, status uses a symbol plus text,
asynchronous messages are announced in a polite live region, focus moves to the results
heading or the no-solution/error block, and the layout stacks at phone width without
horizontal page overflow. Rule evidence and comparisons are reachable by keyboard
through native tables and `<details>` disclosures.

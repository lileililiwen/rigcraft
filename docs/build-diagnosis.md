# Existing build diagnosis and upgrade advice

Status: implemented in `existing-build-diagnosis` (2026-10-31).

This package delivers the reuse and replacement advice required by `requirement.md`
(P1 "已有电脑升级规划；粘贴外部装机清单并诊断", with REQ-D03, REQ-C04, REQ-C05 and
REQ-I02). It takes a build the operator already owns — parts matched from one
immutable snapshot or described by hand — and diagnoses it with the same
compatibility engine and rule set as `/builds/check`. It then separates what must
be replaced, what should be verified first and what is fine as is, and validates a
replacement plan as one whole build before presenting it.

Out of scope: automatic local hardware detection, live pricing, purchase,
reservation, FPS prediction, merchant optimization and any licensed catalog. There
is no catalog in this package, so pasted lines are matched against the selected
snapshot only; a manual entry path always remains.

## Runtime

| Target | Command | Notes |
| --- | --- | --- |
| Rust API | `cargo run` | Same process and SQLite file as inventory, compatibility and recommendation. |
| Web build | `npm run build` | `/diagnose` is served by the Rust SPA fallback. |
| Tests | `cargo test`, `npm test` | Domain/API tests, a store migration test and Vitest component/state tests. |

The route is `/diagnose`. It needs an inventory snapshot for matching and
replacement prices; with none it explains the empty state and links to
`/inventory`. No AI provider, catalog or network service is consulted.

## HTTP contract

All JSON uses camelCase. Errors keep the inventory shape:

```json
{ "error": { "code": "part_invalid", "message": "...", "field": "build", "row": 4 } }
```

### `POST /api/diagnoses/parse`

```json
{
  "snapshotId": "uuid",
  "currency": "USD",
  "text": "gpu: Fixture GeForce RTX 4070\ngpu: Mystery X9000 Adapter"
}
```

- `text` is limited to 8 KiB and 40 non-empty lines. Empty input is 400
  `text_required`; oversized text is 400 `text_too_large`; too many lines is 400
  `line_limit_exceeded` with `details.limit`.
- `currency` is required and must be three uppercase letters (400
  `currency_required`, `currency_format`) and must equal the snapshot currency
  (400 `currency_mismatch`).
- `snapshotId` may be omitted, in which case every line is unmatched and manual
  entry is the only path. An unknown snapshot is 404 `snapshot_not_found`.
- Nothing is persisted and no diagnosis exists after parsing.

Each line is parsed as `[category:] model [xN] [@price]`. The response:

```text
matchVersion = diagnosis-match-v1, snapshotId, currency, lineCount,
lines[] {
  line, raw, status(matched|ambiguous|unmatched|invalid), category|null, model,
  quantity, priceMinor|null,
  issues[] { code, message },
  candidates[] { sku, category, model, priceMinor, currency, available,
                 coverage, exact, sharedTokens[], extraTokens[], missingTokens[],
                 provenance|null },
  disambiguation?, duplicateOf?
},
limits { maxLines, maxBytes, maxCandidates }
```

- Model matching normalises to lowercase alphanumeric tokens, splitting a
  letter/digit run only when both sides keep two characters (`RTX4070` → `rtx`,
  `4070`; `B650` stays one token). Candidates must contain at least 60 % of the
  pasted tokens; at most five are returned, exact-token matches first.
- `matched` requires exactly one candidate whose token set equals the pasted set
  (or the pasted text is its SKU). Several such candidates, or only fuzzy
  candidates, stay `ambiguous`; nothing is coerced to the nearest SKU.
- Every ambiguous or unmatched line states what would resolve it: the record
  tokens the line did not paste (e.g. `twin`, `wind`), the pasted tokens the
  record lacks, or the SKU. No candidate is preselected for an ambiguous line.
- A repeated category and model on a later line keeps its own status and names
  the earlier line in `duplicateOf`; the duplicate is never added silently.
- A category alone never produces a candidate, and no specification is inferred
  from a model name.

### `POST /api/diagnoses`

```json
{
  "snapshotId": "uuid",
  "currency": "USD",
  "build": [
    { "source": "snapshot", "sku": "FIX-MB-AM5", "quantity": 1 },
    { "source": "owned", "category": "cpu", "model": "Fixture AM4 CPU 5600",
      "quantity": 1, "specs": "socket=AM4;tdp=65W;peak_power=88W;igpu=no" }
  ]
}
```

- 1–40 parts (400 `build_empty`, `build_too_large`). Every part is already owned:
  it adds no purchase cost and takes no stock.
- `source: "snapshot"` names an existing SKU (422 `sku_not_found`, 422
  `sku_ambiguous` for conflicting lots) and takes category, model, specifications
  and provenance from that record. A contradicting category is 400 `part_invalid`.
- `source: "owned"` is validated with the inventory rules: a supported category,
  a 1–200 character model, a positive whole quantity up to 1000, at most 2 KiB of
  specifications parsed by the same parser (400 `part_invalid` with
  `details.specCode`). An unrecognised specification key stays `unknown`.
- The same category and model twice is 400 `duplicate_part`; a reused SKU is 400
  `duplicate_part`; two parts in a single-instance category are 422
  `category_multiple`.
- Validation completes before any check or write, so a rejected request stores no
  partial build.
- Currency is required and must equal the snapshot currency (400
  `currency_required`, `currency_format`, `currency_mismatch`). Unknown snapshot:
  404 `snapshot_not_found`.

Returned report:

```text
id, createdAt, snapshotId, snapshotRevision,
ruleSetVersion = compat-rules-v1, matchVersion = diagnosis-match-v1,
adviceVersion = diagnosis-advice-v1, currency,
build[] { row, source(description|snapshot), sku, category, model, quantity,
          owned, reportedPriceMinor, specs[], provenance, match },
check { ...the compatibility CheckReport for the described build... },
advice[] { row, sku, category, model,
           action(must_replace|should_upgrade|keep), conditional, reason,
           ruleIds[], evidence[], verification?, cost {
             currency, keepMinor, replacementSku?, replacementModel?,
             replacementMinor?, additionalMinor?, note } },
groups { mustReplace[], shouldUpgrade[], keep[] },
plan {
  status(not_needed|validated|needs_evidence|no_option|no_valid_plan),
  summary, steps[] { row, category, fromSku, fromModel, toSku, toModel,
                     priceMinor, ruleIds[], reason },
  options[] { row, category, sku, model, priceMinor, status, resolves[],
              ruleIds[], followUps[] },
  rejected[] { row, category, sku, model, priceMinor, ruleIds[], detail },
  additionalCostMinor, currency, followUps[], evaluated, method, validation?
},
stale, currentSnapshotRevision, staleReason?
```

The embedded check's `snapshotId` is the synthetic described-build id
(`diagnosis:described-build`), never a purchase snapshot; its facts come from the
operator's description and the matched snapshot records. The diagnosis itself
references the real snapshot and revision.

### `GET /api/diagnoses/{id}`

Returns the stored report. When the correction chain rooted at the report's
snapshot has advanced, `stale` is true and `staleReason` names both revisions.
Unknown IDs are 404 `diagnosis_not_found`.

## Advice policy

Advice is a projection of the compatibility report and of the plan, never a second
opinion:

- `must_replace` — the part is replaced by a validated step in the plan, or it is
  in a definite conflict and no plan can be validated. The reason quotes the
  conflict and the rule identifiers; evidence lists the compared fields of that
  part with provenance.
- `should_upgrade` (`conditional: true`) — no conflict is recorded, but a required
  fact is missing, so the checker cannot confirm the part. The reason and
  `verification` name the exact question; the cost note says nothing is bought
  until the evidence is confirmed.
- `keep` — every rule involving the part passed, or the plan resolves that part's
  conflict by replacing another part.
- Cost is explicit for every part: keeping costs zero; a replacement price comes
  from the snapshot. Retained owned parts are never charged again and no trade-in
  or resale value is assumed. When the snapshot has no candidate, the report says
  so instead of inventing a SKU or price.

## Plan validation

For each conflicting rule the engine searches replacement candidates for the rule's
parts, cheapest first, substitutes one into the described build and re-checks the
whole build with `compat-rules-v1`. A substitution qualifies as a resolution only
when that rule is no longer incompatible. At most 25 candidates are evaluated per
part and the count is reported. The chosen resolutions are composed and checked
again as one build:

| Composed verdict | Plan status | Presented |
| --- | --- | --- |
| compatible | `validated` | Steps, prices, whole-plan check |
| unknown | `needs_evidence` | Steps, prices, every follow-up, no claim of validity |
| incompatible | `no_valid_plan` | No steps; rejected candidates and their rules |
| no candidate resolves a conflict | `no_option` | No steps; the snapshot has no option |
| no conflict at all | `not_needed` | No steps |

A plan the checker would reject is never shown as valid. A plan reserves no stock,
mutates no snapshot and writes no purchase order.

## Facts and provenance

- Snapshot-sourced parts keep the record's own `source`, `reference` and
  `observedAt`; hand-entered parts record `user` with a `diagnosis-entry:` reference
  and the model normalisation separately.
- `match.confidence` stays `unverified`: no licensed catalog is consulted, so a
  model match is a lexical suggestion confirmed by the operator's SKU choice.
- Fixtures in `tests/fixtures/diagnosis/` are synthetic; every model starts with
  `Fixture`.

## Persistence

SQLite schema version is `4` (was `3`). The new table is derived data:

```sql
diagnoses (
  id TEXT PRIMARY KEY,
  created_at TEXT NOT NULL,
  snapshot_id TEXT NOT NULL REFERENCES snapshots(id),
  snapshot_revision INTEGER NOT NULL,
  rule_set_version TEXT NOT NULL,
  status TEXT NOT NULL,
  report_json TEXT NOT NULL
)
```

The forward migration creates the table without touching snapshots, records,
previews, check reports or recommendations. A diagnosis does not write a check
report row and does not mutate a snapshot. Database location remains
`RIGCRAFT_DATA_DIR`, else `$XDG_DATA_HOME/rigcraft`, else
`$HOME/.local/share/rigcraft`.

## Web states

`/diagnose` covers: no snapshot, paste entry, match review, ambiguity, unmatched
line, duplicate line, invalid line, diagnosing, results with the three advice
groups, validated plan, needs-evidence plan, no-option plan, rejected plan, request
failure with retry, and stale after a snapshot revision. It uses labelled controls,
a polite live status region, a `role="alert"` failure, focus on the match-review and
result headings, symbol-plus-text status, visible focus, stacked cards at phone
width and reduced-motion-safe transitions.

## Evidence

- `docs/assets/diagnosis-*.png` — browser states at 1440×900 and 390×844.
- `docs/evidence/diagnosis-browser.md` — journey, keyboard, responsive,
  accessibility, privacy and measured contrast.
- `docs/evidence/token-contrast.json` — measured WCAG 2.2 AA contrast.
- `docs/evidence/diagnosis-lighthouse/` and `docs/evidence/diagnosis-lighthouse-mobile/`
  — Lighthouse audits.

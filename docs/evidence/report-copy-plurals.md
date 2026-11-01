# Report copy plurals evidence (2026-11-01)

The `(s)` pseudo-plural is gone from the service's user-visible prose. This
document quotes the sentences the running service produced after the change and
names where each came from. No sentence below was edited by hand; every value is
copied from the JSON or Markdown body the service returned.

## How it was measured

- Build: `cargo build` at commit `HEAD` of `main` (debug binary
  `target/debug/rigcraft`), data directory `/tmp/opencode/report-copy-plurals/data`
  (scratch, outside the repository), bound to `127.0.0.1:33231` with
  `RIGCRAFT_ADDR` / `RIGCRAFT_DATA_DIR`.
- The shipped sample was fetched from `GET /api/imports/sample`, previewed with
  `POST /api/imports` (10/10 valid, 0 duplicate rows) and committed with
  `POST /api/imports/{previewId}/commit`. Snapshot
  `50522a51-aae4-4e5b-bc5b-b51c3bd887ef`, revision 1, 10 records.
- The recommendation was `POST /api/recommendations` with
  `budgetMinor: 200000`, `currency: "USD"`, `useCase: "gaming"`,
  `inventoryOnly: true`. Report `a6ed880e-6ef0-4fa9-913d-688e0bdb90c4`.
- The exports were `GET /api/recommendations/{id}/export?format=json` and
  `?format=markdown`. The diagnosis calls were `POST /api/diagnoses`.

## The sentences

| Count | Rendered sentence | Where it came from |
| --- | --- | --- |
| 8 | `Total 1,672.90 USD leaves 327.10 USD under the budget, and snapshot stock supplies 8 units.` | `reasons[]` of candidate 1 in the recommendation JSON, and the same string in the JSON export and the Markdown `## Ranked options` block |
| 1 | `Total 94.00 USD leaves 1,906.00 USD under the budget, and snapshot stock supplies 1 unit.` | the same endpoint with every other part locked (`lockedSkus`); only the case remained unowned |
| 0 | `Total 0.00 USD leaves 2,000.00 USD under the budget, and snapshot stock supplies 0 units.` | the same endpoint with all eight parts locked |
| 1 | `The checker validates the upgraded build as compatible. 1 step adds 249.00 USD; no trade-in or resale value is assumed.` | `plan.summary` of a validated diagnosis over the synthetic `tests/fixtures/diagnosis/upgrade.csv` snapshot |
| 2 | `The checker validates the upgraded build as compatible. 2 steps add 308.00 USD; no trade-in or resale value is assumed.` | the same snapshot with an owned AM4 CPU and an owned DDR4 kit described; the plan replaces both, so two steps |
| 1 | `This snapshot holds no validated replacement for 1 rule: cpu-socket, so no upgrade plan is offered.` | `plan.summary` of a `no_option` diagnosis over the synthetic `tests/fixtures/diagnosis/no-replacement.csv` snapshot |
| 2 | `The search checked ...` and the Markdown `- Search: 14 nodes, 6 complete combinations, 3 valid builds, limit 100000` | `search` section of the same report; the Markdown line is from the export |

The `upgrade.csv` and `no-replacement.csv` snapshots are the committed synthetic
fixtures, labelled `Fixture` in every model name; they are used only because the
shipped sample has no rule whose replacement is absent and no compatible
two-step plan. No licensed catalog data was used.

## Diagnosis on the shipped sample

Describing an owned AM4 CPU against the sample's AM5 board produced
`check.overallStatus = "incompatible"` and a `needs_evidence` plan:

> No conflict remains, but the upgraded build is not fully validated: the
> checker returns unknown. Confirm the listed facts before treating this plan as
> compatible.

That is unchanged prose, quoted to show no `(s)` remains there either.

## Export round trip

The JSON export's `report.candidates[0].reasons` is byte-identical to the API
response's reason array (`reason equals API: True`), so the export carries the
same `8 units` sentence rather than a separately composed one. The Markdown
export carries the same reason line and the `- Search:` line above.

## What did not change

- Every identifier, error code, JSON field name, schema version, route, status
  code, scoring value and rule id is unchanged; only `String` prose moved.
- The e2e-pinned `1 materially different build found; no option was padded to
  reach three.` and `Every described part was checked by at least one rule.`
  were not touched; the source scan and the browser suite both pass.
- The API refusal codes named in the specs (`insufficient_stock`,
  `backup_wrong_shape`, `no_option`, `compatibility`) keep their codes and
  `details` shapes. The stock sentence now reads `needs 99 units` (the
  `tests/compatibility_api.rs` assertion) and the backup detail now reads
  `missing N tables: ...`.

## Explicitly not applicable

- No Lighthouse run: no layout, colour, route, markup or interactive control
  changed, so a new accessibility/performance score would measure the same page.
- No browser capture: the change is visible only in words, and the guard test
  plus the API/export assertions above are the evidence. `cargo test --test
  copy_style` scans `src/` and `apps/web/src/` and fails on a regression.

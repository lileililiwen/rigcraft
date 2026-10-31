# Evidence-backed compatibility

Status: implemented in `evidence-backed-compatibility` (2026-10-31).

This package delivers the three-state compatibility verdict required by `requirement.md` (REQ-I03, REQ-C01–C05). It reads one immutable inventory snapshot, compares the selected parts' recorded facts, and reports every rule with its inputs and provenance. Recommendation, AI planning, catalog distribution and build diagnosis remain out of scope.

## Runtime

| Target | Command | Notes |
| --- | --- | --- |
| Rust API | `cargo run` | Same process and SQLite file as the inventory package. |
| Web build | `npm run build` | `/builds/check` is served by the Rust SPA fallback. |
| Tests | `cargo test`, `npm test` | Domain/API tests and Vitest component/state tests. |

The check route is `/builds/check`. The page needs an inventory snapshot; when none exists it links to `/inventory`.

## HTTP contract

All JSON uses camelCase. Every rejection, including a malformed or wrongly typed JSON body, is `application/json` in this shape (`invalid_json` / `invalid_body` for extractor failures):

```json
{ "error": { "code": "sku_not_found", "message": "...", "field": "selection", "details": { "sku": "..." } } }
```

### `POST /api/builds/check`

```json
{
  "snapshotId": "uuid",
  "selection": [{ "sku": "FIX-CPU-AM5", "quantity": 1 }]
}
```

Selection rules:

- `snapshotId` is required (400 `snapshot_required`).
- At least one item is required (400 `selection_empty`).
- Each item needs a non-empty SKU and a positive whole quantity (400 `selection_invalid`).
- A SKU may appear once (400 `duplicate_selection`); lots with the same SKU are combined only when category, normalized model and specifications are identical, otherwise 422 `sku_ambiguous`.
- CPU, motherboard, graphics card, power supply, case and cooler are single-instance categories (422 `category_multiple`). Memory and storage can have several lines.
- Unknown snapshot 404 `snapshot_not_found`; absent SKU 422 `sku_not_found`; quantity above combined stock 422 `insufficient_stock`; unsupported category 422 `category_unsupported`.

The response is the persisted report:

```text
id, snapshotId, snapshotRevision, ruleSetVersion, checkedAt, selection[],
completeness { status(complete|incomplete), missing[{category, reason, followUp}] },
overallStatus(compatible|incompatible|unknown),
rules[] {
  ruleId, ruleVersion, title, category, status,
  parts[{recordId, sku, category, model}],
  evidence[{part, field, value, provenance{source, reference, observedAt}}],
  explanation, followUp, method
},
summary { compatible, incompatible, unknown },
stale, currentSnapshotRevision, staleReason?
```

`value` and `provenance` are `null` when the fact is absent; the `followUp` names what to confirm. Rules sort by category order (cpu, memory, gpu, cooling, case, power, storage) then `ruleId`. The rule-set version is `compat-rules-v1`. Identical snapshot revision, selection and rule-set version produce identical rule order and verdicts.

### `GET /api/builds/checks/{id}`

Returns the stored report. `stale` becomes `true` with `staleReason` when the correction chain rooted at the report's snapshot has advanced (the immutable snapshot row never changes, so staleness follows `parentSnapshotId` children). A missing snapshot also marks the report stale. Unknown IDs return 404 `check_not_found`.

## Verdict policy

- `compatible`: every rule that could run passed and the build is complete.
- `incompatible`: at least one rule found a definite conflict.
- `unknown`: no conflict, but a rule lacks a required fact or the build is incomplete.

Conflict overrides unknown; unknown overrides compatible. A complete build needs CPU, motherboard, memory, power supply, case and storage, plus graphics unless the CPU records `igpu=yes`, plus cooling unless the CPU records `bundled_cooler=yes`. Those two facts must be user-confirmed specs; their absence stays unknown.

## Rule catalog (`compat-rules-v1`)

| Rule ID | Compares | Definite conflict | Unknown when |
| --- | --- | --- | --- |
| `cpu-socket` | CPU `socket` = board `socket` | Values differ | Either value absent |
| `cpu-bios` | Board `bios_supports` names the CPU model or SKU | Never inferred; a non-matching list stays unknown | Board evidence absent or does not name the CPU |
| `ram-type` | Board `memory_type` = each kit `memory_type` | Any mismatch | Board or a kit value absent |
| `ram-capacity` | Σ(`capacity` × quantity) ≤ board `max_memory` | Total exceeds maximum | Maximum or a capacity absent |
| `ram-slots` | Σ(`modules` × quantity) ≤ board `ram_slots` | Modules exceed slots | Slot or module count absent |
| `gpu-interface` | GPU PCIe generation ≤ board `pcie_version` generation | GPU generation higher | Either value absent/unparseable |
| `gpu-length` | GPU `length` ≤ case `max_gpu_length` | Length exceeds clearance | Either value absent |
| `cooler-socket` | Cooler `cooler_socket` = board `socket` | Values differ | Either value absent |
| `cooler-clearance` | Air `height` ≤ case `max_cooler_height`; liquid `radiator_size` ≤ case `front_radiator_mm` | Size exceeds recorded clearance | Either size/clearance absent, including unknown radiator bay space |
| `board-case-form-factor` | Board form factor ≤ case `supported_form_factors` (E-ATX > ATX > micro-ATX > mini-ITX) | Board larger than the case maximum | Either value absent/unrecognized |
| `psu-capacity` | Supply `wattage` versus nominal and sourced peak draw + 50 W screening headroom | Wattage below combined nominal CPU/GPU draw | Wattage, nominal draw, graphics evidence, or peak draw absent |
| `psu-gpu-connectors` | Required GPU connector tokens ≤ supply `psu_connectors` tokens | Any pin size short | Either list absent/unparseable |
| `psu-case-mount` | `form_factor` equality and supply `length` ≤ case `max_psu_length` | Form differs or length exceeds limit | Either form or length absent |
| `storage-interface` | Board `storage_interface` = each drive `interface` | Any mismatch | Board or a drive value absent |

Power is screening only. A passing nominal estimate without sourced peak draw is `unknown`, never `compatible`; only a wattage below nominal draw is a definite conflict. Socket equality or dimensions alone never claim that the machine boots or installs.

## Facts and provenance

- Every compared value cites the snapshot spec's `source`, `reference` and `observedAt`. An import without a source column records `user` with `csv:<filename>`. An import that declares `source=catalog:<id>` and `sourceUrl` cites those for the model and specifications, while SKU, quantity and price stay `user` (`docs/inventory-import.md`). `match.confidence` is `catalog-declared` for such a row and `unverified` otherwise; neither means RigCraft queried or verified the source.
- The shipped sample declares `catalog:manufacturer@2026-10-31` and a manufacturer page per row. Its complete build reports 12 compatible and 2 unknown rules, no conflict; the two unknowns are the BIOS support and peak-power facts the cited pages do not publish. `tests/compatibility_api.rs` holds that assertion.
- Facts with `status: "unknown"` are not used by a rule. Unknown keys and absent specs produce a `followUp`, never a default (for example, storage is not assumed SATA and a cooler is not assumed to fit).
- Fixtures in `tests/fixtures/compatibility/` are synthetic. Their models are prefixed `Fixture` and they are imported as ordinary user-owned inventory, never presented as licensed catalog data.
- PCIe values are read from the first number in the recorded value, so the manufacturer's own wording (`PCIe 4.0 x16`, `PCIe5`, `5.0`) yields the generation instead of every digit in the string.

## Persistence

SQLite schema version is `2` (was `1`). The new table is derived data:

```sql
check_reports (
  id TEXT PRIMARY KEY,
  snapshot_id TEXT NOT NULL REFERENCES snapshots(id),
  snapshot_revision INTEGER NOT NULL,
  rule_set_version TEXT NOT NULL,
  selection_json TEXT NOT NULL,
  overall_status TEXT NOT NULL,
  completeness TEXT NOT NULL,
  checked_at TEXT NOT NULL,
  report_json TEXT NOT NULL
)
```

Checks do not mutate snapshots, reserve stock, or write a partial report. Database location remains `RIGCRAFT_DATA_DIR`, else `$XDG_DATA_HOME/rigcraft`, else `$HOME/.local/share/rigcraft`.

## Web states

The Rust process serves the SPA shell for extensionless client routes such as `/inventory` and `/builds/check`; an unknown static or metadata path is a real 404, and unknown `/api/*` routes remain JSON `route_not_found` 404 responses.

`/builds/check` covers no snapshot, empty selection, selecting, checking, complete, incompatible, unknown, stale after a part or snapshot change, snapshot-load failure, and check failure with retry. It uses labelled radio/checkbox groups and quantity fields, a live status region, `role="alert"` failures, visible focus, focus on the result heading, symbol-plus-text verdicts, stacked mobile cards, and reduced-motion-safe CSS. The result lists conflict diagnostics, every unknown follow-up, and a per-rule evidence disclosure with source and observation time.

## Evidence

- `docs/assets/compatibility-*.png` — browser states at 1440×900 and 390×844.
- `docs/evidence/compatibility-browser.md` — journey, keyboard, responsive, accessibility, privacy and measured contrast.
- `docs/evidence/token-contrast.json` — measured WCAG 2.2 AA contrast.
- `docs/evidence/compatibility-lighthouse/` and `docs/evidence/compatibility-lighthouse-mobile/` — Lighthouse accessibility audits.

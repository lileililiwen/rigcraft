# Evidence-backed compatibility

## ADDED Requirements

### Requirement: Every verdict is three-state and carries its rule, inputs, and provenance
The system SHALL judge a selected build with exactly one of `compatible`, `incompatible`, or `unknown`. Every rule result SHALL carry a stable rule identifier, the rule-set version, the parts and field values compared, the provenance of each value used, a readable explanation, and a `followUp` question whenever the result is `unknown`. The system SHALL NOT infer a missing Socket, BIOS, memory type, dimension, connector, capacity, or interface value from a similar model name or from a default. Provenance SHALL remain `user` with the inventory reference until a licensed source is separately verified.

#### Scenario: A passing rule exposes its evidence
Given a CPU and motherboard that both declare Socket AM5 with user-provided provenance, when the build is checked, then the socket rule is `compatible` and lists both compared values, their part identifiers, and their source references.

#### Scenario: Missing evidence produces a specific unknown
Given a matching CPU and motherboard Socket but no motherboard BIOS support fact, when the build is checked, then the BIOS rule is `unknown` and its `followUp` asks which BIOS support fact to confirm for that CPU.

### Requirement: Conflicts override unknown and unknown overrides compatible
The system SHALL compose the overall verdict so that any definite conflict yields `incompatible`, otherwise any unknown rule or incomplete build yields `unknown`, otherwise `compatible`. A build SHALL be complete only when CPU, motherboard, memory, power supply, case, and storage are selected, plus a graphics card unless the CPU has sourced integrated-graphics evidence, plus a cooler unless the CPU has sourced bundled-cooling evidence. An incomplete build SHALL NOT be reported as compatible. Unknown results SHALL be visually distinct from verified results and SHALL NOT rely on color alone.

#### Scenario: Definite conflict
Given a CPU with Socket AM4 and a motherboard with Socket AM5, when the user checks the build, then the overall status is `incompatible` and the socket rule names both values and sources.

#### Scenario: Conflict wins over a missing fact
Given a memory-type conflict and a separate rule with missing evidence, when the build is checked, then the overall status is `incompatible`, not `unknown`.

#### Scenario: Unknown without a conflict
Given no conflicts but a missing radiator-clearance fact for a liquid cooler, when the build is checked, then the overall status is `unknown` and the clearance rule names the fact to confirm.

#### Scenario: Incomplete build is not compatible
Given selected parts that all pass their rules but omit storage, when the build is checked, then the completeness status is `incomplete`, the missing storage category is named with a follow-up, and the overall status is `unknown`.

### Requirement: Required P0 rule coverage is deterministic
The system SHALL evaluate at least these rule identifiers for a complete selection: CPU-to-motherboard Socket (`cpu-socket`), CPU/BIOS support evidence (`cpu-bios`), board-to-memory type, capacity, and slot count (`ram-type`, `ram-capacity`, `ram-slots`), graphics interface and case length (`gpu-interface`, `gpu-length`), cooler mounting and case clearance including liquid-radiator clearance (`cooler-socket`, `cooler-clearance`), board-to-case form factor (`board-case-form-factor`), power-supply capacity screening, GPU connectors, and case mounting (`psu-capacity`, `psu-gpu-connectors`, `psu-case-mount`), and storage-to-board interface (`storage-interface`). Results SHALL be ordered by category then rule identifier and SHALL be deterministic for the same snapshot revision, selection, and rule-set version. The power-capacity rule SHALL treat a nominal/peak estimate as screening only and SHALL NOT claim safety when peak or headroom evidence is missing.

#### Scenario: Screen power without proof
Given a power supply whose wattage exceeds the combined nominal draw but no sourced peak draw, when the build is checked, then the power rule is `unknown` and asks for measured peak or transient evidence instead of reporting a safe result.

#### Scenario: Detect an insufficient connector
Given a graphics card requiring two 8-pin connectors and a power supply listing one 8-pin connector, when the build is checked, then the connector rule is `incompatible` and lists both connector facts.

### Requirement: The check API validates selection and persists revision-bound reports
`POST /api/builds/check` SHALL accept `snapshotId` and a `selection` array of SKU and positive integer quantity pairs, resolve each SKU against that immutable snapshot, and return `overallStatus`, `completeness`, `selection`, `snapshotRevision`, `ruleSetVersion`, `checkedAt`, and ordered `rules[]`. It SHALL return HTTP 400 with `selection_empty`, `selection_invalid`, `duplicate_selection`, or `snapshot_required` for malformed input, HTTP 404 with `snapshot_not_found` for an unknown snapshot, and HTTP 422 with `sku_not_found`, `sku_ambiguous`, `insufficient_stock`, or `category_multiple` when a selected SKU cannot be checked. A successful check SHALL persist the report in SQLite against the snapshot ID and revision with the rule-set version and selection. `GET /api/builds/checks/{id}` SHALL return the stored report and SHALL mark it `stale` with a reason when the referenced snapshot is missing or its revision has advanced. The system SHALL NOT reserve stock, mutate the snapshot, or write a partial report on failure.

#### Scenario: Reject an absent SKU
Given a selection naming a SKU that is absent from the snapshot, when the check is requested, then the service returns HTTP 422 with `sku_not_found` and no check report is written.

#### Scenario: Detect a stale report
Given a persisted check for snapshot revision 1, when a correction creates revision 2 and the stored check is read, then the response is marked `stale` with the current revision and does not present the old verdict as current.

### Requirement: The check Web flow is usable with keyboard, assistive technology, and narrow screens
The `/builds/check` route SHALL let the operator choose a snapshot, select parts and quantities, run a check, and read the overall verdict, completeness, rule evidence, conflict diagnostics, and specific unknown follow-ups. It SHALL cover empty-inventory, empty-selection, checking, complete, incompatible, unknown, stale-after-edit, and server-error states; SHALL preserve the selection on failure and offer retry; SHALL announce asynchronous completion in a live status region; SHALL move focus to the result heading or error alert; and SHALL NOT require a pointer gesture. Status SHALL use text and a symbol in addition to color. The layout SHALL stack at narrow widths without horizontal overflow, and fixture inventory used for captures SHALL be labelled synthetic and user-provided, never as licensed catalog data.

#### Scenario: Mark the verdict stale after a part change
Given a completed check, when the operator changes a selected part or its quantity, then the previous verdict is marked stale and a re-check is required before it is presented as current.

#### Scenario: Read an unknown follow-up at phone width
Given an `unknown` result at 390 px width, when the operator navigates by keyboard, then the verdict, the specific follow-up question, the evidence disclosure, and the re-check action are reachable with visible focus and no horizontal overflow.

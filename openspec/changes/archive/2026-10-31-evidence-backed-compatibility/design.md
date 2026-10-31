# Design: Check complete build compatibility

## Boundary and runtime
Implement in RigCraft only. Rust edition 2021 and Cargo own domain, API and SQLite access; add Axum and SQLite dependencies when implementing the first HTTP slice. React/TypeScript with Vite owns `apps/web`; use the pinned shared tokens. Rust API and Web app are separate build targets, served together in local development. Do not import sibling source at runtime. `cargo test` and the Web build/test scripts must be bound to the local Gate when added.

## Ownership and contracts
Rust domain types in src/ for Build, Part, RuleResult and CheckReport; SQLite stores build drafts and references the inventory snapshot, while rule version is embedded in each report. POST /api/builds/check accepts snapshotId and selected SKU/quantity pairs; returns overallStatus, completeness, rules[] with status, field values, source/version and followUp. 400 for malformed selection, 404 for unknown snapshot, 422 for absent SKU.

`RuleResult` has stable `ruleId`, `status` (`Compatible|Incompatible|Unknown`), compared part IDs and field values, provenance references, and `followUp` for unknown. `CheckReport` has snapshot ID, rule-set version, checkedAt, completeness and ordered results. Rule IDs are versioned constants and results sort by category then rule ID. Unit conversion happens before comparison; missing unit or conditional clearance yields Unknown. The local single-user API uses the inventory package's SQLite connection and same-origin Web shell.

## User journey and states
Route: `/builds/check`. Actor: builder reviewing a selected parts list. Hierarchy: page title and instructions, primary input/action, row or part detail, status/evidence, recovery action. States: empty selection, editing, checking, complete, incompatible, unknown, stale after edit, server error. Preserve form data on validation/network errors; focus the first error, announce asynchronous completion, restore focus after dialogs. At narrow widths stack controls and tables into labelled cards; keyboard use must not require pointer gestures. Use semantic headings, labels, status text, visible focus, token-backed measured contrast and reduced-motion-safe transitions.

## Invariants and failure policy
Conflict overrides unknown; unknown overrides compatible. Required coverage includes socket/BIOS, RAM, GPU and cooler clearance, board/case, PSU wattage/connectors and storage interface. iGPU or bundled cooling counts only with sourced evidence. PSU estimate is screening, never proof. Empty input produces a helpful empty state. Malformed input returns field-coded 400; missing referenced resources return 404; conflicts return structured 409/422 as specified above. Never silently drop rows, constraints or evidence. Persist only successful state transitions. Repeated reads and identical requests are deterministic; no background side effect or stock reservation is authorized.

## Verification oracle
table-driven Rust fixtures for each conflict and missing evidence; API contract tests; Playwright selection, conflict and unknown flows with keyboard and mobile. Run format/lint, `cargo test`, UI tests/build, strict OpenSpec validation and Forge full Gate. Record browser and accessibility evidence under `docs/assets/`; capture real UI states with source revision and privacy review. Gate success alone does not prove hardware physical fit.

## Decision ledger
Local ownership and interface boundary are fixed here. Third-party catalogs and external project code are excluded until license and adapter review. Resolve implementation variants within these contracts; changing the contract or adding another lifecycle requires a separate change.

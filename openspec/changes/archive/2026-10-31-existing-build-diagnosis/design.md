# Design: Diagnose an existing or proposed PC build

## Boundary and runtime
Implement in RigCraft only. Rust edition 2021 and Cargo own domain, API and SQLite access; add Axum and SQLite dependencies when implementing the first HTTP slice. React/TypeScript with Vite owns `apps/web`; use the pinned shared tokens. Rust API and Web app are separate build targets, served together in local development. Do not import sibling source at runtime. `cargo test` and the Web build/test scripts must be bound to the local Gate when added.

## Ownership and contracts
Rust PartMatch and Diagnosis types; the selected snapshot is used only for replacement candidates; described parts stay separate with zero incremental price. `POST /api/diagnoses/parse` accepts bounded pasted text and returns candidate SKU matches, ambiguity and unmatched lines; `POST /api/diagnoses` accepts confirmed parts and persists the compatibility report, the three advice groups and the validated plan; `GET /api/diagnoses/{id}` re-reads it with staleness. No guessed match is auto-confirmed. No licensed catalog exists in this package, so matching reads the selected snapshot only and a manual structured entry path always remains.

Match by normalised model tokens against the selected snapshot. Return at most five candidates per line, with exact-token matches before fuzzy matches; a line is `matched` only when exactly one candidate has the same token set, and the user confirms it by naming the SKU. Ambiguous and unmatched lines name the tokens or SKU that would resolve them and offer no default. Manual structured entry remains available and missing fields stay Unknown. Diagnosis reuses the compatibility report schema and rule version, and creates no inventory reservation or purchase order.

## User journey and states
Route: `/diagnose`. Actor: owner of a PC or pasted parts list. Hierarchy: page title and instructions, primary input/action, row or part detail, status/evidence, recovery action. States: empty, paste/manual entry, match review, checking, results, ambiguity, no replacement, error. Preserve form data on validation/network errors; focus the first error, announce asynchronous completion, restore focus after dialogs. At narrow widths stack controls and tables into labelled cards; keyboard use must not require pointer gestures. Use semantic headings, labels, status text, visible focus, token-backed measured contrast and reduced-motion-safe transitions.

## Invariants and failure policy
Never infer exact SKU or BIOS revision from partial labels. Keep part only when it satisfies requested constraints; unknown support remains a verification task. Replacement cost excludes retained owned parts. Empty input produces a helpful empty state. Malformed input returns field-coded 400; missing referenced resources return 404; conflicts return structured 409/422 as specified above. Never silently drop rows, constraints or evidence. Persist only successful state transitions. Repeated reads and identical requests are deterministic; no background side effect or stock reservation is authorized.

## Verification oracle
fixtures for ambiguous GPU names, missing BIOS, an absent replacement and a composed plan the checker rejects; the browser journey covers paste, disambiguate, diagnose, validate and mobile review. The checked-in suite is Rust API tests plus Vitest; the browser journey is run through Chrome DevTools MCP and recorded as such. Run format/lint, `cargo test`, UI tests/build, strict OpenSpec validation and Forge full Gate. Record browser and accessibility evidence under `docs/assets/`; capture real UI states with source revision and privacy review. Gate success alone does not prove hardware physical fit.

## Decision ledger
Local ownership and interface boundary are fixed here. Third-party catalogs and external project code are excluded until license and adapter review. Resolve implementation variants within these contracts; changing the contract or adding another lifecycle requires a separate change.

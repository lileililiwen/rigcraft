# Design: Recommend and compare inventory builds

## Boundary and runtime
Implement in RigCraft only. Rust edition 2021 and Cargo own domain, API and SQLite access; add Axum and SQLite dependencies when implementing the first HTTP slice. React/TypeScript with Vite owns `apps/web`; use the pinned shared tokens. Rust API and Web app are separate build targets, served together in local development. Do not import sibling source at runtime. `cargo test` and the Web build/test scripts must be bound to the local Gate when added.

## Ownership and contracts
Rust request, candidate and scoring types in src/; persist request and result references to immutable inventory snapshot and rule/scoring version; no stock mutation. POST /api/recommendations accepts budgetMinor, currency, useCase, inventoryOnly, locked/excluded SKUs and preferences; returns ranked candidates or reason codes. GET /api/recommendations/{id}/export returns versioned JSON. Reject currency mismatch and nonpositive budget with 400.

P0 use cases are `gaming`, `office`, and `creation`; published fixed weights score available sourced CPU/GPU proxy values, price and upgrade options. A missing proxy removes only that score dimension and lowers score confidence; it never becomes zero performance. Enumerate category candidates after hard filters, prune over-budget and over-stock partial builds, and stop after a deterministic bounded search of 100,000 complete combinations; if the cap is reached, return `search_limited` and do not claim global optimality. Store the scoring version and tie-break keys in the response. Export uses the same JSON report schema as the saved result and never recomputes current prices.

## User journey and states
Route: `/recommend`. Actor: buyer or shop operator. Hierarchy: page title and instructions, primary input/action, row or part detail, status/evidence, recovery action. States: form empty, editing, generating, results, unknown warning, no solution, stale snapshot, export success/error. Preserve form data on validation/network errors; focus the first error, announce asynchronous completion, restore focus after dialogs. At narrow widths stack controls and tables into labelled cards; keyboard use must not require pointer gestures. Use semantic headings, labels, status text, visible focus, token-backed measured contrast and reduced-motion-safe transitions.

## Invariants and failure policy
Hard constraints apply before scoring. Candidate count is 0–3 and distinct; incompatible or incomplete builds are excluded, unknown builds labelled. Deterministic ordering uses score then total price then SKU list. Owned parts have explicit zero incremental price and are excluded from stock consumption. Empty input produces a helpful empty state. Malformed input returns field-coded 400; missing referenced resources return 404; conflicts return structured 409/422 as specified above. Never silently drop rows, constraints or evidence. Persist only successful state transitions. Repeated reads and identical requests are deterministic; no background side effect or stock reservation is authorized.

## Verification oracle
fixed inventory fixtures exercising budget boundary, stock exhaustion, locks, missing prices, no solution and repeated identical results; Playwright full no-AI flow, comparison and export at desktop/mobile. Run format/lint, `cargo test`, UI tests/build, strict OpenSpec validation and Forge full Gate. Record browser and accessibility evidence under `docs/assets/`; capture real UI states with source revision and privacy review. Gate success alone does not prove hardware physical fit.

## Decision ledger
Local ownership and interface boundary are fixed here. Third-party catalogs and external project code are excluded until license and adapter review. Resolve implementation variants within these contracts; changing the contract or adding another lifecycle requires a separate change.

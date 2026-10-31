# Design: Import and correct owned inventory

## Boundary and runtime
Implement in RigCraft only. Rust edition 2021 and Cargo own domain, API and SQLite access; add Axum and SQLite dependencies when implementing the first HTTP slice. React/TypeScript with Vite owns `apps/web`; use the pinned shared tokens. Rust API and Web app are separate build targets, served together in local development. Do not import sibling source at runtime. `cargo test` and the Web build/test scripts must be bound to the local Gate when added.

## Ownership and contracts
SQLite database in the Rust Axum process; immutable snapshot ID, source row, normalized fields, provenance and import timestamp; money stored as integer minor units with one batch currency. POST /api/imports previews a UTF-8 CSV and returns rows plus field/error codes; POST /api/imports/{id}/commit accepts explicit row decisions and returns snapshotId; GET /api/inventory/snapshots/{id} returns the immutable snapshot. Reject files above 10 MiB and non-UTF-8 input with 400; no partial commit.

Canonical record fields: `sku`, `category` (cpu, motherboard, ram, gpu, psu, case, cooler, storage), `modelRaw`, `modelNormalized`, `quantity` (nonnegative integer), `priceMinor` (nonnegative integer), `currency` (ISO 4217), `specs` (typed fields with units), and per-field `provenance` (`user` or licensed source, reference, observedAt). `snapshotId` is a UUID. Preview IDs expire after 24 hours; commit is idempotent for the same preview and decisions. A different decision set returns 409. A user correction creates a new snapshot; it never edits an old one.

The first runtime is a self-hosted, single-user local service bound to loopback by default. No shared account or team access is introduced. Inventory is stored in a repository-external local data directory selected by `RIGCRAFT_DATA_DIR`; the default is an application data directory, not a workspace sibling. The browser uses the same-origin API. CSV content and optional cost fields are excluded from logs.

## User journey and states
Route: `/inventory`. Actor: shop operator or individual owner. Hierarchy: page title and instructions, primary input/action, row or part detail, status/evidence, recovery action. States: empty, selecting, parsing, review, row correction, validation error, commit pending, committed, retry. Preserve form data on validation/network errors; focus the first error, announce asynchronous completion, restore focus after dialogs. At narrow widths stack controls and tables into labelled cards; keyboard use must not require pointer gestures. Use semantic headings, labels, status text, visible focus, token-backed measured contrast and reduced-motion-safe transitions.

## Invariants and failure policy
Duplicate SKU requires an explicit merge/replace/keep-separate decision. Missing required identity, quantity, currency or price prevents that row from commit. Unknown hardware specifications remain unknown with a source tag. Empty input produces a helpful empty state. Malformed input returns field-coded 400; missing referenced resources return 404; conflicts return structured 409/422 as specified above. Never silently drop rows, constraints or evidence. Persist only successful state transitions. Repeated reads and identical requests are deterministic; no background side effect or stock reservation is authorized.

## Verification oracle
CSV fixture with valid, malformed, duplicate, missing-price and mixed-currency rows; Rust unit/integration tests and Playwright import/correction journey at desktop and mobile widths. Run format/lint, `cargo test`, UI tests/build, strict OpenSpec validation and Forge full Gate. Record browser and accessibility evidence under `docs/assets/`; capture real UI states with source revision and privacy review. Gate success alone does not prove hardware physical fit.

## Decision ledger
Local ownership and interface boundary are fixed here. Third-party catalogs and external project code are excluded until license and adapter review. Resolve implementation variants within these contracts; changing the contract or adding another lifecycle requires a separate change.

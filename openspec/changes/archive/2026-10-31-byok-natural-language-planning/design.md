# Design: Parse and revise build requests with BYOK

## Boundary and runtime
Implement in RigCraft only. Rust edition 2021 and Cargo own domain, API and SQLite access; add Axum and SQLite dependencies when implementing the first HTTP slice. React/TypeScript with Vite owns `apps/web`; use the pinned shared tokens. Rust API and Web app are separate build targets, served together in local development. Do not import sibling source at runtime. `cargo test` and the Web build/test scripts must be bound to the local Gate when added.

## Ownership and contracts
Rust provider client behind a local trait; browser keeps API key in memory only and sends it per request to the same-origin local API. Backend never persists key, prompt or response; request/result versions remain owned by recommendation package. POST /api/ai/parse accepts key, baseUrl, model and text; returns proposed structured fields plus ambiguities; POST /api/ai/revise accepts current request and instruction and returns a proposed diff. The user confirms before normal recommendation API runs. Provider timeout 20 s, no automatic retry. Provider URL must use HTTPS, except explicit loopback HTTP for local models; reject other private-address targets and redirects to them. Limit prompt size to 8 KiB and never send the full inventory unless the user explicitly opts in to a visible field summary.

## User journey and states
Route: `/recommend assistant panel`. Actor: user choosing natural-language assistance. Hierarchy: page title and instructions, primary input/action, row or part detail, status/evidence, recovery action. States: disabled, consent, key entry, parsing, proposed fields, ambiguity, provider error, confirmed. Preserve form data on validation/network errors; focus the first error, announce asynchronous completion, restore focus after dialogs. At narrow widths stack controls and tables into labelled cards; keyboard use must not require pointer gestures. Use semantic headings, labels, status text, visible focus, token-backed measured contrast and reduced-motion-safe transitions.

## Invariants and failure policy
AI cannot write compatibility or price fields. Missing budget/currency/locked parts require explicit confirmation. A revision changes only fields in the confirmed diff. No key means standard form remains fully usable. Empty input produces a helpful empty state. Malformed input returns field-coded 400; missing referenced resources return 404; conflicts return structured 409/422 as specified above. Never silently drop rows, constraints or evidence. Persist only successful state transitions. Repeated reads and identical requests are deterministic; no background side effect or stock reservation is authorized.

## Verification oracle
mock OpenAI-compatible server for valid, malformed, timeout and data-leak cases; Playwright consent, ambiguity, local revision and no-key flow. Run format/lint, `cargo test`, UI tests/build, strict OpenSpec validation and Forge full Gate. Record browser and accessibility evidence under `docs/assets/`; capture real UI states with source revision and privacy review. Gate success alone does not prove hardware physical fit.

## Decision ledger
Local ownership and interface boundary are fixed here. Third-party catalogs and external project code are excluded until license and adapter review. Resolve implementation variants within these contracts; changing the contract or adding another lifecycle requires a separate change.

## Resolved implementation decisions

- **Secret storage: browser memory only.** No OS keychain: it would add a platform
  dependency and a second copy of the secret without changing the threat model for a
  loopback service. The key lives in component state for the tab, is cleared by a reload,
  and is never written to `localStorage`, `sessionStorage`, cookies or IndexedDB. The
  server holds it in `SecretKey`, whose `Debug` is `[redacted]` and whose value is read in
  one place; the inbound request types do not derive `Serialize`.
- **Persistence: none for the AI layer.** No table, column or row is added and
  `PRAGMA user_version` stays `3`. Only the confirmed request reaches the existing
  recommendation contract, which persists its own report. This keeps the prompt and the
  model reply out of the database by construction.
- **Transport:** one blocking adapter (`ureq`, rustls) behind `ProviderAdapter`, called
  from `spawn_blocking`. Global 20-second timeout, `max_redirects = 0`, no retry. HTTPS is
  required except for loopback HTTP; credentials, query strings, fragments and
  non-loopback private addresses are refused at validation time.
- **Grounding is enforced server-side, not requested from the model.** Every `sentence`
  field must carry an excerpt that is a literal substring of the normalised sentence; a
  SKU must exist in the snapshot; model output outside the six allowed fields is discarded
  and listed. A `default` source is limited to inventory-only mode and "the sentence named
  no part".
- **Evidence uses a loopback stub, not a mock switch in the binary.** The production
  router always installs the real adapter; tests inject `MockProvider` through
  `AppState::with_provider`, and the browser evidence drives the real adapter against
  `scripts/mock-openai-provider.py`. There is no environment variable that replaces the
  provider at runtime.

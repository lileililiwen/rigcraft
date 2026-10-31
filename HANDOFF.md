# Project Handoff

## Current state

current_spec: none

`requirement.md` is the product baseline. Inventory import and provenance is implemented, archived and tested: `/inventory` previews UTF-8 CSV, reports field-coded row errors, requires explicit duplicate decisions, commits immutable SQLite snapshots with per-field provenance, and creates new revisions for corrections. Compatibility checking, recommendation, BYOK planning and build diagnosis remain unimplemented packages. Deployment remains disabled.

## Next action

Select `evidence-backed-compatibility` only when implementation is requested. Set `current_spec: evidence-backed-compatibility` before work, complete and archive that one package, then stop. The dependency map remains in `ROADMAP.md`.

## Inventory implementation evidence (2026-10-31, commit `ec0f123`)

- `cargo fmt --all -- --check` — pass. `cargo clippy --all-targets --all-features -- -D warnings` — pass.
- `cargo test` — pass: 26 unit tests and 10 HTTP contract tests (CSV errors, duplicate merge/replace/keep-separate, idempotent commit, conflict, expiry, atomic failure, correction immutability, snapshot replay after expiry).
- `npm run check` — pass (measured token contrast + TypeScript). `npm test` — pass: 11 Vitest tests. `npm run build` — pass.
- `sh verify-local.sh` — pass (Cargo fmt/clippy/test, `npm ci`, check, typecheck, test).
- `node scripts/check-openspec-change-names.mjs` — pass. `openspec validate --all --strict` — 5 passed, 0 failed. Archived with `openspec archive inventory-import-and-provenance --yes`; `openspec/specs/inventory-import/spec.md` exists and its Purpose is written.
- Full Gate `/home/paul/code/forge/target/debug/forge gate . --profile full` — aggregate passed, 9 checks (build, security, governance-quality, placeholder, product boundary, repository, source size, tests, UI conformance). The Gate labels the working-tree base revision `f6ebf7d1daa0`; its persisted evidence is `.forge/gate/rigcraft/evidence.json` and the implementation is commit `ec0f123`. The pre-existing workspace `DISCOVERED_UNREGISTERED` findings are outside this project's governance scope; the in-scope finding this cycle (archived-spec Purpose marker) was fixed before the passing run.
- Browser evidence (Chrome DevTools MCP, production build, 1440×900 and 390×844): `docs/evidence/inventory-import-browser.md`, captures in `docs/assets/inventory-*.png`. No horizontal overflow at 390 px; skip link and controls show a 3 px focus outline; focus moves to review, snapshot and error headings.
- Accessibility: Lighthouse accessibility 100 with 0 failing audits on desktop and mobile (`docs/evidence/inventory-lighthouse*/report.json`); measured contrast in `docs/evidence/token-contrast.json` (lowest 5.46:1 for error text, 7.29:1 for the focus ring).
- Persistence is SQLite outside the repository (`RIGCRAFT_DATA_DIR`, else XDG/HOME application data), schema `PRAGMA user_version = 1`. Contract and error codes: `docs/inventory-import.md`. Captures are synthetic; CSV content and cost fields were absent from service logs.
- Not done: a checked-in Playwright suite. The browser journey and responsive/a11y checks were run through Chrome DevTools MCP, with Vitest covering component and state behaviour; this limitation is recorded in the evidence document.

## Bootstrap evidence (2026-10-31)

- Forge rendered `rust-web@0.1.0` and `baseline-service@1.6.0`; `cargo test` passed (one scaffold test).
- `node scripts/check-openspec-change-names.mjs` passed; strict OpenSpec validation found no items because no changes were requested.
- Shared UI token pin is SHA-256 `7ca6b7956b3f6b475254b4a5d0cf599724e4e1af5f0f88aba4fbccb9f38fc19b`; UI conformance passed for the token shell. No browser capture or measured contrast exists because no UI exists.
- GitHub repository `lileililiwen/rigcraft` is public. `gh repo view` confirmed description, homepage and four topics; `git ls-remote --heads origin main` confirmed initial commit `8efd10f54d2000da838946b05f9d848205f545e3`.
- Workspace governance audit reported unrelated `DISCOVERED_UNREGISTERED` projects and provider declaration warnings; inspect its current output before treating Gate as clean.
- Forge fast and full Gate passed against initial revision `8efd10f54d20`; revision-bound evidence is in `.forge/gate/rigcraft/evidence.json`. Full Gate included Cargo build/test, security, governance-quality, UI conformance and repository checks. This is scaffold evidence, not product or browser readiness.
- A prior restricted execution environment printed `Failed to create stream fd: Operation not permitted` and blocked the managed Cargo cache. After the environment permissions changed, `rtk`, `rtk bypass` and `cargo test` ran normally; no project or RTK code change was needed.

Each completed OpenSpec spec/change requires exactly two commits: first the implementation/tests/archive commit, then a HANDOFF.md-only pointer/evidence commit.

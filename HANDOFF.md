# Project Handoff

## Current state

`requirement.md` is the product baseline. Forge `rust-web@0.1.0` scaffold and `baseline-service@1.6.0` pack are present. Governance and MIT license are initialized. Product features and UI are not implemented. There are no active OpenSpec changes by request.

## Next action

When requested, select the first product change around licensed data and the inventory import/correction journey. Add `current_spec: <name>` only after a change is active. Deployment remains disabled.

## Bootstrap evidence (2026-10-31)

- Forge rendered `rust-web@0.1.0` and `baseline-service@1.6.0`; `cargo test` passed (one scaffold test).
- `node scripts/check-openspec-change-names.mjs` passed; strict OpenSpec validation found no items because no changes were requested.
- Shared UI token pin is SHA-256 `7ca6b7956b3f6b475254b4a5d0cf599724e4e1af5f0f88aba4fbccb9f38fc19b`; UI conformance passed for the token shell. No browser capture or measured contrast exists because no UI exists.
- GitHub repository `lileililiwen/rigcraft` is public. `gh repo view` confirmed description, homepage and four topics; `git ls-remote --heads origin main` confirmed initial commit `8efd10f54d2000da838946b05f9d848205f545e3`.
- Workspace governance audit reported unrelated `DISCOVERED_UNREGISTERED` projects and provider declaration warnings; inspect its current output before treating Gate as clean.
- Forge fast and full Gate passed against initial revision `8efd10f54d20`; revision-bound evidence is in `.forge/gate/rigcraft/evidence.json`. Full Gate included Cargo build/test, security, governance-quality, UI conformance and repository checks. This is scaffold evidence, not product or browser readiness.
- A prior restricted execution environment printed `Failed to create stream fd: Operation not permitted` and blocked the managed Cargo cache. After the environment permissions changed, `rtk`, `rtk bypass` and `cargo test` ran normally; no project or RTK code change was needed.

Each completed OpenSpec spec/change requires exactly two commits: first the implementation/tests/archive commit, then a HANDOFF.md-only pointer/evidence commit.

# RigCraft

Open-source PC build planning from owned inventory, with traceable compatibility checks. See [requirements](requirement.md), [roadmap](ROADMAP.md) and the [inventory import contract](docs/inventory-import.md).

## Status

Inventory import and correction is implemented: `/inventory` previews a UTF-8 CSV, reports every row and field error, requires explicit duplicate decisions, commits an immutable SQLite snapshot with provenance, and creates a new revision for later corrections. Compatibility checking, build recommendation, BYOK planning and build diagnosis remain planned packages; deployment is disabled.

## Runtime

Rust service (Axum + SQLite) and React/TypeScript Web app (Vite).

```sh
cargo run                      # serves the API and apps/web/dist on 127.0.0.1:8080
npm install
npm run build                  # builds the Web app
./verify-local.sh              # format, clippy, Cargo tests, Web checks and tests
```

Individual checks:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
npm run check                  # measured token contrast + TypeScript
npm test
npm run build
```

Open `http://127.0.0.1:8080/inventory` after building the Web app.

## Data and privacy

Inventory is stored in SQLite outside the repository: `RIGCRAFT_DATA_DIR`, otherwise `$XDG_DATA_HOME/rigcraft` or `$HOME/.local/share/rigcraft`. One currency is used per import batch and money is stored in integer minor units. CSV contents and optional cost fields are not logged. The service binds to loopback by default.

## Routes

- `GET /inventory` — import, review, correct, commit and revision history.
- `POST /api/imports` — CSV preview.
- `POST /api/imports/{previewId}/commit` — atomic snapshot commit.
- `GET /api/inventory/snapshots` and `GET /api/inventory/snapshots/{id}` — history and immutable snapshots.
- `POST /api/inventory/snapshots/{id}/corrections` — new revision.
- `GET /api/imports/sample` — sample CSV.

Contract shapes, error codes and provenance rules: [docs/inventory-import.md](docs/inventory-import.md).

## Shared layer

Kit `none` (no version) on the `cargo` ecosystem. The UI consumes the pinned vendored token stylesheet; `.project.json` records its SHA-256.

Declared minimum consumption: 0 (a declared zero, not a met floor). Reason: no crate in rust-platform-libs has two external consumers (1 consumer: trailCrew); the 8 crates stay commented-out Cargo.toml entries.

Referencing a kit package never grants the capability it implements.

## Evidence

- `docs/evidence/inventory-import-browser.md` — browser, keyboard, responsive and privacy evidence.
- `docs/evidence/token-contrast.json` — measured WCAG 2.2 AA contrast.
- `docs/evidence/inventory-lighthouse*/` — Lighthouse reports.
- `docs/assets/` — captured UI states.

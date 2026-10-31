# RigCraft

Open-source PC build planning from owned inventory, with traceable compatibility checks. See [requirements](requirement.md), [roadmap](ROADMAP.md), the [inventory import contract](docs/inventory-import.md), the [compatibility contract](docs/compatibility-evidence.md), the [recommendation contract](docs/build-recommendation.md) and the [BYOK planning contract](docs/ai-planning.md).

## Status

Inventory import and correction is implemented: `/inventory` previews a UTF-8 CSV, reports every row and field error, requires explicit duplicate decisions, commits an immutable SQLite snapshot with provenance, and creates a new revision for later corrections. `/builds/check` checks a selected build against one snapshot and reports compatible, incompatible or unknown for every rule, with the compared values, provenance and a specific follow-up for each unknown. `/recommend` searches only the selected snapshot for up to three complete, budget- and stock-valid builds, labels each with its reason, score and excluded alternatives, explains the binding budget, stock or compatibility constraint when there is none, and exports versioned JSON or a Markdown plan. Its optional assistant panel uses the operator's own provider key to turn one sentence into a proposed request with a source for every field; nothing runs until the operator confirms, the deterministic engine still re-validates every constraint, and the page is fully usable without a key. Build diagnosis remains a planned package; deployment is disabled.

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

Open `http://127.0.0.1:8080/inventory`, `http://127.0.0.1:8080/builds/check` or
`http://127.0.0.1:8080/recommend` after building the Web app.

## Data and privacy

Inventory is stored in SQLite outside the repository: `RIGCRAFT_DATA_DIR`, otherwise `$XDG_DATA_HOME/rigcraft` or `$HOME/.local/share/rigcraft`. One currency is used per import batch and money is stored in integer minor units. CSV contents and optional cost fields are not logged. The service binds to loopback by default.

A provider key is optional and belongs to the operator. It is held in browser memory for the session only, is never stored, logged, exported or echoed, and is sent only to the provider URL the operator enters. The assistant sends the sentence, the snapshot currency, the allowed use-case names and — only on explicit opt-in — a bounded part list; the inventory is never sent by default. No key, provider or catalog is configured or bundled.

## Routes

- `GET /inventory` — import, review, correct, commit and revision history.
- `POST /api/imports` — CSV preview.
- `POST /api/imports/{previewId}/commit` — atomic snapshot commit.
- `GET /api/inventory/snapshots` and `GET /api/inventory/snapshots/{id}` — history and immutable snapshots.
- `POST /api/inventory/snapshots/{id}/corrections` — new revision.
- `GET /api/imports/sample` — sample CSV.
- `GET /builds/check` — select parts from a snapshot and read sourced verdicts.
- `POST /api/builds/check` — run and persist a compatibility report.
- `GET /api/builds/checks/{id}` — read a stored report and its staleness.
- `GET /recommend` — enter a budget, lock or exclude parts, and compare recommended builds.
- `POST /api/recommendations` — search, rank and persist a budget- and stock-constrained recommendation.
- `GET /api/recommendations/{id}` — read a stored recommendation and its staleness.
- `GET /api/recommendations/{id}/export?format=json|markdown` — download the versioned artifact.
- `POST /api/ai/parse` — interpret one sentence into a proposed request with a source per field (no search runs).
- `POST /api/ai/revise` — propose a field-level revision of a confirmed request.

Contract shapes, error codes and provenance rules: [docs/inventory-import.md](docs/inventory-import.md), [docs/compatibility-evidence.md](docs/compatibility-evidence.md), [docs/build-recommendation.md](docs/build-recommendation.md) and [docs/ai-planning.md](docs/ai-planning.md).

## Shared layer

Kit `none` (no version) on the `cargo` ecosystem. The UI consumes the pinned vendored token stylesheet; `.project.json` records its SHA-256.

Declared minimum consumption: 0 (a declared zero, not a met floor). Reason: no crate in rust-platform-libs has two external consumers (1 consumer: trailCrew); the 8 crates stay commented-out Cargo.toml entries.

Referencing a kit package never grants the capability it implements.

## Evidence

- `docs/evidence/inventory-import-browser.md` — inventory browser, keyboard, responsive and privacy evidence.
- `docs/evidence/compatibility-browser.md` — compatibility verdicts, conflicts, unknowns, stale state, keyboard, responsive, accessibility and privacy evidence.
- `docs/evidence/recommendation-browser.md` — recommendation, comparison, export, no-solution, keyboard, responsive, accessibility and privacy evidence.
- `docs/evidence/byok-planning-browser.md` — BYOK assistant parse → edit → confirm → deterministic result, no-key degradation, provider error, keyboard, responsive, accessibility and secret-handling evidence.
- `docs/evidence/token-contrast.json` — measured WCAG 2.2 AA contrast.
- `docs/evidence/inventory-lighthouse*/`, `docs/evidence/compatibility-lighthouse*/`, `docs/evidence/recommendation-lighthouse*/` and `docs/evidence/byok-lighthouse*/` — Lighthouse reports.
- `docs/assets/` — captured UI states.

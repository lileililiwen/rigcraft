# RigCraft

Open-source PC build planning from owned inventory, with traceable compatibility checks. See [requirements](requirement.md), [roadmap](ROADMAP.md), the [inventory import contract](docs/inventory-import.md), the [data portability contract](docs/data-portability.md), the [compatibility contract](docs/compatibility-evidence.md), the [recommendation contract](docs/build-recommendation.md), the [BYOK planning contract](docs/ai-planning.md), the [build diagnosis contract](docs/build-diagnosis.md), the [catalog ingestion contract](docs/catalog-ingestion.md) and the [data-source assessment](docs/data-sources.md).

## Status

Inventory import and correction is implemented: `/inventory` opens the newest snapshot on arrival with its revision, records, stock value and latest/earlier state, presents history as revision cards with an explicit view, previews a UTF-8 CSV, reports every row and field error, requires explicit duplicate decisions, commits an immutable SQLite snapshot with provenance, and creates a new revision for later corrections. The same page exports a snapshot as the importer's CSV or the authoritative JSON, downloads the whole catalog as a standard SQLite backup, and restores a backup only after an inspection and a typed confirmation; a pre-restore copy protects the previous catalog. Import, history and export/backup are separate panels, and input-dependent actions stay disabled with a readable reason until their input is valid. `/builds/check` checks a selected build against one snapshot and reports compatible, incompatible or unknown for every rule, with the compared values, provenance and a specific follow-up for each unknown, and presents them in a verdict-console matrix under a summary bar. `/recommend` is a three-region workbench at desktop width — constraints, options and a verdict rail — and one column at phone width: it searches only the selected snapshot for up to three complete, budget- and stock-valid builds, labels each as a specification module with its reason, score and excluded alternatives, explains the binding budget, stock or compatibility constraint when there is none, and exports versioned JSON or a Markdown plan. Its optional assistant panel uses the operator's own provider key to turn one sentence into a proposed request with a source for every field; nothing runs until the operator confirms, the deterministic engine still re-validates every constraint, and the page is fully usable without a key. `/diagnose` takes a build the operator already owns — pasted lines matched to the snapshot, or parts entered by hand — and reports what must be replaced, what should be verified first, what is fine as is and what no rule could evaluate, with the rule evidence and cost of each; an ambiguous or unmatched line is never coerced to a nearby SKU, and a replacement plan is labelled validated only when the same compatibility engine accepts the whole upgraded build; the four groups, evidence table and plan use the same console language. `/catalog` loads a hardware catalog the operator is licensed to use, validates the whole document before storing it, shows the registry as a dense table with its licence, version, counts, active state and source URL, activates or removes a dataset, and searches it with rank and dataset version preserved; the import review uses only active datasets to propose fields the operator's row leaves empty, with `catalog:<dataset>@<version>` provenance and no overwrite. No catalog data is bundled. Deployment remains disabled.

## Runtime

Rust service (Axum + SQLite) and React/TypeScript Web app (Vite).

```sh
cargo run                      # serves the API and apps/web/dist on 127.0.0.1:8080
RIGCRAFT_ADDR=127.0.0.1:8787 cargo run   # use another port when 8080 is taken
npm install
npm run build                  # builds the Web app
./verify-local.sh              # format, clippy, Cargo tests, Web checks and tests, browser suite
```

`RIGCRAFT_ADDR` overrides the loopback bind address; use it when port 8080 is
occupied. `RIGCRAFT_DATA_DIR` moves the SQLite file described below.

Browser regression suite:

```sh
cargo build                    # prerequisite: the service binary
npm run build                  # prerequisite: the bundle the service serves
npm run test:e2e               # real Chrome, 25 browser tests, under a minute
```

`npm run test:e2e` picks a free loopback port, starts `target/debug/rigcraft`
with a fresh data directory under the operating system's temp directory, waits
for `/api/health`, and removes the directory when it finishes. It never uses
port 8080, never reuses your inventory and needs no network; `RIGCRAFT_E2E_PORT`
pins a port only when you need one. The Playwright suite lives in `e2e/` and its
config in `playwright.config.ts`; the JSON report is written to the gitignored
`test-results/`.

Individual checks:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
npm run check                  # measured contrast + converter tests + TypeScript
npm test
npm run build
npm run test:e2e
```

Open `http://127.0.0.1:8080/inventory`, `http://127.0.0.1:8080/catalog`,
`http://127.0.0.1:8080/builds/check`, `http://127.0.0.1:8080/recommend` or
`http://127.0.0.1:8080/diagnose` after building the Web app.

## Data and privacy

Inventory is stored in SQLite outside the repository: `RIGCRAFT_DATA_DIR`, otherwise `$XDG_DATA_HOME/rigcraft` or `$HOME/.local/share/rigcraft`. One currency is used per import batch and money is stored in integer minor units. CSV contents and optional cost fields are not logged. The service binds to loopback by default.

A snapshot export, a backup and an inspection happen only on an explicit request to this local service. The JSON export and a database backup contain the complete private inventory, including purchase costs if stored; treat those files like the database itself. Restore writes a permanent `inventory.sqlite3.pre-restore-<UTC>` safety copy into the data directory first. See [docs/data-portability.md](docs/data-portability.md).

A provider key is optional and belongs to the operator. It is held in browser memory for the session only, is never stored, logged, exported or echoed, and is sent only to the provider URL the operator enters. The assistant sends the sentence, the snapshot currency, the allowed use-case names and — only on explicit opt-in — a bounded part list; the inventory is never sent by default. No key, provider or catalog is configured or bundled.

## Routes

- `GET /inventory` — import, review, correct, commit and revision history.
- `POST /api/imports` — CSV preview.
- `POST /api/imports/{previewId}/commit` — atomic snapshot commit.
- `GET /api/inventory/snapshots` and `GET /api/inventory/snapshots/{id}` — history and immutable snapshots.
- `GET /api/inventory/snapshots/{id}/export?format=csv|json` — importer-column CSV or the authoritative JSON artifact.
- `POST /api/inventory/snapshots/{id}/corrections` — new revision.
- `GET /api/backup` — consistent whole-database copy as a standard SQLite file.
- `POST /api/backup/inspect` — validate an uploaded backup and return its counts without writing.
- `POST /api/backup/restore` — validate, write a pre-restore copy and replace the live catalog with the upload.
- `GET /catalog` — upload and validate an operator-supplied catalog, manage the registry and search it.
- `POST /api/catalog/datasets` — validate and store a `rigcraft-catalog` version 1 document with an acknowledgement.
- `GET /api/catalog/datasets` — registry with licence, version, counts and active flag.
- `PATCH /api/catalog/datasets/{id}` — activate or deactivate a dataset for matching.
- `DELETE /api/catalog/datasets/{id}?confirm=remove` — remove a dataset and its parts only.
- `GET /api/catalog/search?q=&category=&limit=` — deterministic catalog search.
- `GET /api/catalog/parts/{id}` — one stored part with its specifications and provenance.
- `POST /api/catalog/match` — three-state matching and enrichment proposals for import rows.
- `GET /api/imports/sample` — sourced sample CSV (see Licensing).
- `GET /builds/check` — select parts from a snapshot and read sourced verdicts.
- `POST /api/builds/check` — run and persist a compatibility report.
- `GET /api/builds/checks/{id}` — read a stored report and its staleness.
- `GET /recommend` — enter a budget, lock or exclude parts, and compare recommended builds.
- `POST /api/recommendations` — search, rank and persist a budget- and stock-constrained recommendation.
- `GET /api/recommendations/{id}` — read a stored recommendation and its staleness.
- `GET /api/recommendations/{id}/export?format=json|markdown` — download the versioned artifact.
- `POST /api/ai/parse` — interpret one sentence into a proposed request with a source per field (no search runs).
- `POST /api/ai/revise` — propose a field-level revision of a confirmed request.
- `GET /diagnose` — describe an existing build, resolve matches, and read reuse and upgrade advice.
- `POST /api/diagnoses/parse` — parse and match a bounded pasted parts list; stores nothing.
- `POST /api/diagnoses` — diagnose the confirmed build and persist the advice and validated plan.
- `GET /api/diagnoses/{id}` — read a stored diagnosis and its staleness.

Contract shapes, error codes and provenance rules: [docs/inventory-import.md](docs/inventory-import.md), [docs/catalog-ingestion.md](docs/catalog-ingestion.md), [docs/data-portability.md](docs/data-portability.md), [docs/compatibility-evidence.md](docs/compatibility-evidence.md), [docs/build-recommendation.md](docs/build-recommendation.md), [docs/ai-planning.md](docs/ai-planning.md) and [docs/build-diagnosis.md](docs/build-diagnosis.md).

## Licensing

The source code is MIT; see [`LICENSE`](LICENSE).

The shipped sample data is a RigCraft compilation licensed CC BY 4.0, with the
attribution and the list of changes in [`NOTICE`](NOTICE). Its compatibility
facts were checked against the manufacturer pages recorded per row on
2026-10-31; the pages used are listed in [`docs/data-sources.md`](docs/data-sources.md).
Its quantity and price are operator-entered sample values, not market data.

BuildCores OpenDB is **not** distributed with RigCraft. It is a much larger
third-party dataset under CC BY-NC-SA 4.0, so its licence and obligations apply
to whoever fetches it. `scripts/opendb-to-csv.mjs` converts a local checkout into
the import CSV and `scripts/opendb-to-catalog.mjs` converts it into the
operator-uploaded catalog document; both are local steps and neither result is
redistributed. The commands and obligations are in
[`docs/data-sources.md`](docs/data-sources.md). PCPartPicker data is never used.
An operator may also upload a catalog they are licensed to use through `/catalog`;
it is stored only in that instance and is never committed or shared.

## Shared layer

Kit `none` (no version) on the `cargo` ecosystem. The UI consumes the pinned vendored token stylesheet and layers one app-owned dark brand token file (`apps/web/src/styles/brand.css`) on top; `.project.json` records the pin's SHA-256 and `docs/branding.md` records the measured palette.

Declared minimum consumption: 0 (a declared zero, not a met floor). Reason: no crate in rust-platform-libs has two external consumers (1 consumer: trailCrew); the 8 crates stay commented-out Cargo.toml entries.

Referencing a kit package never grants the capability it implements.

## Evidence

- `docs/evidence/inventory-import-browser.md` — inventory browser, keyboard, responsive and privacy evidence.
- `docs/evidence/compatibility-browser.md` — compatibility verdicts, conflicts, unknowns, stale state, keyboard, responsive, accessibility and privacy evidence.
- `docs/evidence/recommendation-browser.md` — recommendation, comparison, export, no-solution, keyboard, responsive, accessibility and privacy evidence.
- `docs/evidence/workbench-recommendation-browser.md` — the three-region workbench, spec modules, verdict rail, grouped budget, collapsed assistant, focus ring, responsive measurements and Lighthouse scores.
- `docs/evidence/byok-planning-browser.md` — BYOK assistant parse → edit → confirm → deterministic result, no-key degradation, provider error, keyboard, responsive, accessibility and secret-handling evidence.
- `docs/evidence/diagnosis-browser.md` — existing-build matching (ambiguity, miss, duplicate), reuse advice groups, validated/needs-evidence/rejected plan states, keyboard, responsive, accessibility and privacy evidence.
- `docs/evidence/correctness-audit-browser.md` — corrected unverified advice group, stable `/diagnose` numbering, ISO 8601 UTC timestamps and static/metadata 404 behaviour.
- `docs/evidence/catalog-seed-browser.md` — corrected sourced sample download, a catalog-column import and the operator-versus-catalog distinction.
- `docs/evidence/browser-suite.md` — the checked-in Playwright suite: command, the twenty tests and what each holds, and its explicit non-coverage.
- `docs/evidence/data-portability-browser.md` — snapshot export, backup download, inspected restore, the typed confirmation guard and a rejected restore.
- `docs/evidence/catalog-ingestion-browser.md` — catalog upload and refusal, the registry with a licence, search and a catalog-enriched import with committed provenance.
- `docs/evidence/verdict-console-browser.md` — the rule matrix, single coverage sentence, grammatical gap sentences, exact-match token noise and the verdict-console Lighthouse numbers.
- `docs/evidence/ui-foundation-browser.md` — dark token layer, shell-chrome fixes, focus ring, caption width and icon evidence, with the Lighthouse numbers.
- `docs/evidence/token-contrast.json` — measured WCAG 2.2 AA contrast for the effective dark theme (40 pairs, both token source hashes).
- `docs/evidence/inventory-lighthouse*/`, `docs/evidence/compatibility-lighthouse*/`, `docs/evidence/recommendation-lighthouse*/`, `docs/evidence/workbench-lighthouse*/`, `docs/evidence/byok-lighthouse*/`, `docs/evidence/diagnosis-lighthouse*/`, `docs/evidence/correctness-diagnose-lighthouse*/`, `docs/evidence/catalog-lighthouse*/`, `docs/evidence/data-portability-lighthouse*/` and `docs/evidence/catalog-ingestion-lighthouse*/` and `docs/evidence/verdict-lighthouse/` — Lighthouse reports.
- `docs/assets/` — captured UI states.

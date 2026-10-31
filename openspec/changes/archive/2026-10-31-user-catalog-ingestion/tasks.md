# Tasks: Operator-supplied catalog ingestion and enrichment

## 1. BFS — Baseline and impact coverage
- [x] 1.1 Read `AGENTS.md`, `.ai-rules/`, `HANDOFF.md`, `ROADMAP.md`, `docs/data-sources.md`, `docs/inventory-import.md`, `docs/data-portability.md`, `src/store.rs`, `src/csv_import.rs` and `scripts/opendb-to-csv.mjs`; confirm no other active change and set `current_spec`.
- [x] 1.2 Map impact across the store and migration, routes, error envelope, commit contract, `/catalog`, `/inventory` review, Rust tests, Vitest, Playwright, converter and docs.
- [x] 1.3 Assess shared infrastructure by name (`serde_json`/`uuid`, `rusqlite`, the existing normalizer, Forge Rust libraries, pinned tokens) and record that no dependency is added.

## 2. DFS — Document and validation
- [x] 2.1 Define the `rigcraft-catalog` version 1 document and the upload request with its acknowledgement.
- [x] 2.2 Validate format, version, dataset header and every part; report row index, field, reason and the accepted categories and specification keys.
- [x] 2.3 Refuse duplicates, unknown spec keys and unparseable spec values as a whole document; store nothing on refusal.

## 3. DFS — Storage and registry
- [x] 3.1 Add the schema 5 forward migration with the dataset registry and dataset-scoped parts, preserving schema 4 data.
- [x] 3.2 Implement atomic import/replace by dataset name, coexistence of other datasets, activation, confirmation-guarded delete and part lookup.
- [x] 3.3 Keep inventory, snapshots, checks, recommendations and diagnoses outside the catalog's delete and replace paths.

## 4. DFS — Search, matching and enrichment
- [x] 4.1 Implement deterministic search ranking (exact SKU/external ID, model prefix, token overlap) with paging and dataset attribution.
- [x] 4.2 Implement three-state matching (exact SKU, MPN, normalized model in category) over active datasets only.
- [x] 4.3 Implement proposals for missing fields and conflicts for differing values, with no default on ambiguity.
- [x] 4.4 Extend the commit contract with the per-row opt-in and apply only proposals with `catalog:<name>@<version>` provenance and the part's source URL.

## 5. DFS — Interface
- [x] 5.1 Add catalog API helpers and types; add the `/catalog` route and header entry.
- [x] 5.2 Build the upload, validation report, registry, activation, confirmation-guarded removal, search and part detail states with accessible focus and announcements.
- [x] 5.3 Show each preview row's catalog status on `/inventory`, the per-row opt-in, conflicts and the added fields labelled catalog-sourced.
- [x] 5.4 Add Vitest coverage for the catalog page and the import opt-in.

## 6. DFS — Converter and docs
- [x] 6.1 Extract the shared OpenDB field mapping and add `scripts/opendb-to-catalog.mjs`, with tests, as an operator-run companion.
- [x] 6.2 Write `docs/catalog-ingestion.md`; update README routes/status, `docs/inventory-import.md`, `docs/data-sources.md`, the capture plan and any stale schema claim.

## 7. Verification and evidence
- [x] 7.1 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test`, `npm run check`, `npm test`, `npm run build`, `npm run test:e2e`, `./verify-local.sh`.
- [x] 7.2 Run `node scripts/check-openspec-change-names.mjs`, `openspec validate user-catalog-ingestion --strict` and `openspec validate --all --strict`.
- [x] 7.3 Extend the browser suite with a catalog upload/registry/search journey, a refused document and an enriched import whose snapshot carries catalog provenance.
- [x] 7.4 Capture browser evidence with Chrome DevTools MCP at 1440×900 and 390×844; write `docs/evidence/catalog-ingestion-browser.md`; re-run Lighthouse on `/catalog`.
- [x] 7.5 Run the Forge full Gate and record its aggregate verdict.
- [x] 7.6 Archive without `--skip-specs`; commit everything except `HANDOFF.md`, then commit `HANDOFF.md` only; do not push.

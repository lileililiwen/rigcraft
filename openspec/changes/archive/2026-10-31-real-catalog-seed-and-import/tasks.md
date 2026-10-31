# Tasks: A sourced catalog seed and OpenDB import

## 1. BFS — Baseline and impact coverage
- [x] 1.1 Read `AGENTS.md`, `.ai-rules/`, `HANDOFF.md`, `ROADMAP.md`, `requirement.md`, `docs/inventory-import.md` and `docs/compatibility-evidence.md`; confirm no other active change and set `current_spec`.
- [x] 1.2 Map the defect across the sample constant, CSV mapping, `RowValues`, `evaluate_row`, commit overrides, snapshot provenance, the preview/snapshot UI and the docs; identify every assertion that quotes `CPU-5800X`.
- [x] 1.3 Assess shared infrastructure by name (Forge Rust libraries, pinned tokens) and record that only the token pair is reused and no new dependency is needed.

## 2. DFS — Seed and provenance
- [x] 2.1 Add `source` and `sourceUrl` to `RowValues`, CSV aliases and mapping; validate both; keep empty columns behaviour-identical.
- [x] 2.2 Apply catalog provenance to `model` and each spec; keep `sku`, `quantity`, `price` and `currency` on `user`; replace the edited field's catalog claim when the operator overrides model or specs.
- [x] 2.3 Set `match.confidence` to `catalog-declared` with a naming basis when a source is declared; keep `unverified` otherwise.
- [x] 2.4 Replace the sample with ten verified rows and per-row manufacturer URLs; check every value against the cited page on 2026-10-31 and omit anything the page does not state.
- [x] 2.5 Add Rust unit/HTTP tests for parsing, invalid source values, provenance routing, override clearing, backward compatibility and the sample's conflict-free compatibility verdict.
- [x] 2.6 Update stale test fixtures and assertions that name `CPU-5800X` or count eight sample rows.

## 3. DFS — Converter, documentation and UI
- [x] 3.1 Add `scripts/opendb-to-csv.mjs` with the eight-category mapping, commit SHA, `manufacturer_url`, operator price/quantity options, and loud failure on unmapped category, missing name/SKU, unreadable JSON or missing options, and explicit warnings for shared SKUs, over-length values and unusable URLs.
- [x] 3.2 Add a `node --test` suite for the converter and wire it into `npm run check`; verify unknown-never-guessed omissions.
- [x] 3.3 Write `docs/data-sources.md` (assessment table), `NOTICE` (CC BY 4.0 attribution), README Licensing section, and update `docs/inventory-import.md`, `docs/compatibility-evidence.md`, `AGENTS.md` layout, `ROADMAP.md` and the evidence index.
- [x] 3.4 Show operator-versus-catalog facts in the preview and snapshot, derive the spec badge from provenance, state the sample licence and date in the download area, and add the measured contrast pair.
- [x] 3.5 Add or update Vitest coverage for the badges, row labels and download-area text; keep all existing states and announcements.

## 4. BFS — Cross-surface regression
- [x] 4.1 Recheck callers, stored data, error codes, merge equality, correction provenance, privacy, responsive layout and accessibility.
- [x] 4.2 Recheck README claims against the tree; confirm no dataset, secret, lockfile or generated directory was added.

## 5. Verification and evidence
- [x] 5.1 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test`, `npm run check`, `npm test`, `npm run build`, `./verify-local.sh`.
- [x] 5.2 Run `node scripts/check-openspec-change-names.mjs`, `openspec validate real-catalog-seed-and-import --strict` and `openspec validate --all --strict`.
- [x] 5.3 Capture the corrected sample download, a real import with the new columns, and the operator/catalog distinction at 1440×900 and 390×844; write `docs/evidence/catalog-seed-browser.md`; run Lighthouse on `/inventory`.
- [x] 5.4 Run the Forge full Gate and record its aggregate verdict.
- [x] 5.5 Archive without `--skip-specs`; commit everything except `HANDOFF.md`, then commit `HANDOFF.md` only; do not push.

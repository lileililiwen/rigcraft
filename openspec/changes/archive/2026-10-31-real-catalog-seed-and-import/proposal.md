# Proposal: Ship a sourced catalog seed and an OpenDB import path

## Why

`GET /api/imports/sample` ships an impossible build. `CPU-5800X` is an AMD Ryzen 7
5800X, an AM4 part, but the row declares `socket=AM5` and is paired with an AM5
B650E motherboard. Every screenshot in `docs/evidence` inherits the claim. A product
whose promise is traceable compatibility cannot ship a fabricated example: the sample
is the first thing a new operator imports, and it teaches the opposite of the rule
that an absent fact must stay unknown.

The sample also has no provenance. Its values cite `user` with the CSV filename, so a
reader cannot tell which facts came from a manufacturer, which came from the operator,
and which were invented. There is no supported way to bring in a real catalog without
hand-editing and guessing, and no recorded licence position for the data.

## What Changes

- Replace the sample with ten rows for one physically buildable AM5 machine, each
  compatibility fact checked against the manufacturer's own product page on
  2026-10-31 and carrying that page's URL. A fact that cannot be checked is omitted,
  not filled in.
- Add optional `source` and `sourceUrl` CSV columns. A catalog row records the dataset
  and revision in `source` (`catalog:<name>@<revision>`) and the per-row page in
  `sourceUrl`; both reach preview, commit, snapshot and correction provenance.
  Operator fields (SKU, quantity, price, currency) keep `user` provenance, and a row
  without the columns behaves exactly as it does today.
- Add `scripts/opendb-to-csv.mjs`, a dependency-free Node converter for a local
  BuildCores OpenDB checkout. It maps the eight supported categories, fills the commit
  SHA and `manufacturer_url`, refuses unmapped categories or missing required fields,
  and requires the operator to supply quantity and price because OpenDB carries none.
- Record the data-source assessment in `docs/data-sources.md`, add a `NOTICE` for the
  shipped seed, and add a README Licensing section: code MIT; shipped sample data
  CC BY 4.0 with attribution; OpenDB not distributed and only fetched by the operator
  under CC BY-NC-SA 4.0.
- Show on `/inventory` which facts are catalog-provided and which are operator-entered,
  including within the spec list, and state the sample's licence and verification date
  next to the download.

## Package Boundary and Split Assessment

Single outcome: the shipped example becomes real, sourced and traceable, and a real
catalog can be imported with provenance. Depends on `inventory-import` (CSV, preview,
commit, snapshot) and touches its compatibility inputs; it adds no rule, no
recommendation, no diagnosis and no price feed. The seed, the columns, the converter and
the licence record share one acceptance gate, so splitting them would ship a sourced
sample that no import can carry.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence | Compatibility and ownership | Decision |
| --- | --- | --- | --- |
| BuildCores OpenDB | `github.com/buildcores/buildcores-open-db` `NOTICE.md`, `schemas/*.schema.json` | CC BY-NC-SA 4.0; active bot sync; per-category JSON with the clearance fields the engine reads. Redistribution in an MIT repository would misstate downstream rights. | consume by operator fetch, never vendor |
| TechFuelHQ pc-builder-parts | `github.com/iBlessi/pc-builder-parts`, GitHub licence API `CC-BY-4.0` | CC BY 4.0 permits redistribution but the dataset is 126 rows and is not needed for a ten-row seed; manufacturer pages are the stronger primary source. | assessed, not vendored |
| PCPartPicker | site terms | Collection and derivative use prohibited. | never used |
| Octopart / Nexar, Open Icecat | user research; Icecat runtime account and token | Supply-chain or datasheet data; no clearance fields in the former, runtime credentials and ML-training restriction in the latter. | not a seed source |
| Forge Rust platform libraries | `../forge/modules/rust-platform-libs/Cargo.toml`, `forge.yaml` | Profile declares zero consumption; no verified package owns CSV provenance or catalog conversion. | keep local |
| Pinned design tokens | `apps/web/src/styles/vendor/tokens.css`, `.project.json` | Already vendored and pinned; the new catalog badge reuses `color-primary` and needs one measured pair. | adopt |

## User Experience and Interface Impact

Actors: the shop operator importing stock, and anyone who downloads the sample to learn
the format. Surfaces: `/inventory` import form and download area, preview table, snapshot
table and provenance disclosure. Each spec gains an `Operator-confirmed`, `Catalog` or
`Unknown` badge, each record names whether its facts are catalog or operator facts, and
the download area states the licence and verification date. Empty, parsing, review,
error, duplicate, committed, retry, earlier-revision and mobile states are preserved,
along with keyboard order, focus moves and live announcements. Browser and Lighthouse
evidence covers the changed page.

## BFS Impact Map

Rust: `domain::RowValues`, `evaluate_row`, `MatchInfo`, CSV mapping in `csv_import`,
override handling in `commit`, and the sample constant. TypeScript: `RowValues`,
`sourceLabel`, spec badges, preview/snapshot rows and the import form. Tests: Rust unit
and HTTP contracts, Vitest component tests, a Node test for the converter. Docs:
`docs/inventory-import.md`, `docs/compatibility-evidence.md`, new `docs/data-sources.md`,
`NOTICE`, README, `AGENTS.md` layout line, `ROADMAP.md`, evidence. SQLite schema is
unchanged; provenance lives in existing JSON columns. No dependency, lockfile, secret,
deployment flag or sibling source is touched.

## Capabilities

- `inventory-import` (modified): optional provenance columns, operator-versus-catalog
  labelling, and a sourced sample with a licence statement.
- `catalog-seed` (added): the seed's verification and licence rules, and the OpenDB
  converter contract.

## Non-goals

New compatibility rules, recommendation or diagnosis behaviour, prices or market data,
vendored third-party datasets, scraping PCPartPicker, a runtime catalog service, and a
checked-in Playwright suite.

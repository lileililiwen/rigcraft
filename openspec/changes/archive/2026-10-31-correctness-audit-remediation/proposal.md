# Proposal: Correct audited correctness and contract defects

## Why

Eight defects were reproduced against a fresh instance. Three are data-integrity or
contract failures that no test currently catches: snapshot totals saturate silently
at `i64::MAX`, operator text can break or inject into the Markdown export, and every
rejected request outside a handler returns plain text instead of the documented JSON
error envelope. Five are truthfulness or usability failures: unverified diagnosis
parts are labelled "Fine as is", `/diagnose` skips a section number before a list is
matched, the SPA fallback answers every unknown static path with index HTML, lowercase
currency is accepted while the contracts say it is invalid, and timestamps follow the
browser locale in a `lang="en"` interface. Each defect retires a question while
answering it wrongly, so the fix is verified by a failing test first and by browser
evidence for the human-facing states.

## What Changes

- Use checked arithmetic for every line total and snapshot total; a correction or
  commit whose amounts cannot be represented fails with HTTP 422 `price_overflow`
  and writes nothing.
- Give a described part with zero evaluated rules its own `unverified` action and
  group, and make the summary state what was and was not evaluated.
- Escape Markdown table cells and neutralise embedded HTML in the Markdown export;
  the JSON export remains exact data.
- Serve the SPA shell only for extensionless client routes; unknown static or
  metadata paths return 404 instead of index HTML.
- Wrap JSON and multipart extractor rejections in the documented
  `{ "error": { code, message } }` envelope with `application/json`.
- Render `/diagnose` as a stable 1–5 skeleton with a real match-review empty state.
- Normalise currency case-insensitively to upper case, require exactly three ASCII
  letters, and state that contract in the docs.
- Replace locale-dependent date formatting with one ISO 8601 UTC helper used across
  the UI, and add `RIGCRAFT_ADDR` to the README runtime instructions.

## Package Boundary and Split Assessment

Single outcome: correct the eight audited defects without adding scope. Depends on
all five archived packages because the defects cross inventory, compatibility,
recommendation, diagnosis and the shared HTTP surface. Excludes new hardware rules,
catalogs, pricing, deployment and new product capability. The defects share one
acceptance and verification gate, so splitting them would create eight partial
changes for one audited result.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence | Compatibility and ownership | Decision |
| --- | --- | --- | --- |
| Forge Rust platform libraries | `../forge/modules/rust-platform-libs/Cargo.toml`, `forge.yaml` | Profile declares zero consumption; no verified two-consumer package owns inventory totals, Markdown export or SPA routing. | keep local |
| Forge design tokens | `apps/web/src/styles/vendor/tokens.css`, `.project.json` | Pinned artifact already vendored; the new unverified badge reuses an existing measured token pair. | adopt |
| Existing PC builder projects | References in `requirement.md` | No source-level dependency; no external code or data is needed for a correctness fix. | not applicable |

## User Experience and Interface Impact

Actors: inventory operator, builder checking an existing PC, and anyone reading a
downloadable plan. Surfaces: `/inventory`, `/diagnose`, `/recommend` export, and the
service root. The diagnosis result gains a fourth honest group and a summary count;
`/diagnose` keeps its five numbered steps before a list is matched; all visible
timestamps become ISO 8601 UTC. Each state needs browser and accessibility evidence,
including before/after captures and a Lighthouse re-run for the changed pages.

## BFS Impact Map

Rust domain totals, commit and correction paths, diagnosis advice and plan, Markdown
export, Axum extractors and static fallback, and currency parsing are affected. The
React/TypeScript time helper, diagnosis result panel and page skeleton are affected.
SQLite schema and stored rows are unchanged; existing snapshots whose amounts already
fit remain readable. Docs, OpenSpec deltas, fixtures, API tests, Vitest tests, browser
captures and the Forge full Gate are affected. No dependency, lockfile, secret,
deployment setting or sibling source is touched.

## Capabilities

- `inventory-import` (modified): representable totals, extractor error envelope,
  honest static fallback, case-insensitive currency normalisation.
- `compatibility-evidence` (unchanged contract, shared envelope verified by tests).
- `build-recommendation` (modified): Markdown table escaping and HTML neutralisation.
- `build-diagnosis` (modified): explicit unverified advice state and stable numbering.

## Non-goals

New hardware rules, licensed catalogs, live pricing, deployment enablement, a
checked-in Playwright suite, and re-classifying parts whose rules did pass.

# Proposal: Make every report sentence grammatical

## Why

The service composes its user-visible sentences with the `(s)` pseudo-plural.
A recommendation reason reads `Total 1,672.90 USD leaves 127.10 USD under the
budget, and 8 unit(s) come from snapshot stock.`; a diagnosis plan reads
`1 step(s) add USD 379.00`; a stock refusal reads `needs 2 unit(s)`; a backup
refusal reads `missing table(s): diagnoses`. Every one of those is visible in
the API, in the Markdown export and in the browser, and `1 unit(s)` is not
English. Earlier packages fixed the counts the Web page composes and recorded
the engine's pseudo-plurals as a residual; this change closes that residual.

## What Changes

- **One helper.** `src/text.rs` holds `plural(count, singular, plural)` and
  renders `0 units`, `1 unit`, `2 units` for every count type already in use
  (unsigned counts, `i64` quantities Rust and the stored data keep signed, and
  the `f64` slot arithmetic). Zero is plural in English; only exactly one takes
  the singular noun.
- **Every service sentence.** The confirmed sites in `src/recommend/{score,
  explain,candidate,export}.rs`, `src/diagnosis/advice.rs`,
  `src/portability.rs` and `src/compatibility/mod.rs` are rewritten to call the
  helper, and the verbs around a count are rephrased so `1 complete combination
  … none passed`, `1 step adds`, `1 owned part` and `1 unit` all read
  correctly. A repository scan finds no other site.
- **A guard.** `tests/copy_style.rs` scans every `.rs` under `src/` and every
  `.ts`/`.tsx` under `apps/web/src/`, and fails naming the file and line when
  `\b[a-z]+\(s\)` appears. It also asserts the helper's zero/one/many output.
- **Assertions translated, not deleted.** The one Web fixture that quoted
  `1 step(s) add` now reads `1 step adds`. Rust API and export tests gain the
  grammatical sentence so the round trip proves the export carries it.
- **Docs follow the copy.** The evidence documents that recorded the engine's
  pseudo-plurals as a residual now say it is closed, and
  `docs/evidence/report-copy-plurals.md` quotes the corrected sentences as they
  were rendered by the running service.

## Impact

- Affected specs: `build-recommendation` (report and export prose),
  `build-diagnosis` (plan summary), `data-portability` (backup refusal),
  `compatibility-evidence` (stock refusal) each gain one copy requirement, and
  `ui-foundation` gains the source-scan guard; its existing sentence that the
  page renders engine prose verbatim is kept, and the clause that pinned the
  workbench strings as unchangeable is replaced by the same verbatim rule
  stated from the service's side.
- Affected code: `src/text.rs` (new), `src/lib.rs`, the eight source files with
  a pseudo-plural, `tests/copy_style.rs` (new), the one Web fixture string,
  `docs/`.
- Affected behaviour: prose only. Identifiers, error codes, JSON field names,
  schema, routes, status codes, scoring, rules, the vendored token file and the
  render tree are unchanged. Every e2e-asserted sentence, including
  `1 materially different build found; no option was padded to reach three.`,
  stays byte-identical.
- Non-goals: no new dependency, no colour or layout change, no route or API
  shape change, no change to app copy the Web already composes correctly.

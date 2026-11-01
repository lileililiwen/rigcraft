# Tasks: Make every report sentence grammatical

## 1. BFS — Baseline and impact
- [x] 1.1 Read `AGENTS.md`, `.ai-rules/`, `HANDOFF.md`, `docs/gap-closure.md` (package 1) and the four capability specs; confirm no active change.
- [x] 1.2 Scan `src/` and `apps/web/src/` for `\b[a-z]+\(s\)` and record the thirteen sites in eight files (design.md table); confirm `e2e/` asserts none of the edited sentences.
- [x] 1.3 Map the affected surface: the seven Rust modules, the Markdown/JSON export paths, the one Vitest fixture, the four owning capability specs, `ui-foundation`, `docs/design-program.md`, the two evidence documents that recorded the residual, and the shipped seed used for the proof.
- [x] 1.4 Assess shared infrastructure by name: `apps/web/src/plural.ts` already owns the Web rule and is reused for the one Web fixture; no Rust plural crate exists and none is added; no new dependency.

## 2. DFS — Helper and every site
- [x] 2.1 Add `src/text.rs` with `plural(count, singular, plural_form)` and its zero/one/many unit tests; declare `pub mod text` in `src/lib.rs`.
- [x] 2.2 Rewrite the four recommendation sentences (`score.rs`, `explain.rs` ×3, `candidate.rs` ×4, `export.rs`) so the count is grammatical and the surrounding verb still agrees.
- [x] 2.3 Rewrite the diagnosis plan summary and no-option summary (`advice.rs`), the backup missing-table detail (`portability.rs`) and the stock refusal (`compatibility/mod.rs`).
- [x] 2.4 Keep every identifier, error code, JSON field, schema, route and e2e-asserted string unchanged.

## 3. DFS — Guard test
- [x] 3.1 Add `tests/copy_style.rs`: scan every `.rs` under `src/` and every `.ts`/`.tsx` under `apps/web/src/`; fail with `path:line: text` on a pseudo-plural.
- [x] 3.2 Prove the detector with a positive and a negative sample and assert the helper writes `0 units`, `1 unit`, `2 units` for `u64`, `i64`, `usize` and `f64`.

## 4. BFS — Tests and docs
- [x] 4.1 Translate the Vitest fixture from `1 step(s) add` to `1 step adds`; add the grammatical sentence to the Rust export round-trip suite so the Markdown and JSON exports prove it.
- [x] 4.2 Update `docs/design-program.md` (the `(s)` finding is closed) and the two evidence documents whose "not done" bullets recorded the engine residual.
- [x] 4.3 Write `docs/evidence/report-copy-plurals.md` from the running service: the actual recommendation and diagnosis sentences, where each came from, and the explicit statement that no Lighthouse run applies.

## 5. Verification and close-out
- [x] 5.1 `cargo fmt --all -- --check`; `cargo clippy --all-targets --all-features -- -D warnings`; `cargo test`; `npm run check`; `npm test`; `npm run build`; `npm run test:e2e`; `./verify-local.sh`.
- [x] 5.2 `node scripts/check-openspec-change-names.mjs`; `openspec validate report-copy-plurals --strict`; `openspec validate --all --strict`.
- [x] 5.3 Run the Forge full Gate and record its aggregate verdict and labelled revision.
- [x] 5.4 Archive with `openspec archive report-copy-plurals --yes` (no `--skip-specs`), commit everything except `HANDOFF.md`, then commit `HANDOFF.md` only; do not push.

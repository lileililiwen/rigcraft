# Design: One pluralisation helper, one source guard

## Context

The `(s)` pseudo-plural is spread over four engines. A scan of `src/` and
`apps/web/src/` before the change found thirteen sites in eight files:

| File | Line | Text |
| --- | --- | --- |
| `src/recommend/score.rs` | 115 | `{free} free memory slot(s) … {modules} module(s)` |
| `src/recommend/explain.rs` | 120 | `{count} complete combination(s)` |
| `src/recommend/explain.rs` | 130 | `{} complete combination(s) … {} rejection(s)` |
| `src/recommend/explain.rs` | 151 | `{} node(s) and {} complete combination(s)` |
| `src/recommend/candidate.rs` | 99 | `{} validated build(s)` |
| `src/recommend/candidate.rs` | 130 | `{} unit(s) come from snapshot stock` |
| `src/recommend/candidate.rs` | 142 | `{owned_units} owned part(s)` |
| `src/recommend/candidate.rs` | 151 | `{} conflict(s) and {} unknown rule(s)` |
| `src/recommend/export.rs` | 134 | `{} node(s), {} complete combination(s), {} valid build(s)` |
| `src/portability.rs` | 309 | `missing table(s)` |
| `src/diagnosis/advice.rs` | 656 | `the rule(s) {}` |
| `src/diagnosis/advice.rs` | 676 | `{} step(s) add` |
| `src/compatibility/mod.rs` | 262 | `needs {} unit(s)` |

`apps/web/src` already had no pseudo-plural after the design program; the one
remaining reference was a Vitest fixture that quoted the engine's
`1 step(s) add` sentence.

## Decision 1 — one generic helper in `src/text.rs`

`plural<T>(count: T, singular: &str, plural_form: &str) -> String` where
`T: Display + PartialEq + Copy + From<u8>`. The existing counts are `u64`
(search and export counters), `usize` (rule and step lists), `i64`
(`unitsFromStock`, `ownedUnits`, selected quantity) and `f64` (the memory-slot
arithmetic). Requiring `From<u8>` is enough to compare against one without a
second trait or a stringified comparison, and `Display` already prints `2.0`
as `2`. No new dependency is added; a regex crate for the guard would be a
dependency for one ASCII scan.

The signature takes the plural form explicitly rather than appending `s`,
because `memory slot`/`memory slots` and `module`/`modules` are the same rule
but the caller owns the wording, and a later irregular noun does not need a
second helper.

## Decision 2 — rephrase the verb, do not add a second helper

Only the noun is pluralised, so a sentence whose verb agrees with the count
would read `1 complete combination were checked`. The four affected sentences
move the count into a position where the verb does not agree with it:

- `The search checked {n complete combinations} … and none passed: {n rejections} came from …`
- `… and snapshot stock supplies {n units}.`
- `Included at zero incremental price and taking no stock: {n owned parts}.`
- `… {n steps} {adds|add} {money} …` keeps the one explicit verb branch because
  "step adds" and "steps add" are both natural and neither phrasing hides the
  count.

No sentence gains a second grammar rule beyond singular/plural.

## Decision 3 — a source scan, not a rendered-text assertion

`tests/copy_style.rs` walks `src/**/*.rs` and `apps/web/src/**/*.{ts,tsx}` and
flags a lowercase word immediately followed by `(s)`, which is the
`\b[a-z]+\(s\)` shape the requirement names. It is a plain byte scan, not a
regex dependency, and it reports `path:line: text` so a failure names the
offender. The scan is deliberately broader than the eight edited files: an
engine file added later is covered without anyone remembering the rule.

The test also runs the detector against a positive and a negative sample, so a
scan that silently stopped matching would fail instead of passing.

## Decision 4 — the Web renders engine prose verbatim, the service owns grammar

`ui-foundation` already requires the check and diagnosis interfaces to render
recommendation prose and exported report text verbatim. That stays true: the
page still prints exactly what the engine sends. The one clause that described
the workbench strings as permanently pinned is replaced with the same rule
stated from the service's side, because otherwise this change would contradict
it by making the engine sentences grammatical. The page composes no `(s)` of
its own and its `plural` helper is untouched.

## Decision 5 — no structure moves

Only `format!` arguments and four sentence shapes change. Every identifier,
error code (`insufficient_stock`, `backup_wrong_shape`, `no_option`,
`compatibility`), JSON field name (`reasons`, `detail`, `summary`,
`explanation`), schema version, route, status code and score is unchanged. The
JSON export still serialises the same `report` object; only its sentence
strings differ, and the Markdown export writes the same sentence. Ship-only
strings asserted by `e2e/` are not touched: the scan confirms the edited
sentences do not include `1 materially different build found` or
`no option was padded to reach three`.

## Verification shape

`cargo test --test copy_style` proves the guard and the helper; the changed
API/export suites prove the sentence in the response and both exports; the
evidence document quotes the running service's own text for a recommendation
and a diagnosis; `./verify-local.sh` runs the whole matrix. There is no
Lighthouse run: no layout, colour or route changed, and the change is invisible
except in the words.

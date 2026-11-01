# ui-foundation deltas

## MODIFIED Requirements

### Requirement: Console counts use real plural forms
The check and diagnosis interfaces SHALL render every count they compose through
one shared pluralisation helper that yields the singular noun for a count of one
and the plural noun otherwise. No check-composed or diagnosis-composed string
SHALL contain an `(s)` pseudo-plural. Recommendation prose and exported report
text SHALL be rendered verbatim; the service owns their grammar and the page
SHALL NOT paraphrase them.

#### Scenario: Singular and plural forms follow the count
Given the shared helper, when it is called with one unit and with two units, then it returns `1 unit` and `2 units`.

#### Scenario: No page-composed pseudo-plural remains
Given a completed check and a completed diagnosis at any count, when the rendered page text is searched for `line(s)`, `part(s)`, `unit(s)`, `rule(s)` or `conflict(s)`, then none is found in text the page composed.

## ADDED Requirements

### Requirement: A source scan rejects the `(s)` pseudo-plural
`tests/copy_style.rs` SHALL scan every Rust source under `src/` and every
TypeScript source under `apps/web/src/` and fail when a lowercase word is
immediately followed by `(s)`, naming the file and line. The scan SHALL cover a
source file added later without a per-file allowlist. The shared Rust helper
SHALL render `0 units`, `1 unit` and `2 units` for the counts it is given, and
zero SHALL take the plural noun.

#### Scenario: The scan names an offender
Given a source line containing `2 unit(s)`, when the guard test runs, then it fails and prints that file path and line number.

#### Scenario: The shipped sources pass
Given the shipped `src/` and `apps/web/src/` trees, when the guard test runs, then it passes and finds at least one source file in each tree.

#### Scenario: The helper distinguishes zero, one and many
Given the shared Rust helper, when it is called with 0, 1 and 2 units, then it returns `0 units`, `1 unit` and `2 units`.

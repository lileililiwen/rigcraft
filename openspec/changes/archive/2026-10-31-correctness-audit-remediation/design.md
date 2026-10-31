# Design: Correct audited correctness and contract defects

## Boundary and runtime

Implement in RigCraft only. Rust 2021 + Axum + SQLite owns validation, totals,
export and routing; React/TypeScript + Vite owns presentation. No dependency is
added. The service keeps listening on `RIGCRAFT_ADDR` with data in
`RIGCRAFT_DATA_DIR`; the README documents that override because port 8080 is not
always free.

## Decision ledger

### 1. Representable totals

`Snapshot::totals_for` returns `Result<SnapshotTotals, TotalsOverflow>` and uses
`checked_mul` for each line total and `checked_add` for unit and value totals. A
caller maps overflow to HTTP 422 `price_overflow` with field `price`. The commit
path rejects the candidate before any transaction; the correction path rejects
before inserting the new revision; the diagnosis path propagates the same error.
`i64::MAX` is never a valid stored total, and no `saturating_*` remains in total
arithmetic. Each row's own price and quantity are still validated by the existing
CSV rules, so the new failure is specifically a combination that cannot be
represented.

### 2. Unverified diagnosis advice

A described part whose rules are all absent is not evidence of compatibility.
`advice_for` therefore computes the union of pass, conflict and unknown rule ids;
when it is empty the action is `unverified` with an explicit reason and zero rule
ids. `AdviceGroups` gains `unverified`, so the API returns four non-overlapping
groups. Parts with at least one passing rule stay `keep`; nothing is reclassified.
The result panel labels the fourth group "Not verified — no rule evaluated" and the
summary reports both the rule counts and how many parts the checker did not
evaluate.

### 3. Markdown injection

Every dynamic table cell passes through one escaper: backslash becomes `\\`, pipe
becomes `\|`, line breaks become spaces, and `<`/`>` become `&lt;`/`&gt;`. Structural
Markdown stays authored, not escaped. The JSON export is unchanged: it is data and
must round-trip exactly. A fixture model containing `|`, a backslash and an HTML tag
proves the rendered table keeps its declared column count.

### 4. SPA fallback

The SPA shell is served only when the requested path is extensionless and is not
under `/assets/`. A path whose final segment contains a dot is treated as a static
or metadata file: it is served by `ServeDir` when present and otherwise returns 404.
`/robots.txt`, `/llms.txt` and `/ai-catalog.json` are not shipped, so they 404 rather
than claiming content the project cannot evidence. `/api/*` keeps its JSON 404.

### 5. Extractor error envelope

`ApiJson<T>` wraps `axum::Json` and maps every `JsonRejection` to `ApiError`:
missing/unsupported content type and body-limit failures become `invalid_body`,
syntax and type failures become `invalid_json`, all with `application/json`.
`preview_import` accepts `Result<Multipart, MultipartRejection>` so a missing or
malformed boundary becomes `invalid_body` through the same envelope. Handlers keep
their existing field-coded errors.

### 6. Stable `/diagnose` numbering

Section 3 is rendered whenever a snapshot is loaded, with an empty state that says
the review appears after a list is matched. Focus moves to its heading only when a
list exists. The rendered skeleton is therefore 1–5 on the first paint, and no number
is reserved for conditional content.

### 7. Currency case

`parse_currency` trims, uppercases, and requires exactly three ASCII letters. That is
case-insensitive normalisation, not case-sensitive acceptance: `usd` becomes `USD`,
`us` is `currency_format`. The UI continues to send uppercase. All five contracts
state the normalisation rule.

### 8. Timestamps

One helper `formatTimestamp(iso)` returns ISO 8601 UTC (`toISOString`, trimming a
zero millisecond field) and never consults `navigator.language`. Every visible
timestamp uses it, and timestamp elements use `<time dateTime={iso}>`. A Vitest test
sets two different `navigator.language` values and asserts the same output.

## Invariants and failure policy

Rejected requests never change state; a totals failure writes no snapshot or
revision. A part is never called verified unless at least one rule evaluated it.
Markdown is never allowed to gain or lose a cell because of operator text. An
unshipped static file is never replaced by HTML. The API error shape is identical
for handler and extractor failures. No catalog, AI provider, secret, deployment flag
or SQLite schema changes.

## Verification oracle

- Rust: unit tests for checked totals; API tests for commit and correction overflow,
  extractor envelopes, SPA routes, currency normalisation, the unverified group and
  Markdown escaping.
- Web: Vitest for the stable skeleton, the unverified group, timestamp invariance and
  `<time>` attributes.
- Browser: Chrome DevTools MCP at 1440×900 and 390×844 for `/diagnose` before/after,
  the unverified result, the stable numbering, ISO timestamps and static 404s;
  captures under `docs/assets/` and `docs/evidence/correctness-audit-browser.md`.
- Gates: `cargo fmt`, `cargo clippy`, `cargo test`, `npm run check`, `npm test`,
  `npm run build`, `./verify-local.sh`, the change-name checker, strict OpenSpec
  validation and the Forge full Gate. Lighthouse is re-run for the changed pages and
  its SEO/agentic scores are reported honestly.

## Decision on non-goals

The audit's own instructions settle the currency question (normalise case) and the
metadata question (404 rather than invented content). Parts that did pass rules stay
`keep`, and existing snapshots are not rewritten; only new writes are checked.

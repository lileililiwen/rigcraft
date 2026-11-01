# Design: Make the HTTP surface and the CSV export honest

## Context

`build_router` mounts the API routes, then `.route("/api/{*rest}", any(api_not_found))`,
and finishes with `fallback_service(ServeDir::new(web_dir).fallback(spa_service))`.
`spa_service` answers with the shell whenever `is_client_route(uri.path())` is
true, and `is_client_route` only rejects `/assets/` and a final segment containing
a dot. So:

- `/api` and `/api/` never reach `api_not_found` (the wildcard needs at least one
  segment after the slash), fall through to `ServeDir`, miss on disk, reach
  `spa_service`, and get the shell with 200.
- `/foo`, `/recomend` and any other dotless miss reach the same branch and get the
  shell with 200. The SPA then resolves the unknown path to `InventoryPage`,
  because `App.tsx` falls through to it.
- `/API/health` is not the `/api/health` route (axum matching is case-sensitive),
  so it takes the same shell path.

`snapshot_csv` writes nine cells per lot through `csv::Writer` with
`QuoteStyle::Necessary`. Nothing looks at the first character, so a model or SKU
that begins with `=`, `+`, `-`, `@`, TAB or CR reaches the spreadsheet as an
executable formula.

`resolve_mapping` builds the `mapping_missing` refusal from the required-field
list alone. A semicolon-delimited file parses as one column containing `;`, so
every required field is absent and the message lists all five, which is exactly
what the operator can see in the file.

## Decision 1 — enumerate the client routes instead of inferring them

`is_client_route` becomes an exact allowlist: `/`, `/inventory`, `/catalog`,
`/builds/check`, `/recommend`, `/diagnose`, each also accepted with one trailing
slash. Anything else without a file extension is not a client route.

This is deliberately a closed list rather than a pattern. The six routes are the
routes `App.tsx` resolves; a seventh page exists only when this function and
`App.tsx` change together. An inferred rule is what produced the soft 404: it
guessed that "extensionless" means "a page".

Trailing slashes are accepted because a pasted link with a trailing slash is the
same page, and rejecting it would be a new way to break a bookmark. `/assets/*`
and any final segment containing a dot stay non-client, so static and metadata
files keep the existing 404.

## Decision 2 — a plain-text 404 document

An unrecognised path returns `404`, `Content-Type: text/plain; charset=utf-8` and
a static body naming the real entry points:

```text
404 Not found.
RigCraft has no page at this address. Open /inventory to import stock, or
/catalog to manage datasets.
```

It is not a styled page. That is the minimal honest answer: the path is not a
page, the response looks nothing like the app, and there is no new dependency, no
colour literal, no token pair to measure and no second render path to keep in
step with the shell. The body is a constant, so the requested path is never
reflected into the response — a reflected path is how a 404 becomes an injection
surface.

A plain-text document has no meaningful Lighthouse audit: the tool audits an HTML
document's accessibility, SEO and best-practice signals, and this response has no
document structure to audit. The evidence document records that reason instead of
a score.

## Decision 3 — the API root is an API route

`/api` and `/api/` are mounted explicitly with `any(api_not_found)`, beside the
existing `/api/{*rest}` wildcard. All three go through the same
`ApiError::not_found("route_not_found", "This API route does not exist.")`, so the
envelope, content type and code are identical to `/api/nope`.

`any(...)` is used rather than `get(...)` on those two paths: a `POST /api` is not
a wrong method on a real endpoint, it is an unknown route. Real endpoints keep
`get(...)`/`post(...)`, so `POST /api/health` still returns axum's 405.

`/API/health` is not special-cased. It is an unknown extensionless path, so it
now returns the 404 document instead of the shell. Matching the API prefix
case-insensitively would invent aliases the documented route table does not have.

## Decision 4 — neutralise by prefix, keep JSON raw

```rust
fn neutralise(value: &str) -> String {
    let triggers = ['=', '+', '-', '@', '\t', '\r'];
    let mut chars = value.chars();
    let dangerous = match chars.next() {
        Some(first) if triggers.contains(&first) => true,
        Some('\'') | Some('"') => chars.next().is_some_and(|next| triggers.contains(&next)),
        _ => false,
    };
    if dangerous {
        format!("'{value}")
    } else {
        value.to_string()
    }
}
```

Every cell of every row is passed through it, including the numeric `quantity`
and `price` columns, because a negative quantity is `-5` and the same rule must
hold everywhere rather than being remembered per column. `sourceUrl` is a
declared `http(s)` URL and cannot begin with a trigger, so it passes unchanged.

The quote case exists because a value beginning `"=1+1` is quoted by the CSV
writer and read by the spreadsheet as `=1+1`; the trigger is behind one quote
character, not at position zero.

The cost is real and is documented: a value that genuinely begins `-` or `+`
gains one character on re-import, and an apostrophe can appear in a cell that
visibly should not have one. The JSON export is unaffected and remains the
authoritative artifact.

## Decision 5 — name the delimiter, echo nothing

In `resolve_mapping`, after the missing list is computed:

```rust
let semicolon_header =
    headers.len() == 1 && headers[0].contains(';');
```

When that is true the message becomes

```text
Required columns are missing: sku, category, model, quantity, price.
The header looks semicolon-delimited; RigCraft expects comma-separated columns.
```

and `details` gains `"delimiter": "semicolon"`. The code stays `mapping_missing`,
the field stays `missing[0]`, and `details.missing` stays the same array, so a
caller that switches on any of them is unaffected.

The single-column check is what makes the hint true rather than a guess: a
comma-separated file with one column and a `;` inside a value would fail the
same missing-columns check but would not produce a one-column header, and a
normal header that merely contains a semicolon in a quoted value still parses
into several columns. The file's content is never echoed; only the fact of the
delimiter is named.

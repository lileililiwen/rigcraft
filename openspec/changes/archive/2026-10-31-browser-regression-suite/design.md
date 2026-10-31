# Design: A checked-in browser regression suite

## Problem shape

The product is a Rust service that serves a built React bundle. Its defects live at the
seam between the two: a route that returns the shell with the wrong status, an
envelope that stops being JSON, a group whose heading says "Fine as is" for a part no
rule evaluated, a list that skips a number when a section is conditionally rendered.
Vitest sees the components with a mocked fetch and cannot see any of it. The suite must
therefore render the real bundle from the real binary, seed the real sample, and assert
what a person sees.

## Decisions

### Real Chrome and no browser download

`channel: 'chrome'` uses `/usr/bin/google-chrome` (154.0.8037.97 here). Playwright's
own Chromium download is not required, so a fresh checkout runs the suite after
`npm ci` with no additional network step. One project, headless, `chromium`. No
Firefox or WebKit project: the product ships one Web bundle and the evidence documents
describe one engine, so a second engine would be an unclaimed compatibility statement.

### A dedicated port the suite selects, never 8080

`npm run test:e2e` resolves a free loopback port (`net` server bound to `0`, read the
port, close) and passes it to Playwright as `RIGCRAFT_E2E_PORT`. `RIGCRAFT_E2E_ADDR`
does not exist; the value is the suite's own. Port 8080 is used by an unrelated tool on
this host and 8098/8099 are stale rigcraft instances, so a fixed default would either
collide or silently test the wrong process. The config still falls back to 8137 when
`npx playwright test` is run directly, and the server entry point fails loudly if the
bind fails instead of retrying.

### A fresh scratch data directory per run, never committed

`e2e/start-server.mjs` creates `mkdtemp(join(tmpdir(), 'rigcraft-e2e-'))`, sets
`RIGCRAFT_DATA_DIR` to it, spawns `./target/debug/rigcraft` with
`RIGCRAFT_ADDR=127.0.0.1:<port>`, forwards termination signals, and removes the
directory when the child exits. The repository therefore never receives an inventory
database, and a run never reads a developer's private stock. `cargo build` is a
documented prerequisite; the server entry point refuses to start when the binary is
missing rather than building implicitly, so the suite cannot be the only place a build
happens.

### `webServer` waits for `/api/health`

Playwright starts the service and polls `/api/health` before the first test, and stops
it after the last. That is the same contract README documents for a manual run, and it
means a hung or panicking service fails as a start-up error with its stderr, not as
thirty unrelated assertion failures.

### One worker, no retries, tests isolated by snapshot

The suite runs with `workers: 1` and `retries: 0`. The tests are independent in data:
each one seeds its own snapshot through the API and selects that exact snapshot id in
the page's `<select>`, so no test reads another test's rows. Each preview commit is a
new chain rooted at revision 1, so asserting "revision 1" is true for every test rather
than only for the first writer. Parallel workers are deliberately not enabled: the
tests share one SQLite file and one server process, so calling them "isolated" would be
false even though each owns its rows. A retry would hide a flake in exactly the
behaviour this suite exists to hold, so retries stay at zero and a failure is reported.

### Assert the seed through the UI; seed through the API

The inventory journey performs the whole operator path in the browser — download,
file input, preview, commit — because that path is what the earlier evidence claimed.
The other journeys seed through `POST /api/imports` and its commit because the import
is not what they assert, and repeating it through the UI would triple the runtime
without adding a compatibility, recommendation or diagnosis assertion. Seeding is
verified: the helper fails if the preview is not exactly 10 valid rows with no
duplicates, or if the commit is not revision 1 with 10 records.

### What each journey asserts, and why it can fail

| Journey | Assertion that can fail |
| --- | --- |
| Inventory | download filename, CSV header and 10 data rows, preview counts, 10 table rows, heading "Committed snapshot · revision 1", 10 committed records, `model`/`specs` provenance `Catalog · manufacturer@2026-10-31` with the AMD URL, `sku`/`quantity`/`price` provenance `User-provided · csv:<file>` and no catalog claim on stock |
| Compatibility | the exact status line `12 compatible · 0 conflicts · 2 unknown`, summary 12/0/2, overall badge `Unknown`, the two unknown rule ids, and their follow-up sentences |
| Recommendation | first ranked option is `1.`, its total parses and is below the budget, its badge is `Unknown` and not `Compatible`, the "Confirm before building" list is present; a budget below the minimum returns heading `Budget constraint`, headline "The budget is below the cheapest complete build." and the minimum amount |
| Diagnosis | three lines reported `Matched` / `Ambiguous` / `No match`, no preselected candidate on the ambiguous line, "Resolve the match review first" before resolution, then `Not verified — no rule evaluated (2)`, `Fine as is (0)`, zero `.advice-card--keep` and no compatible badge inside the unverified group |
| Regression | three static/metadata paths return 404 with no `id="root"` shell while `/diagnose` returns 200 HTML; a malformed JSON POST is `application/json` with `error.code === 'invalid_json'`; `/diagnose` h2 sequence is exactly sections 1–5; a `<time>` renders ISO 8601 UTC under `locale: 'de-DE'` |
| Keyboard/responsive | the first Tab lands on the skip link, Tab reaches the budget field, Enter submits the form and a ranked option appears with no pointer input; at 390 px `documentElement.scrollWidth === clientWidth` on all four routes |

No assertion is satisfied by "the page loaded": each names a value, a count, an id, a
status, a header or a heading. Where a behaviour was claimed but not reproducible, the
assertion is dropped and reported rather than weakened.

### Artifacts stay out of git

`test-results/` holds the JSON report, failure screenshots and traces, and is
gitignored. The downloaded sample is written to the test's own output directory. No
screenshot is committed: this change adds executable evidence, not new captures, and
the existing captures remain the visual record.

### Determinism

`locale: 'en-US'`, `timezoneId: 'UTC'` and no `Date`-dependent assertion keep money and
time formatting stable; the regression spec overrides the locale to `de-DE` so the ISO
8601 timestamp assertion proves locale independence instead of restating the default.
No test calls an external host: every request is same-origin to the suite's own port.

## Compatibility and cost

`@playwright/test` is a root devDependency; it is not runtime code and does not enter
the bundle. Its three transitive packages add no native build step, and `channel:
'chrome'` avoids the ~130 MB bundled-browser download. The suite runs in tens of
seconds with one worker, well inside the two-minute budget.

## Failure modes considered

- **Port stolen between probe and bind.** The server exits with the OS bind error and
  Playwright reports start-up failure with stderr, rather than talking to the other
  process.
- **Stale binary.** `start-server.mjs` requires `target/debug/rigcraft`; a missing
  binary is a named error. `verify-local.sh` runs `cargo build` through `cargo test`
  first, and the README lists the prerequisite.
- **Leftover temp directory after SIGKILL.** Cleanup runs on child exit, process exit
  and forwarded signals; a SIGKILL can leave a uniquely named directory in the OS temp
  directory, which is outside the repository and harmless.
- **A future UI copy change.** Assertions target roles, ids and the values the contracts
  already publish; where they quote copy (headings, follow-ups), that copy is the
  behaviour the evidence documents claim, so a change must update both.

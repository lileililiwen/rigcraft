# browser-regression Specification

## Purpose
Define the executable browser regression contract: a checked-in Playwright suite that drives the built Web bundle against the real Rust service and the shipped sample, the journeys and audited-fix guards it must hold, and the local gate that runs it.
## Requirements
### Requirement: A checked-in Playwright suite drives the built Web bundle against the real service
The repository SHALL contain a Playwright suite that renders the bundle the Rust service serves, not a mocked component tree. The suite SHALL run real Chrome through `channel: 'chrome'` so no browser download is needed, in one headless chromium project, with no retries, JSON and list reporters, and a trace kept only on retry.

The suite SHALL own its runtime. It SHALL select its own free loopback port, start `./target/debug/rigcraft` with a fresh `RIGCRAFT_DATA_DIR` under the operating system's temporary directory, wait for `/api/health` before the first test, stop the service afterwards, and remove the scratch directory. It SHALL NOT bind port 8080, reuse an existing data directory, build implicitly, or require network access. `cargo build` SHALL be documented as its prerequisite.

#### Scenario: The suite starts the service on its own port
Given the repository with a built `target/debug/rigcraft`, when `npm run test:e2e` runs, then the service binds a free loopback port with a new temporary data directory, `/api/health` answers before the first test, and the directory is removed after the run.

#### Scenario: A missing binary is a named error
Given no `target/debug/rigcraft`, when the suite starts, then it reports that the service must be built and starts no test.

#### Scenario: No test reaches the network
Given the suite's configuration, when the journeys run, then every request is same-origin to the suite's port and no catalog, provider or third-party host is contacted.

### Requirement: The journeys assert the shipped seed, not synthetic fixtures
The suite SHALL seed the shipped sample and assert what the operator sees. Each test SHALL seed its own snapshot and select that snapshot explicitly, so no test depends on another test's rows or on execution order. The inventory journey SHALL drive download, preview and commit through the page; the other journeys MAY seed through the documented API.

#### Scenario: Inventory commit and provenance split
Given the shipped sample downloaded from `/api/imports/sample` and loaded through the file input, when it is previewed and committed, then the page shows revision 1 with ten records, `model` and `specs` cite `catalog:manufacturer@2026-10-31` with the manufacturer URL, and `sku`, `quantity` and `price` cite the operator's CSV rather than the catalog.

#### Scenario: Compatibility is not a green pass
Given the full seeded AM5 build selected on `/builds/check`, when the check runs, then the page reports `12 compatible · 0 conflicts · 2 unknown`, the overall verdict badge reads `Unknown`, and `cpu-bios` and `psu-capacity` are each listed with a stated fact to confirm.

#### Scenario: Recommendation is ranked and honest
Given a budget that fits the seeded build, when the recommendation runs, then a ranked option is shown with a total below the budget and an `Unknown` status label rather than `Compatible`. Given a budget below the cheapest complete build, then the page names the budget as the binding constraint and states the minimum.

#### Scenario: Diagnosis never coerces a line or praises an unverified part
Given a pasted list containing one exact, one ambiguous and one unmatched line, when it is matched, then each line reports its own status and the ambiguous line preselects no candidate. When a diagnosis is attempted before resolution, then it is blocked with the unresolved lines named. After resolution, a described build that no rule can evaluate shows the group `Not verified — no rule evaluated`, shows `Fine as is (0)`, and carries no check mark for any unverified part.

### Requirement: Regression guards hold the audited fixes
The suite SHALL fail if an audited browser-visible fix regresses.

#### Scenario: Static and metadata paths are not the SPA shell
Given the built bundle, when `/assets/nope.js`, `/robots.txt` or `/llms.txt` is requested, then the response is 404 and does not contain the shell's root element, while a client route still returns 200 HTML.

#### Scenario: A malformed JSON body keeps the JSON envelope
Given a malformed JSON body sent as `application/json`, when a JSON endpoint receives it, then the response is `application/json` and its `error.code` is `invalid_json`.

#### Scenario: The diagnosis page keeps its numbering
Given a snapshot and no matched list, when `/diagnose` renders, then sections 1 to 5 appear in order.

#### Scenario: A timestamp is ISO 8601 UTC under any locale
Given a browser locale whose customary date format is not ISO 8601, when a snapshot timestamp renders, then it matches ISO 8601 in UTC with a trailing `Z` and carries no locale-dependent separators.

### Requirement: Keyboard and responsive behaviour is held by the suite
The suite SHALL include one journey completed from the keyboard alone and SHALL assert, at a 390 px viewport, that `document.documentElement.scrollWidth` equals `clientWidth` on every page it touches.

#### Scenario: A keyboard-only journey completes
Given the recommendation page, when the operator tabs from the document start to the budget field, types a budget and presses Enter, then the first Tab landed on the skip link and a ranked option appears without pointer input.

#### Scenario: No horizontal overflow at 390 px
Given a 390 px viewport, when each of `/inventory`, `/builds/check`, `/recommend` and `/diagnose` is opened, then its document scroll width equals its client width.

### Requirement: The suite is wired into verification and its evidence is recorded
`npm run test:e2e` SHALL run the suite, and `./verify-local.sh` SHALL run it so a local gate blocks on it. `AGENTS.md` Project facts and the README Runtime section SHALL name the command and its prerequisite. Each browser-evidence document that recorded a missing checked-in suite SHALL state that the suite exists and what it covers, instead of leaving the limitation in place.

#### Scenario: The gate runs the browser suite
Given a passing Rust and Web build, when `./verify-local.sh` runs, then it reports the browser suite's verdict and fails when a journey fails.

#### Scenario: Documents stop claiming the suite is missing
Given `docs/evidence/*-browser.md`, when they are read, then none claims that no checked-in Playwright suite exists, and each names the coverage the suite adds or links to the statement that does.

### Requirement: The browser suite compares the rendered pixels
The declared browser command SHALL run a visual spec that compares committed
baselines for the six routes with Playwright's `toHaveScreenshot()`. The spec SHALL
run inside `npm run test:e2e` with no opt-in flag, no skipped project and no
separate command that a gate could omit, and `./verify-local.sh` SHALL therefore
block on it. A pixel difference beyond the bounded tolerance SHALL fail the test
and leave a diff artifact naming the expected, actual and diff images.

#### Scenario: The visual spec is part of the declared suite
Given a clean checkout with a built service and bundle, when `npm run test:e2e` runs, then the twelve visual captures run in the same invocation as the existing journeys and no flag is needed to include them.

#### Scenario: A regression fails with an artifact
Given a changed pixel in a captured route, when the suite runs, then the visual test fails, prints the differing pixel count or ratio, and writes the diff image Playwright names.

#### Scenario: The local gate blocks on the pixels
Given `./verify-local.sh`, when it reaches the browser stage, then a failing visual comparison fails that stage rather than being reported as a separate optional check.

### Requirement: Critical journeys run in Firefox and WebKit as separate smoke projects
The declared browser command SHALL run one smoke subset in a `firefox` project and
a `webkit` project, in addition to the Chrome project. Those two projects SHALL
match only `e2e/smoke.spec.ts`; the Chrome project SHALL keep running every
journey, including the committed visual baselines. The smoke subset SHALL cover
the six client routes returning the shell with their own document title, the skip
link receiving focus on the first Tab and moving focus to the main region,
`document.documentElement.scrollWidth === clientWidth` at 390 px on every route, a
typo path returning the real 404 document, a recommendation generated from the
seeded sample, and the inventory CSV download carrying its revision-bearing
filename. It SHALL reuse the suite's existing seeding and overflow helpers rather
than duplicate them. Neither project SHALL run the Chrome pixel baselines or
replace a Chrome journey. A browser that cannot launch on the host SHALL be
reported with its exact dependency error and recorded in the evidence document,
never omitted or silently skipped, and the smoke subset SHALL NOT depend on an
environment variable that switches a project off.

#### Scenario: Firefox and WebKit run the smoke subset
Given a built service and bundle and installed browsers, when `npm run test:e2e` runs, then the `firefox` and `webkit` projects each run the six smoke tests and each reports a pass count.

#### Scenario: Chrome keeps every journey
Given the same run, when the projects are read, then the `chromium` project still runs every journey including the twelve visual comparisons, and the smoke tests are additional rather than a replacement.

#### Scenario: The smoke assertions hold outside Chrome
Given the `firefox` or `webkit` project, when the smoke subset runs, then each route serves the shell with its own title, the first Tab stop is the skip link and activating it focuses `main`, no route overflows at 390 px, `/recomend` is a real 404, a 1,800.00 recommendation shows the gaming rule, and the CSV download is named `rigcraft-inventory-snapshot-r1-*.csv`.

#### Scenario: A browser that cannot run is named
Given a host whose WebKit dependencies are missing, when the evidence is read, then it quotes the loader or host-validation error and the browser's verdict, rather than listing the browser as covered or leaving it out.

#### Scenario: No cross-engine pixel baseline is claimed
Given the committed snapshot directory, when it is read, then it holds Chrome baselines only, and no Firefox or WebKit image is committed by this change.


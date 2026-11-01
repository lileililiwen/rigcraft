# browser-regression deltas

## ADDED Requirements

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

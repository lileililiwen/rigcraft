# ui-foundation deltas

## ADDED Requirements

### Requirement: The production build is measured on every client route
A Lighthouse performance-only report SHALL exist for `/`, `/inventory`, `/catalog`,
`/builds/check`, `/recommend` and `/diagnose`, measured against the production build
with the shipped sample snapshot and an active catalog dataset, on the mobile form
factor and with the desktop preset, and SHALL be committed under
`docs/evidence/perf-lighthouse/`. `docs/evidence/performance-evidence.md` SHALL
tabulate, per route and device, the performance score, first contentful paint,
largest contentful paint, total blocking time, cumulative layout shift, speed index
and transferred bytes; SHALL name the measured asset file names and their hashes;
and SHALL record the exact command and report path. The record SHALL state that
these are localhost lab measurements, not field data.

#### Scenario: Every route has both reports
Given the six client routes, when the evidence directory is read, then each route has one desktop report and one mobile report, each a Lighthouse JSON document for that URL with `only-categories` performance.

#### Scenario: The table matches the shipped build
Given the reports and the built `apps/web/dist/assets` files, when the evidence document is read, then the asset file names and hashes it names are the ones the reports measured.

#### Scenario: A finding is fixed or explained
Given a non-perfect performance audit, when the evidence document is read, then either the change that fixes it and its re-measured number are recorded, or the measured number and the reason it is not fixed are recorded.

### Requirement: The built client assets have an enforced byte ceiling
`scripts/check-bundle-size.mjs` SHALL read every built `apps/web/dist/assets/*.js`
and `*.css` file, compare each against an explicit per-type ceiling declared in the
script, and exit non-zero when a file exceeds its ceiling, naming the file, its size,
the ceiling and the delta. A missing or empty build directory SHALL fail the check
rather than pass it. `npm run build` and `npm run check` SHALL run the check, and the
declared verification script SHALL build the Web app before it runs the check, so the
check reads the fresh build. `scripts/check-bundle-size.test.mjs` SHALL prove with a
temporary directory that a within-budget fixture passes and an over-budget fixture
fails, without writing to the repository.

#### Scenario: A build within the ceiling passes
Given a build whose every JavaScript and CSS asset is at or below its type's ceiling, when the check runs, then it exits zero and reports each file with its size and ceiling.

#### Scenario: An over-budget file fails and is named
Given one asset larger than its ceiling by one byte, when the check runs, then it exits non-zero and names the file, its size, the ceiling and the delta.

#### Scenario: No build is a failure, not a pass
Given a checkout with no `apps/web/dist/assets` directory, when the check runs, then it exits non-zero and says the Web build must run first.

#### Scenario: The gate reads the fresh build
Given `./verify-local.sh`, when the Node stages run, then `npm run build` completes before `npm run check`, and a bundle over its ceiling fails that script.

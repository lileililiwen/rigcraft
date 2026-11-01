# ui-foundation deltas

## ADDED Requirements

### Requirement: The interface has committed pixel baselines
The repository SHALL hold a committed Playwright `toHaveScreenshot()` baseline for
each of `/inventory`, `/catalog`, `/builds/check`, `/recommend`, `/diagnose` and
the real 404 at `/nope`, at 1440×900 and 390×844, generated from the production
build with the data the browser suite seeds. Each baseline SHALL be a viewport
capture, not a full-page capture. Every test SHALL seed its own state immediately
before the capture, so a baseline depends on the shipped seed and not on which
spec file ran before it. Dynamic values — snapshot, check, recommendation and
diagnosis identifiers, committed and checked timestamps, the sha256 fragment and
the snapshot `<select>` label that embeds a timestamp — SHALL be masked by a
locator over a block-level container, SHALL NOT be absorbed by raising the pixel
tolerance, and the per-pixel `threshold` SHALL bound antialiasing only.

#### Scenario: Every route and width has a baseline
Given the committed snapshot directory, when it is read, then it holds twelve images — one per route and per viewport — and `e2e/visual.spec.ts` captures each of them from the built bundle.

#### Scenario: A second run on the same database matches
Given the service with a database that already holds a sample snapshot and the synthetic catalog, when the visual spec runs twice in a row, then both runs compare against the same committed baselines and pass.

#### Scenario: Dynamic values are masked, not tolerated
Given a captured route, when the mask list is read, then each dynamic value is covered by a locator over a block-level container and the capture tolerance is at most one percent of the viewport.

#### Scenario: An unintended visual change fails
Given a one-token change to a brand surface colour and a rebuilt bundle, when the visual spec runs, then it fails and reports the diff ratio and a diff artifact instead of passing on a tolerance.

### Requirement: Baselines are platform-specific and are updated deliberately
The baseline file names SHALL carry the browser project and the platform, and the
evidence document SHALL state the platform that generated the committed set and
that a different OS, GPU or font stack needs its own set. The documented update
command SHALL be `npm run test:e2e -- --update-snapshots`, and the documentation
SHALL state that updating is legitimate only for an intended visual change that a
human reviewed and that ships in the same change.

#### Scenario: The generating platform is named
Given `docs/evidence/visual-regression.md`, when it is read, then it names the browser, the platform and the baseline file suffix, and does not claim the images are portable.

#### Scenario: The update command is explicit
Given the README and the evidence document, when the update instruction is read, then it is the exact command that regenerates the baselines and it states when regeneration is legitimate.

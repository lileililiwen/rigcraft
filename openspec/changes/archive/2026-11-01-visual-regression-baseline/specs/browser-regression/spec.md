# browser-regression deltas

## ADDED Requirements

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

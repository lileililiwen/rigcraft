# Browser regression deltas

## ADDED Requirements

### Requirement: Keyboard focus assertions run against a rendered, frontmost page
A Playwright test that asserts the first keyboard focus target SHALL bring the
page to front and wait for the page heading to be visible before dispatching the
first key event. The assertion itself SHALL NOT be retried, loosened, skipped or
made conditional on the data state.

#### Scenario: The first Tab stop is the skip link
Given `/inventory` has rendered its heading, when the page is brought to front and Tab is pressed once, then the skip link has focus.

#### Scenario: Enter moves focus into the named main region
Given the skip link has focus, when Enter is pressed, then focus is on the `main` element whose accessible name is the page heading.

#### Scenario: The test does not depend on inventory data
Given a fresh or populated database, when the skip-link journey runs, then it makes the same assertions and does not require a seeded snapshot.

# ui-foundation deltas

## ADDED Requirements

### Requirement: An assistive-technology review is recorded, not claimed as a screen-reader pass
`docs/evidence/assistive-technology-review.md` SHALL record, for each of the six
client routes and from the running production build, the accessibility tree
(roles, names, landmarks and heading order), the focus order through the primary
journey (skip link, navigation, main control, primary action, result), every live
region with what it announces, and any role or name gap found. The document SHALL
state at the top that it is not a screen-reader pass. It SHALL list the exact
human script — browser and screen-reader pairing, route order, keys and expected
announcement — that must still be run with NVDA, VoiceOver and JAWS before any
screen-reader claim is made.

#### Scenario: Every route has a tree and a heading order
Given the review document, when a route is read, then it names that route's document title, its landmarks, its heading order and an accessibility-tree extract captured from the running bundle.

#### Scenario: The primary journey's focus order is recorded end to end
Given the recommendation route in the review, when the focus order is read, then it starts at the skip link, passes the primary navigation, the snapshot control and the budget field, reaches the primary action, and names the element that receives focus when the result renders.

#### Scenario: Live regions name their announcements
Given each route in the review, when the live regions are read, then each carries its politeness and the text it announced, including the announcement after the route's primary action.

#### Scenario: The screen-reader claim is refused and handed over
Given the top of the review document, when it is read, then it states plainly that this is an assistive-technology review and not a screen-reader pass, and the human script names the three screen readers and the steps and announcements to verify.

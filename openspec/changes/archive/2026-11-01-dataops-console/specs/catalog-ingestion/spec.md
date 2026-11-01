# catalog-ingestion deltas

## MODIFIED Requirements

### Requirement: The catalog page is usable, accessible and self-hosted
The `/catalog` route SHALL let the operator choose and upload the document with the
acknowledgement, show the accepted report or the refused rows with their reasons,
list the registry with licence and counts, activate or deactivate a dataset,
remove one only through a confirmation step, and search. The registry and the
search results SHALL be presented as dense console tables that name, per row, the
dataset and its provenance chip, the licence, the version, the part and category
counts, the source URL and the active state, and the registry SHALL summarise the
dataset, part and active counts. Search SHALL keep its rank order and the dataset
attribution of every hit and SHALL be disabled, with an accessible reason, while
the query is empty. The page SHALL use labelled controls, visible focus, live status
announcements, non-colour-only statuses, a desktop layout and a stacked narrow
layout. The `/inventory` review SHALL show each row's catalog status and the
per-row opt-in, and SHALL mark catalog-applied fields.

#### Scenario: Upload and see the registry
Given a small valid document, when it is uploaded through `/catalog`, then the page shows the accepted report and the dataset in the registry with its licence, version, counts, source URL and active state.

#### Scenario: Refused document shows the reason
Given a document with an invalid row, when it is uploaded, then the page shows the refused state with the row index and field reason and the registry is unchanged.

#### Scenario: Apply a proposal on the import review
Given a preview row that matches an active dataset and has a missing specification, when the operator checks the per-row opt-in and commits, then the committed snapshot shows the added specification with catalog provenance and the operator's own values unchanged.

#### Scenario: The empty query cannot be submitted
Given an empty search field, when the search panel renders, then `Search catalog` is disabled and its reason names the query, and a non-empty query enables it.

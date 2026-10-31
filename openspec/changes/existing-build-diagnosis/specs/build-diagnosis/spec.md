# Diagnose an existing or proposed PC build

## ADDED Requirements

### Requirement: build-diagnosis provides a sourced diagnosis of reusable parts, conflicts and an incremental replacement path
The system SHALL deliver a sourced diagnosis of reusable parts, conflicts and an incremental replacement path through /diagnose, using POST /api/diagnoses/parse accepts pasted text and returns candidate SKU matches/unknown lines; POST /api/diagnoses/check accepts confirmed parts and returns compatibility report, keep/replace candidates and unknowns. No guessed match is auto-confirmed. It SHALL preserve this invariant: Never infer exact SKU or BIOS revision from partial labels. Keep part only when it satisfies requested constraints; unknown support remains a verification task. Replacement cost excludes retained owned parts.

#### Scenario: Disambiguate list
Given a pasted list with two plausible SKU matches, when parsing finishes, then the UI requires the user to choose or leave the part unknown.

#### Scenario: Reuse owned part
Given a verified reusable SSD, when diagnosing, then the replacement path retains it with zero incremental purchase cost.

#### Scenario: Explain blocked upgrade
Given incompatible RAM and uncertain BIOS, when checked, then the RAM conflict and BIOS unknown appear separately with replacement and confirmation actions.

# Check complete build compatibility

## ADDED Requirements

### Requirement: compatibility-evidence provides a three-state result with rule-level evidence and specific unknowns
The system SHALL deliver a three-state result with rule-level evidence and specific unknowns through /builds/check, using POST /api/builds/check accepts snapshotId and selected SKU/quantity pairs; returns overallStatus, completeness, rules[] with status, field values, source/version and followUp. 400 for malformed selection, 404 for unknown snapshot, 422 for absent SKU. It SHALL preserve this invariant: Conflict overrides unknown; unknown overrides compatible. Required coverage includes socket/BIOS, RAM, GPU and cooler clearance, board/case, PSU wattage/connectors and storage interface. iGPU or bundled cooling counts only with sourced evidence. PSU estimate is screening, never proof.

#### Scenario: Definite conflict
Given different CPU and board sockets, when the user checks the build, then overall status is Incompatible and the socket rule exposes its compared values and sources.

#### Scenario: Missing BIOS evidence
Given matching sockets without verified BIOS support, when checked, then overall status is Unknown and the UI asks which BIOS support fact to confirm.

#### Scenario: Recheck after edit
Given a checked build, when any part or referenced fact changes, then the old verdict is marked stale until a new check completes.

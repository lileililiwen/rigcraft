# data-portability deltas

## ADDED Requirements

### Requirement: Backup refusal prose uses grammatical plural forms
A `backup_wrong_shape` refusal that names missing SQLite tables SHALL spell its
count with the singular noun only for exactly one and the plural noun otherwise,
so one missing table reads `missing 1 table:` and two read `2 tables`. The
refusal code, its HTTP status and `details` shape SHALL NOT change, and no
backup-refusal sentence SHALL contain the `(s)` pseudo-plural.

#### Scenario: One missing table is named
Given an upload that is otherwise a RigCraft database but lacks one required table, when restore is attempted, then the refusal is `backup_wrong_shape` and `details.reason` reads `missing 1 table:` followed by the table name.

#### Scenario: Several missing tables are named
Given an upload that lacks two required tables, when restore is attempted, then `details.reason` reads `2 tables` and names both, and the live catalog is unchanged.

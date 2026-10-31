# Recommend and compare inventory builds

## ADDED Requirements

### Requirement: build-recommendation provides up to three reproducible, budget- and stock-valid builds with comparison and export
The system SHALL deliver up to three reproducible, budget- and stock-valid builds with comparison and export through /recommend, using POST /api/recommendations accepts budgetMinor, currency, useCase, inventoryOnly, locked/excluded SKUs and preferences; returns ranked candidates or reason codes. GET /api/recommendations/{id}/export returns versioned JSON. Reject currency mismatch and nonpositive budget with 400. It SHALL preserve this invariant: Hard constraints apply before scoring. Candidate count is 0–3 and distinct; incompatible or incomplete builds are excluded, unknown builds labelled. Deterministic ordering uses score then total price then SKU list. Owned parts have explicit zero incremental price and are excluded from stock consumption.

#### Scenario: Recommend valid options
Given a feasible inventory snapshot and budget, when the user generates, then every result is complete, within budget, in stock and compatible or visibly Unknown.

#### Scenario: No solution
Given insufficient stock or budget, when generation finishes, then no invalid build is shown and the blocking reason identifies an actionable constraint.

#### Scenario: Repeat and export
Given unchanged snapshot, rules and scoring, when the request repeats, then ordering matches and export records all version IDs, price and unknown evidence.

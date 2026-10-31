# Parse and revise build requests with BYOK

## ADDED Requirements

### Requirement: ai-planning provides a confirmed structured request or local revision that is rechecked by the deterministic pipeline
The system SHALL deliver a confirmed structured request or local revision that is rechecked by the deterministic pipeline through /recommend assistant panel, using POST /api/ai/parse accepts key, baseUrl, model and text; returns proposed structured fields plus ambiguities; POST /api/ai/revise accepts current request and instruction and returns a proposed diff. The user confirms before normal recommendation API runs. Provider timeout 20 s, no automatic retry. It SHALL preserve this invariant: AI cannot write compatibility or price fields. Missing budget/currency/locked parts require explicit confirmation. A revision changes only fields in the confirmed diff. No key means standard form remains fully usable.

#### Scenario: Confirm parsed intent
Given a user-provided key and text, when parse returns fields, then the UI presents a diff for correction before recommendation runs.

#### Scenario: Reject fabricated fact
Given a model response with a compatibility verdict or price, when parsed, then those fields are ignored and the deterministic pipeline supplies the final result.

#### Scenario: Remain usable offline
Given no key or provider failure, when the user opens the page, then the structured form and saved recommendations still work.

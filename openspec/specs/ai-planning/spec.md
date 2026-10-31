# ai-planning Specification

## Purpose
Define the optional bring-your-own-key interpretation layer: no bundled key or default provider, a session-only credential that is never persisted, logged, echoed or exported, an interpreter that only proposes `budgetMinor`, `currency`, `useCase`, `inventoryOnly`, `lockedSkus` and `excludedSkus` with a traceable source per field, explicit confirmation before the deterministic engine runs, field-level revisions, graceful no-key and provider-failure paths, a bounded and disclosed prompt with an HTTPS/loopback-only provider URL, and an accessible `/recommend` assistant panel.
## Requirements
### Requirement: Natural-language assistance is optional and brings its own key
The system SHALL provide natural-language assistance only as an optional layer over the existing deterministic recommendation flow. It SHALL NOT ship, embed, default to or discover any provider key, vendor account or cloud provider. The application SHALL remain fully usable — import, snapshot, compatibility check, recommendation, comparison and export — with no key configured. A request that requires provider interpretation without a key SHALL fail with HTTP 400 `ai_key_required` and a message that names the deterministic flow, never with a dead end.

#### Scenario: No key configured
Given an operator who never entered a provider key, when the recommendation page loads, then every deterministic control is available, the assistant panel states that no key is configured, and no network request is made to any provider.

#### Scenario: A missing key is not replaced by a bundled default
Given a parse request with an empty key, when it is submitted, then the service returns HTTP 400 `ai_key_required`, uses no built-in credential, and contacts no provider.

### Requirement: Provider credentials are session-only and never persisted or echoed
The system SHALL hold the provider key in memory only, for the current browser session and the current request. It SHALL NOT write the key to `localStorage`, `sessionStorage`, cookies, SQLite, a log, an export, an error response, a successful response or an evidence file. The backend SHALL NOT persist the key, the prompt or the model response, and SHALL NOT include the key in any serialized response. The key SHALL be sent only to the provider URL the operator entered. Redacted debugging SHALL print a fixed marker rather than the value. A repository test SHALL scan the service source and the HTTP surface for key leakage.

#### Scenario: Key never appears in a response, a log or the database
Given a parse request carrying a distinctive sentinel key, when it completes or fails, then no response body, response header, log line, export or SQLite page contains the sentinel, and the database has no AI table or column.

#### Scenario: Key is sent only to the configured provider
Given a parse request, when the adapter runs, then the credential appears exactly once, in the request to the operator's provider URL, and no other outbound call is made.

### Requirement: Interpretation is grounded and never invents facts
The system SHALL interpret a sentence into a proposal containing only `budgetMinor`, `currency`, `useCase`, `inventoryOnly`, `lockedSkus` and `excludedSkus`. Each field SHALL carry its source: `sentence`, `snapshot`, `default` or `unresolved`. A `sentence` source SHALL carry a non-empty excerpt that is a literal substring of the submitted sentence after whitespace normalisation; a field that fails this check SHALL become `unresolved` and raise an ambiguity instead of being accepted. A `snapshot` source SHALL only be accepted when it equals the value derived from the selected snapshot. Only two values may be a `default`: `inventoryOnly` when the sentence is silent (inventory-only mode is always on), and an empty locked or excluded list when the sentence named no such part. Each SHALL be labelled as a default, and a revision SHALL leave the confirmed value unchanged for a defaulted field. A locked or excluded SKU SHALL exist in the selected snapshot; a name that does not SHALL be dropped and reported. Any field the model returns outside the allowed set SHALL be discarded and listed. The proposal SHALL NOT contain prices, stock, specifications or compatibility verdicts, and the interpreter SHALL NOT write any of them. A missing budget, currency or lock decision SHALL be reported as an ambiguity for explicit confirmation rather than filled in.

#### Scenario: An ungrounded value is rejected
Given a model reply proposing a budget whose excerpt does not appear in the sentence, when the proposal is built, then the budget is `unresolved`, an ambiguity names it, and no request is proposed with that budget.

#### Scenario: An invented part name is dropped
Given a model reply naming a SKU that is absent from the selected snapshot, when the proposal is built, then the SKU is removed from the proposal, an ambiguity names it, and no fabricated part reaches the request.

#### Scenario: A default is labelled
Given a sentence that does not mention inventory-only mode, when the proposal is built, then `inventoryOnly` is true with source `default` and the interface shows it as a default.

### Requirement: Nothing runs before explicit confirmation
`POST /api/ai/parse` SHALL accept `text`, `snapshotId`, `apiKey`, `baseUrl`, `model` and `includeInventorySummary`, and SHALL return a proposal with its per-field sources, ambiguities, discarded fields, the provider identity, a prompt version and `confirmRequired: true`. It SHALL NOT run a recommendation, mutate a snapshot, reserve stock, or persist a report. The proposed request SHALL be produced only when every hard field is resolved, and it SHALL be valid for `POST /api/recommendations`, which remains the only path that produces results and re-validates every constraint. `POST /api/ai/revise` SHALL accept a current confirmed request plus an instruction and SHALL return the proposal with a field-level `diff`; applying the revision SHALL change only fields named in that diff and SHALL preserve every other confirmed field.

#### Scenario: Parsing does not recommend
Given a successful parse that yields a complete proposed request, when the operator stops before confirming, then no recommendation report exists, the snapshot totals are unchanged and no file was written.

#### Scenario: A revision is limited to its diff
Given a confirmed request with a gaming use case and one locked part, when the instruction asks only to raise the budget to 1,500.00, then the diff names only `budgetMinor`, and the applied request keeps the use case and the lock.

### Requirement: No-key and provider failure degrade to the deterministic flow
When no key is configured, the provider is unreachable, the provider times out after 20 seconds, the provider returns a non-success status or the reply cannot be parsed, the system SHALL report a stable field-coded failure, SHALL NOT retry automatically, SHALL preserve the operator's sentence and deterministic form values, and SHALL offer the deterministic flow as the recovery path. Provider timeout SHALL be HTTP 504 `ai_provider_timeout`; an unreachable provider SHALL be HTTP 502 `ai_provider_unreachable`; a non-success provider status SHALL be HTTP 502 `ai_provider_error`; an unusable reply SHALL be HTTP 422 `ai_response_invalid`. No error message SHALL include the provider response body or the credential.

#### Scenario: Provider timeout keeps the operator's work
Given the operator's provider does not answer within 20 seconds, when the parse request fails, then the response is HTTP 504 `ai_provider_timeout`, the sentence and form values are still present in the interface, and a retry action and the deterministic form are offered.

#### Scenario: Deterministic recommendation still works after a provider failure
Given a failed parse attempt, when the operator generates recommendations through the form, then the normal recommendation contract runs and returns its usual result.

### Requirement: Prompt scope and provider URL are bounded and disclosed
The system SHALL limit the submitted sentence and the built prompt to 8 KiB, and SHALL reject an oversized sentence with HTTP 400 `ai_text_too_large` for field `text` without contacting the provider. The prompt SHALL contain the sentence, the snapshot currency, the allowed use cases, and — only when the operator explicitly opts in — a bounded part summary of at most 50 records limited to SKU, category, model, price and quantity. The interface SHALL state exactly which fields leave the machine and whether the part summary is included. The provider URL SHALL be HTTPS, except plain HTTP for a loopback host, and SHALL be rejected with HTTP 400 `ai_provider_url_invalid` when it uses another scheme, embeds credentials, carries a query or fragment, or targets a non-loopback private address. Redirects SHALL NOT be followed.

#### Scenario: Oversized sentence is refused before any network call
Given a sentence larger than 8 KiB, when it is parsed, then the service returns HTTP 400 `ai_text_too_large`, the provider mock records zero calls and no prompt is built.

#### Scenario: A plain-HTTP remote provider is refused
Given a provider URL such as `http://198.51.100.7/v1`, when it is parsed, then the service returns HTTP 400 `ai_provider_url_invalid` and no connection is attempted.

#### Scenario: The inventory is not sent by default
Given a parse request without the inventory-summary opt-in, when the prompt is built, then it contains no SKU, price or quantity from the snapshot.

### Requirement: The assistant Web flow is keyboard and screen-reader usable
The `/recommend` assistant panel SHALL let the operator enter a key, a provider URL, a model and a sentence, read what leaves the machine, consent, parse, read a per-field proposal with its source, edit it, confirm or cancel, and continue without AI. It SHALL cover the disabled, no-key, consent, parsing, proposal, ambiguity, provider-error and confirmed states. It SHALL keep the sentence on failure, announce parsing, success, failure and confirmation in a live region, move focus to the proposal heading and to the error alert, use text rather than colour alone, reach every control by keyboard with a visible focus ring, never render the key back as readable text, and stack at phone width without horizontal page overflow.

#### Scenario: Keyboard-only parse, edit and confirm
Given the recommendation page at phone width, when the operator uses Tab and Enter only, then the key, URL, model, consent, sentence, parse, per-field edits, confirm and cancel controls are reachable with visible focus, the confirmed values reach the deterministic form, and the page has no horizontal overflow.

#### Scenario: The proposal states where each value came from
Given a confirmed proposal, when it is displayed, then every field shows its source as the sentence excerpt, the snapshot, a default or an unresolved clarification, and an edited field is marked as edited.


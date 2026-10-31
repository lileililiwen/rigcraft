# BYOK natural-language planning

Status: implemented in `byok-natural-language-planning` (2026-10-31).

This package delivers the optional, bring-your-own-key interpretation layer required by
`requirement.md` §5 (the P1 AI boundary). It turns one sentence into a *proposed*
recommendation request, shows every field with its source, and runs nothing until the
operator confirms. The deterministic recommendation contract stays the only path that
produces results, prices or verdicts. Build diagnosis, model-authored verdicts,
autonomous loops and provider-specific SDKs remain out of scope.

## Runtime

| Target | Command | Notes |
| --- | --- | --- |
| Rust API | `cargo run` | Same process and SQLite file as inventory, compatibility and recommendation. |
| Web build | `npm run build` | The assistant panel is part of `/recommend`. |
| Tests | `cargo test`, `npm test` | Transport, contract, grounding, hygiene and component tests. |
| Loopback provider stub | `python3 scripts/mock-openai-provider.py 8799` | Evidence and manual testing only; never contacts a network. |

No provider is configured by default and no key is bundled. Without a key the page is
fully usable; the assistant panel states that and offers **Continue without AI**.

## Secrets policy

The key is **session-only, in memory**:

- The browser holds it in React state for the current tab. It is never written to
  `localStorage`, `sessionStorage`, `document.cookie` or IndexedDB, and a reload clears
  it. `tests/ai_secret_hygiene.rs` fails if browser storage appears in `apps/web/src`.
- The server reads it from the request body into `SecretKey`, whose `Debug` prints
  `[redacted]`. The raw value is read in exactly one place
  (`SecretKey::expose_to_provider`) and placed directly in the outbound
  `Authorization` header.
- The inbound request types (`ParseRequest`, `ReviseRequest`) do not derive `Serialize`,
  so a response cannot echo them; a test guards the declaration.
- Nothing about a request is logged: the only log line records the operation and the
  number of inventory records in the prompt. `tests/ai_secret_hygiene.rs` fails if a
  logging or panic line mentions a credential.
- Nothing about a request is persisted: no AI table, column or row exists. Schema
  `PRAGMA user_version` stays `3`. The confirmed request is persisted only by the
  existing recommendation contract.

OS keychains were considered and rejected: they add a platform dependency and a
second copy of the secret without changing the threat model for a loopback service.
Session-only memory is the documented, enforced choice.

## HTTP contract

All JSON uses camelCase. Errors keep the inventory shape:

```json
{ "error": { "code": "ai_key_required", "message": "...", "field": "apiKey" } }
```

### `POST /api/ai/parse`

```json
{
  "text": "Build a $2000 USD gaming PC from my stock and keep FIX-R-CPU-A.",
  "snapshotId": "uuid",
  "apiKey": "sk-...",
  "baseUrl": "https://provider.example/v1",
  "model": "provider-model-name",
  "includeInventorySummary": false
}
```

`baseUrl` is the OpenAI-compatible base; the adapter posts to
`{baseUrl}/chat/completions` with `temperature: 0`.

Response:

```text
mode: "provider"
promptVersion: "ai-parse-v1"
provider { kind, baseUrl, model }
proposal {
  budgetMinor   { value, source, excerpt?, note? },
  currency      { ... }, useCase { ... }, inventoryOnly { ... },
  lockedSkus    { value: [sku], ... }, excludedSkus { value: [sku], ... }
}
ambiguities[] { field, code, message }
ignoredFields[]            // model output outside the contract
sent { sentence, currency, useCases[], inventoryFields[], inventoryRecords }
confirmRequired: true
request | absent           // a proposal only; never executed here
```

`source` is one of:

| Source | Meaning | Proof required |
| --- | --- | --- |
| `sentence` | Copied from the operator's sentence | `excerpt` is a literal substring after whitespace normalisation |
| `snapshot` | Derived from the selected snapshot (its currency, or a SKU that exists in it) | The value is a fact of the snapshot |
| `default` | A stated default: inventory-only mode, or "the sentence named no part" | Shown in the interface as a default |
| `unresolved` | Could not be established | Listed in `ambiguities`; the operator must supply it |

A `sentence` value whose excerpt is absent, empty or not a substring becomes
`unresolved` and raises `ungrounded_value`. A locked or excluded SKU that is not in the
snapshot is dropped and reported as `unknown_sku`. A part named neither in the sentence
nor in the included part list is dropped as `ungrounded_value`. A part that is both kept
and excluded is kept, and the exclusion is reported as `locked_excluded_conflict`. Two
kept parts in one category are reported as `locked_category_conflict`. A sentence asking
for catalog mode is reported as `catalog_unavailable` and never produces a request.

`request` is present only when `budgetMinor`, `currency` and `useCase` are resolved and
the part lists are resolved. It is a valid `RecommendRequest`; `POST /api/recommendations`
re-validates every field, so the AI layer cannot bypass validation or invent inventory.

### `POST /api/ai/revise`

Same transport fields plus `instruction` and `current`
(`budgetMinor`, `currency`, `useCase`, `lockedSkus`, `excludedSkus`). The response is the
same shape plus `diff[] { field, from, to }`. Only fields present in `diff` change when
the revision is applied; every other confirmed field is preserved, and a resolved field
in `proposal` that is absent from `diff` was already equal.

### Error codes

| Status | Code | When |
| --- | --- | --- |
| 400 | `ai_text_required` / `ai_text_too_large` | Empty or over 8 KiB, before any network call |
| 400 | `ai_key_required` | No key supplied; the message names the deterministic form |
| 400 | `ai_model_required` | No model name |
| 400 | `ai_provider_url_invalid` | Not HTTPS, or plain HTTP to a non-loopback host; credentials, query, fragment or private-network target |
| 400 | `ai_prompt_too_large` | The built prompt would exceed 8 KiB |
| 404 | `snapshot_not_found` | `snapshotId` does not exist |
| 422 | `ai_response_invalid` | The reply is not an OpenAI chat-completions envelope or contains no JSON object |
| 502 | `ai_provider_error` | Non-success status, or a redirect (not followed) |
| 502 | `ai_provider_unreachable` | Connection failure |
| 504 | `ai_provider_timeout` | No answer within 20 seconds |

There is no automatic retry. No error message includes the provider response body or the
credential.

## Prompt scope

- The sentence and the serialized prompt are capped at 8 KiB.
- The default prompt carries the sentence, the snapshot currency and the allowed
  use-case names. No SKU, price or quantity leaves the machine.
- With `includeInventorySummary`, at most 50 records are added, limited to
  `sku`, `category`, `model`, `priceMinor` and `quantity`. The response reports the exact
  records that were included, and the panel states which fields leave the machine before
  the request is sent.
- Specs, provenance, cost and every other column are never included.

## Web flow

`/recommend` section 2 is the assistant panel. States: no key, consent, parsing,
proposal, ambiguity, edited field, confirmed, provider error. The panel:

- requires an explicit consent checkbox before parsing;
- states exactly what leaves the machine and whether the part list is included;
- shows every proposed field with a symbol and text source, then marks edits
  `· Edited by you`;
- applies a confirmed proposal to the deterministic form and focuses
  **Generate recommendations**; it never runs the search itself;
- keeps the sentence, key and form values on failure and offers retry and
  **Continue without AI**;
- renders the key as a password field and never reprints it;
- has no horizontal overflow at 390 px and is fully keyboard operable.

## Verification

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test                       # includes tests/ai_planning_api.rs,
                                 # tests/ai_provider_transport.rs and
                                 # tests/ai_secret_hygiene.rs
npm run check && npm test && npm run build
./verify-local.sh
```

`tests/ai_provider_transport.rs` exercises the real adapter against a loopback HTTP
server, so the transport, its URL policy and its failure mapping are covered without
external network access. Evidence: `docs/evidence/byok-planning-browser.md`.

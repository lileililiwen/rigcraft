# Assistive-technology gap evidence (2026-11-01)

`docs/evidence/assistive-technology-review.md` recorded five role and name gaps
from the running build and deliberately did not fix them. This change fixes four
and records the fifth as a human decision. Nothing here changes a pixel, a
sentence, a route, an API shape, a schema or a dependency, and the twelve
committed visual baselines were not regenerated.

## What was measured, and how

- Production build: `target/debug/rigcraft` serving `apps/web/dist`
  (`index-B-c8V-0n.css` 27,698 bytes, `index-DcR4563W.js` 308,449 bytes) on
  `127.0.0.1:33411` with a scratch data directory.
- Seed: `GET /api/imports/sample`, `POST /api/imports` (currency `USD`), then the
  commit with every row accepted. Snapshot
  `86173b61-1bf3-4147-a0e7-c2a7e91e1950`, revision 1, 10 records,
  US$2,120.90 stock value. No other record was written.
- Playwright 1.64.1 driving Google Chrome 154.0.8037.97, headless, 1440×900,
  `locale: en-US`, `timezoneId: UTC`.
- `locator("body").ariaSnapshot()` after `networkidle`, one call per route.
  `/recommend` was captured with the assistant disclosure opened, because a
  closed `<details>` is not in the accessibility tree.

An accessibility tree is what an assistive technology *should* receive. It is not
a transcript of what a screen reader *does* receive. The human script in the
review document is still the open item.

## Names added

| Route | What was unnamed | Accessible name now | How |
| --- | --- | --- | --- |
| `/inventory` | import panel (a `<section>`, no `<form>`) | `Import a CSV` | real `<form>`, `aria-labelledby="import-title"` |
| `/inventory` | restore inspection form | `Restore from a backup` | `aria-labelledby` to the existing `h3` |
| `/inventory` | restore confirmation form | `Confirm replacing the inventory` | `aria-label` (no heading names it) |
| `/inventory` | preview row correction form | `Correct row N` | `aria-labelledby` to the existing `h3` |
| `/inventory` | record correction form | `Create a new revision from <SKU>` | `aria-labelledby` to the existing `h3` |
| `/catalog` | upload form | `Upload a catalog` | `aria-labelledby="upload-title"` |
| `/catalog` | search form | `Search` | `aria-labelledby="search-title"` |
| `/builds/check` | parts form | `2. Select parts` | `aria-labelledby="parts-title"` |
| `/recommend` | budget form | `3. Budget and constraints` | `aria-labelledby="request-form-title"` |
| `/recommend` | assistant transport form | `Assistant provider settings` | `aria-label` (no heading names it) |
| every route | `main#main` | the page `h1` text | `aria-labelledby` to `h1` `id` |

The catalog upload result heading is now `h3#catalog-result-title`, and the
export/backup line on `/` and `/inventory` is `role="note"` with the action
results announced by a visually hidden polite region beside it.

## `/inventory` accessibility tree

Verbatim `ariaSnapshot()` of `body`, with only the ten seeded record rows elided
(the elision is marked in place). It proves the named `main`, the named import and
restore forms, and that the export sentence is a `note` beside an empty `status`
rather than a live region.

```
- link "Skip to inventory":
  - /url: "#main"
- banner:
  - link "RigCraft":
    - /url: /inventory
  - navigation "Primary":
    - link "Inventory":
      - /url: /inventory
    - link "Catalog":
      - /url: /catalog
    - link "Compatibility":
      - /url: /builds/check
    - link "Recommend":
      - /url: /recommend
    - link "Diagnose":
      - /url: /diagnose
- main "Inventory":
  - heading "Inventory" [level=1]
  - paragraph: Import owned hardware from a UTF-8 CSV, correct every bad row, and commit a versioned snapshot. Each value keeps its original input, normalized value, and provenance.
  - paragraph: Inventory is private to this local service. It is not uploaded to a catalog, shared with an AI provider, or logged.
  - status: "Opened the newest snapshot, revision 1: 10 records, US$2,120.90 stock value. Export and backup are ready below."
  - region "Snapshot state":
    - text: Latest revision
    - heading "Snapshot state" [level=2]
    - term: Revision
    - definition: "1"
    - term: Records
    - definition: "10"
    - term: Units in stock
    - definition: "10"
    - term: Stock value
    - definition: US$2,120.90
    - paragraph:
      - text: Source rigcraft-inventory-sample.csv · committed
      - time: 2026-11-01T08:33:19.627Z
  - region "Committed snapshot · revision 1":
    - heading "Committed snapshot · revision 1" [level=2]
    - paragraph: The snapshot is immutable. A later correction creates a new revision and keeps this one readable.
    - term: Snapshot ID
    - definition: 86173b61…1950
    - term: Committed
    - definition:
      - time: 2026-11-01T08:33:19.627Z
    - term: Currency
    - definition: USD
    - term: Source file
    - definition: rigcraft-inventory-sample.csv · sha256 2486517c5b49…
    - term: Parent revision
    - definition: None (first revision)
    - term: Note
    - definition: —
    - term: Records
    - definition: "10"
    - term: Units in stock
    - definition: "10"
    - term: Stock value
    - definition: US$2,120.90
    - table "Committed stock records with provenance and unknown specs":
      - caption: Committed stock records with provenance and unknown specs
      - rowgroup:
        - row "Part Category Model Stock Price Specifications Provenance Action":
          - columnheader "Part"
          - columnheader "Category"
          - columnheader "Model"
          - columnheader "Stock"
          - columnheader "Price"
          - columnheader "Specifications"
          - columnheader "Provenance"
          - columnheader "Action"
      … ten record rows (one per seeded part), unchanged by this change …
  - button "Start a new import"
  - form "Import a CSV":
    - heading "Import a CSV" [level=2]
    - paragraph:
      - text: One batch uses one currency. Prices are stored in minor units (for example 189.00 becomes 18900). Specifications use
      - code: key=value
      - text: pairs separated by semicolons, such as
      - code: socket=AM5;tdp=105W
      - text: .
    - text: Inventory CSV (UTF-8)
    - button "Inventory CSV (UTF-8)"
    - paragraph: Choose a CSV with at least sku, category, model, quantity and price columns.
    - text: Batch currency
    - combobox "Batch currency": USD
    - paragraph: A three-letter ISO 4217 code. Every row must match this currency.
    - paragraph: Choose a CSV file to enable Preview CSV.
    - button "Preview CSV" [disabled]
    - link "Download sample CSV":
      - /url: /api/imports/sample
    - paragraph:
      - text: The sample is one real AM5 build. Its compatibility facts were checked on 2026-10-31 against the manufacturer pages recorded in the file's
      - code: sourceUrl
      - text: column. Sample data is CC BY 4.0 (see NOTICE); its quantity and price are operator-entered stock values, not market data.
    - paragraph: Files are sent only to this local RigCraft service. CSV contents and optional cost fields are not written to logs.
    - paragraph: Need a row removed from the file? Preview first, then choose Exclude or correct the row before committing.
  - region "Snapshot history":
    - heading "Snapshot history" [level=2]
    - paragraph: Every revision stays readable. Opening an older one shows its exact records and provenance.
    - list:
      - listitem:
        - heading "Revision 1" [level=3]
        - text: Viewing now
        - term: Committed
        - definition:
          - time: 2026-11-01T08:33:19.627Z
        - term: Records
        - definition: "10"
        - term: Stock value
        - definition: US$2,120.90
        - button "View 86173b61…1950"
  - region "Export and backup":
    - heading "Export and backup" [level=2]
    - paragraph: Take the catalog with you, or put it back. JSON is the authoritative export; CSV uses the import columns and is lossy. A backup is a standard SQLite file and contains the shop's entire private inventory.
    - note: Exports and backups are created by this local service only.
    - status
    - heading "Export this snapshot" [level=3]
    - paragraph: "CSV carries the import columns only: no lot identity, snapshot metadata or per-field provenance reference. JSON keeps everything."
    - button "Download CSV"
    - button "Download JSON"
    - paragraph: "Ready: revision 1, 10 records."
    - heading "Back up this database" [level=3]
    - paragraph: One consistent copy of every snapshot, correction and derived report, as a plain SQLite file any SQLite tool can inspect.
    - button "Download backup"
    - form "Restore from a backup":
      - heading "Restore from a backup" [level=3]
      - paragraph: Inspecting reads the file without changing anything. Replacing is a separate, explicitly confirmed step.
      - text: SQLite backup file
      - button "SQLite backup file"
      - paragraph: Choose a RigCraft backup or another RigCraft SQLite catalog.
      - button "Inspect backup" [disabled]
```

## `/recommend` accessibility tree

Verbatim `ariaSnapshot()` of `body` with the assistant disclosure opened. It
proves the named `main`, the named assistant transport form and the named budget
form.

```
- link "Skip to recommendation":
  - /url: "#main"
- banner:
  - link "RigCraft":
    - /url: /inventory
  - navigation "Primary":
    - link "Inventory":
      - /url: /inventory
    - link "Catalog":
      - /url: /catalog
    - link "Compatibility":
      - /url: /builds/check
    - link "Recommend":
      - /url: /recommend
    - link "Diagnose":
      - /url: /diagnose
- main "Build recommendation":
  - heading "Build recommendation" [level=1]
  - paragraph: Enter a total budget and what the machine is for. RigCraft searches only your inventory, keeps every part within available stock, and checks every candidate with the compatibility rules before it is shown. No conflict is recommended; an unknown result is labelled as unverified.
  - paragraph: The search runs locally with no hardware catalog. The assistant panel is optional, needs your own provider key, and only proposes the request you confirm. Searches are deterministic and bounded, and generating a plan never reserves stock.
  - status: "Snapshot revision 1 loaded: 10 records, currency USD."
  - region "1. Choose an inventory snapshot":
    - heading "1. Choose an inventory snapshot" [level=2]
    - text: Snapshot
    - combobox "Snapshot":
      - option "Choose a snapshot"
      - option "Revision 1 · rigcraft-inventory-sample.csv · 2026-11-01T08:33:19.627Z · 10 records · 86173b61…1950" [selected]
    - paragraph: Snapshot 86173b61…1950 · revision 1 · USD · source rigcraft-inventory-sample.csv
  - group "2. Ask in your own words (optional)":
    - heading "2. Ask in your own words (optional)" [level=2]
    - paragraph: Describe the build in one sentence. Your own provider turns it into a proposed request that you review and confirm; the deterministic search below still checks every part, price and compatibility rule.
    - heading "No key configured" [level=3]
    - paragraph:
      - text: The assistant needs your own provider key and is switched off until you enter one. Everything on this page — budget, constraints, generate, compare and export — works without it. Use
      - strong: Continue without AI
      - text: below to go straight to the form.
    - form "Assistant provider settings":
      - text: Provider key
      - textbox "Provider key"
      - paragraph: Kept in this tab for this session only. Never stored, logged, exported or shown again.
      - text: Provider base URL
      - textbox "Provider base URL":
        - /placeholder: https://provider.example/v1
      - paragraph: HTTPS, or plain HTTP only for a loopback model such as http://127.0.0.1:11434/v1. Redirects are not followed.
      - text: Model name
      - textbox "Model name"
      - paragraph: Whatever your provider calls the model.
    - heading "What leaves this machine" [level=3]
    - paragraph:
      - text: Your sentence, the snapshot currency (USD) and the allowed use-case names go to the provider URL you typed. Your inventory is
      - strong: not
      - text: sent unless you include it below.
    - 'checkbox "Include a part list: at most 50 records, only SKU, category, model, price and quantity."'
    - text: "Include a part list: at most 50 records, only SKU, category, model, price and quantity."
    - paragraph: Your key is used for this request only. It is never saved, never written to a log or an export, and never sent anywhere except the URL above.
    - text: Your sentence
    - textbox "Your sentence":
      - /placeholder: A $1200 gaming build from my stock, keep the board I already have.
    - checkbox "I understand this sends my sentence, the fields above and my key to the provider URL I entered."
    - text: I understand this sends my sentence, the fields above and my key to the provider URL I entered.
    - button "Interpret sentence" [disabled]
    - button "Continue without AI"
    - paragraph: Enter your own provider key to use the assistant.
  - form "3. Budget and constraints":
    - region "3. Budget and constraints":
      - heading "3. Budget and constraints" [level=2]
      - text: Total budget (USD)
      - textbox "Total budget (USD)":
        - /placeholder: "1200.00"
      - paragraph: "One currency per batch: USD. A positive amount with at most two decimals; a comma or a space may group thousands, as in 1,800 or 1,800.50."
      - text: Primary use
      - combobox "Primary use":
        - option "Gaming" [selected]
        - option "Office and general use"
        - option "Creation and heavy workloads"
      - paragraph: Gaming requires a discrete graphics card in every option. Office accepts a CPU with sourced integrated graphics. Creation does not require a discrete card.
      - paragraph: "Inventory-only mode is on. A part is never added from outside this snapshot, and a locked part is treated as already owned: zero incremental price, no stock consumption."
      - group "Already owned parts (locked)":
        - text: Already owned parts (locked)
        - paragraph: Optional. A locked part must exist in this snapshot; it appears in every build at zero cost.
        - group: Choose owned parts (0 parts selected)
      - group "Parts to exclude":
        - text: Parts to exclude
        - paragraph: Optional. Excluded parts are never chosen and are listed in the report with the reason.
        - group: Choose excluded parts (0 parts selected)
      - button "Generate recommendations" [disabled]
      - text: 0 locked · 0 excluded
      - paragraph: Generate stays disabled until the budget is a positive amount with at most two decimals.
  - region "4. Read and compare options":
    - heading "4. Read and compare options" [level=2]
    - paragraph: Enter a budget and generate. Each option states why it was chosen, its cost and stock use, every rule verdict, what was excluded, and exactly which fact to confirm when evidence is missing.
  - complementary "Verdict and score":
    - heading "Verdict and score" [level=2]
    - paragraph: Generate a recommendation to read its status, budget delta, stock use, score and rule matrix here.
```

## What the trees prove

- `main "Inventory"` and `main "Build recommendation"` — the `main` landmark now
  carries the page name, and there is still exactly one `main` per route.
- `form "Import a CSV"`, `form "Restore from a backup"`, `form "Assistant
  provider settings"` and `form "3. Budget and constraints"` — a `<form>` is
  exposed with role `form` only when it has an accessible name, so each of these
  entries is the accessible name in the tree, not a CSS selector.
- `note: Exports and backups are created by this local service only.` followed by
  an empty `status` — the static sentence is not a live region, and the region
  that will carry a download, inspection or restore result exists and is empty on
  load.
- The forms that appear only after an interaction (`Correct row N`, `Create a new
  revision from <SKU>`, `Confirm replacing the inventory`) and the catalog result
  heading's `id` are proved by the component tests and the browser suite rather
  than by these two captures.

## What these captures do not establish

- Not a screen-reader pass. No NVDA, VoiceOver or JAWS output was captured, and
  nothing here says how the names are pronounced or how verbose the reading order
  is. The script at the end of
  [`assistive-technology-review.md`](assistive-technology-review.md) is still the
  open item.
- The `aria-live="off"` `/builds/check` selection counter is unchanged. It stays
  silent, and whether that costs a keyboard operator context is left to the
  human pass.
- No forced-colours, high-contrast or mobile screen-reader behaviour was
  exercised.

## Verification

- `cargo fmt --all -- --check` — pass. `cargo clippy --all-targets --all-features
  -- -D warnings` — pass: no issues. `cargo test` — pass: 197 tests across 18
  suites (no Rust source touched).
- `npm run build` — pass, byte budget enforced: `PASS index-B-c8V-0n.css 27698
  bytes (CSS ceiling 32000, 4302 bytes headroom)`, `PASS index-DcR4563W.js 308449
  bytes (JS ceiling 330000, 21551 bytes headroom)`. `npm run check` — pass: the
  measured contrast pairs, the converter and budget tests, and TypeScript.
- `npm test` — pass: 89 Vitest tests across 13 files (83 before; 6 added for the
  form names, the note, the catalog heading `id` and the page `main` names).
- `npm run test:e2e` — pass: 62 tests in 41.5 s, one worker, no retries —
  `chromium` 50 (the 46 journeys plus 4 in `e2e/landmarks.spec.ts`, including the
  twelve committed pixel baselines), `firefox` 6, `webkit` 6. No baseline was
  regenerated and `e2e/visual.spec.ts-snapshots/` is unchanged.
- `./verify-local.sh` — pass on every stage (fmt, clippy, `cargo test`, `cargo
  build`, `npm ci`, `npm run build`, `npm run check`, `npm run typecheck`, `npm
  test`, `npm run test:e2e`).
- Full Gate `/home/paul/code/forge/target/debug/forge gate . --profile full` —
  aggregate passed, 9 checks (build, security, governance-quality, placeholder,
  product-code-boundary, repository, source-file-size, tests, ui-conformance),
  33/33 files under 1000 lines. Base revision `adf7e88763ee`; persisted evidence
  `.forge/gate/rigcraft/evidence.json`.

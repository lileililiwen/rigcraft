# Assistive-technology review

> **This is not a screen-reader pass.** It records what the browser exposes to
> assistive technology and what the keyboard does. A human using NVDA, VoiceOver
> or JAWS with the script at the end of this document is still required before
> the application can claim screen-reader support. No screen-reader result is
> claimed anywhere in this repository.

## What was measured, and how

The production build (`target/debug/rigcraft` serving `apps/web/dist`) was run on
`127.0.0.1:33301` with a scratch data directory and the shipped sample committed:
snapshot `87f3e411-f2b2-401e-9469-361ce57825f4`, revision 1, 10 records, USD,
stock value US$2,120.90. Only `/api/imports/sample` was used to seed it.

- Browser: Playwright 1.64.1 driving Google Chrome 154.0.8037.97, headless,
  viewport 1440×900, `locale: en-US`, `timezoneId: UTC`.
- Accessibility tree: `locator.ariaSnapshot()` on `body`, one call per route
  after `networkidle`.
- Heading order and landmarks: read from `main h1..h4` and
  `header, nav, main, footer, aside, form` in the live DOM.
- Live regions: every `[aria-live]`, `[role=status]` and `[role=alert]` element
  with its text, before and after the route's primary action.
- Focus: real `Tab` key presses from the document start on `/recommend`, and the
  programmatically focused element after each route's primary action.

An accessibility tree is what an assistive technology *should* receive. It is not
a transcript of what a screen reader *does* receive, and it cannot judge
verbosity, pronunciation or interrupt handling. That is why the human script
exists.

## Landmarks and headings

Every route shares one shell: the skip link is the first focusable node and is
outside the `banner`, the banner holds the `RigCraft` link and a
`navigation "Primary"` with five links, and the page body is `main#main`
(`tabindex="-1"`). `/recommend` adds `complementary "Verdict and score"` for the
verdict rail.

| Route | `document.title` | Landmarks | Live regions |
| --- | --- | --- | --- |
| `/` | RigCraft inventory | banner, navigation "Primary", main | 2 × status |
| `/inventory` | RigCraft inventory | banner, navigation "Primary", main, form | 2 × status |
| `/catalog` | RigCraft catalog | banner, navigation "Primary", main, 2 × form | 1 × status |
| `/builds/check` | RigCraft compatibility check | banner, navigation "Primary", main, form | 1 × status + 1 × `aria-live="off"` |
| `/recommend` | RigCraft build recommendation | banner, navigation "Primary", main, 2 × form, complementary | 1 × status + 1 × polite hint |
| `/diagnose` | RigCraft build diagnosis | banner, navigation "Primary", main | 1 × status |

Heading order per route, as exposed:

- `/` and `/inventory`: `h1 Inventory` → `h2 Snapshot state` →
  `h2 Committed snapshot · revision 1` → `h2 Import a CSV` →
  `h2 Snapshot history` (`h3 Revision 1`) → `h2 Export and backup` →
  `h3 Export this snapshot` → `h3 Back up this database` →
  `h3 Restore from a backup`.
- `/catalog`: `h1 Catalog` → `h2 Upload a catalog` → `h2 Datasets` → `h2 Search`.
- `/builds/check`: `h1 Compatibility check` → `h2 1. Choose an inventory
  snapshot` → `h2 2. Select parts` → `h2 3. Read the verdict`.
- `/recommend`: `h1 Build recommendation` → `h2 1. Choose an inventory snapshot`
  → `h2 2. Ask in your own words (optional)` (`h3 No key configured`,
  `h3 What leaves this machine`) → `h2 3. Budget and constraints` →
  `h2 4. Read and compare options` → `h2 Verdict and score` (inside the
  complementary region).
- `/diagnose`: `h1 Diagnose an existing build` → `h2 1. Choose an inventory
  snapshot` → `h2 2. Paste the parts list` → `h2 3. Review the matches` →
  `h2 4. Add a part by hand` → `h2 5. Read the diagnosis`.

No heading level is skipped and each route has exactly one `h1`.

## Accessibility-tree extracts

One extract per route, from the `main` subtree of `locator("body").ariaSnapshot()`
after the page settled. The banner above it is identical on every route, so it is
shown once.

Shared shell, every route:

```
- link "Skip to recommendation":        (name varies per route)
  - /url: "#main"
- banner:
  - link "RigCraft":
    - /url: /inventory
  - navigation "Primary":
    - link "Inventory": /url: /inventory
    - link "Catalog": /url: /catalog
    - link "Compatibility": /url: /builds/check
    - link "Recommend": /url: /recommend
    - link "Diagnose": /url: /diagnose
```

`/` and `/inventory`:

```
- main:
  - heading "Inventory" [level=1]
  - paragraph: Import owned hardware from a UTF-8 CSV, correct every bad row, and commit a versioned snapshot. …
  - paragraph: Inventory is private to this local service. It is not uploaded to a catalog, shared with an AI provider, or logged.
  - status: "Opened the newest snapshot, revision 1: 10 records, US$2,120.90 stock value. Export and backup are ready below."
  - region "Snapshot state":
    - heading "Snapshot state" [level=2]
    - term: Revision  /  definition: "1"
    - term: Records  /  definition: "10"
    - term: Units in stock  /  definition: "10"
    - term: Stock value  /  definition: US$2,120.90
```

`/catalog`:

```
- main:
  - heading "Catalog" [level=1]
  - paragraph: Load a hardware catalog you are licensed to use. …
  - paragraph: The catalog is your data, not RigCraft's. …
  - status: No catalog uploaded in this session.
  - region "Upload a catalog":
    - heading "Upload a catalog" [level=2]
    - paragraph: One JSON document in the rigcraft-catalog version 1 format. …
```

`/builds/check`:

```
- main:
  - heading "Compatibility check" [level=1]
  - paragraph: Check a selected build against the facts in one inventory snapshot. …
  - paragraph: This check runs only against your local inventory. …
  - status: "Snapshot revision 1 loaded: 10 records."
  - region "1. Choose an inventory snapshot":
    - heading "1. Choose an inventory snapshot" [level=2]
    - combobox "Snapshot":
```

`/recommend`:

```
- main:
  - heading "Build recommendation" [level=1]
  - paragraph: Enter a total budget and what the machine is for. …
  - paragraph: The search runs locally with no hardware catalog. …
  - status: "Snapshot revision 1 loaded: 10 records, currency USD."
  - region "1. Choose an inventory snapshot":
    - heading "1. Choose an inventory snapshot" [level=2]
    - combobox "Snapshot":
  - region "2. Ask in your own words (optional)":
    - group "Choose owned parts" / group "Parts to exclude"
    - button "Generate recommendations" [disabled]
    - text: 0 locked · 0 excluded
  - complementary "Verdict and score":
    - heading "Verdict and score" [level=2]
```

`/diagnose`:

```
- main:
  - heading "Diagnose an existing build" [level=1]
  - paragraph: Describe the PC you already have — …
  - paragraph: Matching compares your lines with the selected local snapshot only. …
  - status: "Snapshot revision 1 loaded: 10 records. Matching compares pasted models with these records only."
  - region "1. Choose an inventory snapshot":
    - heading "1. Choose an inventory snapshot" [level=2]
    - combobox "Snapshot":
```

The decorative uppercase labels on recommendation options (`BUILD SPEC · OPTION
1`) are `aria-hidden="true"` and do not appear in the tree; the option heading
carries the same meaning in words (asserted by `ui-foundation` and by
`RecommendPage.test.tsx`).

## Focus order through the primary journey

`/recommend`, seeded snapshot selected, `Tab` pressed from the document start,
then the budget filled so the primary action is enabled. `Tab` stops, in order:

| # | Element | Accessible name |
| --- | --- | --- |
| 1 | `a.skip-link` | Skip to recommendation |
| 2 | `a.brand` | RigCraft |
| 3 | `nav a` | Inventory |
| 4 | `nav a` | Catalog |
| 5 | `nav a` | Compatibility |
| 6 | `nav a` | Recommend |
| 7 | `nav a` | Diagnose |
| 8 | `select#recommend-snapshot` | Snapshot |
| 9 | `summary` | 2. Ask in your own words (optional) |
| 10 | `input#recommend-budget` | Total budget (USD) |
| 11 | `select#recommend-use-case` | Primary use |
| 12 | `summary` | Choose owned parts (0 parts selected) |
| 13 | `summary` | Choose excluded parts (0 parts selected) |
| 14 | `button` | Generate recommendations |

Activating the skip link (`Enter` on stop 1) focuses `main#main` — measured as
`MAIN#main`. After `Generate recommendations`, focus is moved programmatically to
`h2#recommend-result-title` ("Recommended builds"), so the result is announced
without the operator hunting for it.

The same programmatic focus move exists on the other result routes:
`h2#result-title` on `/builds/check`, `h2#diagnosis-result-title` on `/diagnose`,
the review heading after matching a list (`#review-title`), the snapshot heading
after a commit (`#snapshot-title`), and the "Catalog accepted" heading after an
upload. The export and restore panels focus their own headings at the same
points.

Measured after the primary action on each route:

| Route | Action | Focused element |
| --- | --- | --- |
| `/builds/check` | Check compatibility | `h2#result-title` |
| `/recommend` | Generate recommendations | `h2#recommend-result-title` |
| `/diagnose` | Match this list → Diagnose this build | `h2#diagnosis-result-title` |
| `/catalog` | Import catalog | the `h3` heading "Catalog accepted" |
| `/inventory` | Download CSV | focus stays on the button; status region announces |

## Live regions and what they announce

| Route | Region | Politeness | Announcement |
| --- | --- | --- | --- |
| `/`, `/inventory` | `div.snapshot-status` | polite (`role=status`) | On load: "Opened the newest snapshot, revision 1: 10 records, US$2,120.90 stock value. Export and backup are ready below." After a download: "Downloaded rigcraft-inventory-snapshot-r1-87f3e411.csv. CSV export is lossy: it omits lot identity, snapshot metadata and per-field provenance references. Use format=json for the authoritative artifact." |
| `/`, `/inventory` | `div.export-hint` | polite (`role=status`) | On load: "Exports and backups are created by this local service only." Static text, not a change. |
| `/catalog` | `div.catalog-status` | polite (`role=status`) | On load: "No catalog uploaded in this session." After an upload: "Catalog accepted: 1 part from synthetic-fixture." |
| `/builds/check` | `div.check-status` | polite (`role=status`) | On load: "Snapshot revision 1 loaded: 10 records." After a check: "Check complete: Unknown. 0 compatible · 0 conflicts · 0 unknown · build incomplete." |
| `/builds/check` | `span.small.muted` | `aria-live="off"` | Inner text only, "1 unit selected"; deliberately not announced. |
| `/recommend` | `div.recommend-status` | polite (`role=status`) | On load: "Snapshot revision 1 loaded: 10 records, currency USD." After generating: "Recommendation complete: 1 materially different option within US$1,800.00." |
| `/recommend` | `p#assistant-parse-hint` | polite | "Enter your own provider key to use the assistant." Assistant guidance only. |
| `/diagnose` | `div.diagnose-status` | polite (`role=status`) | On load: "Snapshot revision 1 loaded: 10 records. Matching compares pasted models with these records only." After matching: "Match review ready: 2 lines. 2 matched, 0 ambiguous, 0 without a match, 0 needing a fix. Resolve every line before diagnosing." After diagnosing: "Diagnosis complete: unknown. 0 parts must be replaced, 2 parts should be verified, 0 parts can be kept, and 0 parts not evaluated by any rule. Upgrade plan: not needed." |

Errors use `role="alert"` and take focus (`tabIndex={-1}`), so an error is both
announced assertively and reachable; the refusal and stale-note regions are
`role=status`. Recommendation, check and diagnosis results also carry a stale
note (`role=status`) when the snapshot changes underneath a rendered result.

## Role and name gaps found

Recorded here, not fixed: this change adds tests and documentation only.

1. **Unnamed forms.** `/inventory`, `/catalog` (two) and `/recommend` (two)
   render `<form>` elements with no accessible name. They are not named
   landmarks, so landmark navigation cannot distinguish "the import form" from
   "the search form". A `aria-labelledby` to the section heading (or a visually
   hidden `aria-label`) would fix it without changing layout.
2. **A static hint marked as a live status.** On `/` and `/inventory`,
   "Exports and backups are created by this local service only." is
   `role="status"`. It may be announced on page load even though nothing
   changed, adding noise before the operator acts. `role="note"` or no role
   would describe it better.
3. **The focused "Catalog accepted" heading has no id.** The other result
   headings are addressable (`#result-title`, `#recommend-result-title`,
   `#diagnosis-result-title`, `#review-title`, `#snapshot-title`); this one is
   focused by ref only, so it cannot be linked to or re-focused from a test or a
   skip target.
4. **The `aria-live="off"` selection counter.** `1 unit selected` is deliberately
   silent, so a keyboard operator who changes selection with the checkbox gets no
   spoken count. That is a reasonable choice against chattiness, but it should be
   confirmed by a human with a screen reader rather than assumed.
5. **No accessible name on `main`.** There is exactly one `main` per route, so
   this is not a landmark collision today; it would matter if a route ever added
   a second.

The plain-text 404 document has no landmarks, headings or controls by design; a
screen reader reads the sentence `404 Not found.` and nothing else.

## The human script to hand over

Run this on the built application with the shipped sample committed. Do not
change the application; if something is wrong, record the route, the key and what
was announced, and file it as a defect.

**Pairs:** NVDA 2026.x with Firefox on Windows; VoiceOver with Safari on macOS;
JAWS 2026.x with Chrome on Windows. A pass with one pair is not a pass for the
others.

1. **Load `/inventory`.** Expect the document title "RigCraft inventory", then
   the page heading "Inventory". Expect the status message beginning "Opened the
   newest snapshot". Note whether the second status ("Exports and backups are
   created by this local service only.") is also spoken on load.
2. **Press `Tab` once.** Expect "Skip to inventory, link". Press `Enter`. Expect
   focus on the main region and the next `Tab` to reach the `RigCraft` link, then
   the five primary navigation links with the current one reported as "current
   page".
3. **Navigate `/inventory` by headings (`H`/`1`–`6`).** Expect the order listed
   above, one `h1`, and the records table announced as "Committed stock records
   with provenance and unknown specs" with its column headers read per cell.
4. **On `/catalog`,** upload the synthetic fixture, tick the rights checkbox and
   activate "Import catalog". Expect focus on "Catalog accepted", the status
   announcing the accepted part count, and the registry table announced with its
   caption.
5. **On `/builds/check`,** select the snapshot, select a CPU and check the build.
   Expect the result heading announced when focus moves, and the status
   "Check complete: …" read as one sentence. Confirm the unknown rule is read as
   "unknown", not "compatible".
6. **On `/recommend`,** follow the focus order table above to the budget field,
   type `1800`, and press `Enter`. Expect "Recommendation complete: 1 materially
   different option within US$1,800.00." and the result heading announced. Read
   the first option: confirm the decorative "BUILD SPEC · OPTION 1" label is not
   spoken, the status chip is spoken as a word, and the gaming rule heading is
   reachable by heading navigation.
7. **On `/diagnose`,** paste the two-line list, activate "Match this list", then
   "Diagnose this build". Expect the match-review and diagnosis announcements
   above, the 1–5 section headings, and each advice group read with its count.
8. **Navigate `/recomend` and `/nope`.** Expect a real 404: the document is plain
   text, the title is empty, and the only content is "404 Not found." Confirm no
   application shell is announced.
9. **Repeat step 6 at a 390 px window** and confirm nothing is clipped or
   reachable only by horizontal scrolling.
10. **Record, per route:** the spoken heading order, whether every status change
    was announced once and only once, whether focus moved to the result, and
    whether any control was reachable but unlabelled. Attach the screen reader,
    browser, OS and version.

## What this document does not establish

- No screen-reader output was captured. Nothing here says NVDA, VoiceOver or JAWS
  reads the application correctly.
- No automated accessibility audit score is claimed; the earlier Lighthouse
  accessibility results live in their own route evidence documents.
- No mobile screen reader (TalkBack, VoiceOver on iOS) was exercised.
- No high-contrast or forced-colours mode was reviewed; `prefers-reduced-motion`
  and the dark palette are covered by `ui-foundation` evidence, not here.

# Verdict console browser evidence (2026-11-01)

Source: the production Web build (`npm run build`) served by `target/debug/rigcraft`
on `127.0.0.1:8823` with a scratch data directory
`/tmp/opencode/rigcraft-verdict`. The shipped sample was imported through the
documented API: 10 valid rows, 0 errors, 0 duplicates, snapshot
`ff546e40-59fa-4c9c-a8a0-0d43180b6ac5` revision 1 (10 records, value 212090
minor units, created 2026-11-01T03:14:58.593Z). Tool: Chrome DevTools MCP against
real Chrome; viewports 1440×900 and 390×844.

## What was exercised

### `/builds/check` — CPU only (real incomplete check)

Selected `AMD Ryzen 7 7800X3D` (`CPU-7800X3D`) and ran the check. The real
report returned:

- Overall `Unknown`, `Incomplete build · 6 missing`, `0` rules passed, `0`
  conflicts, `0` unknown rules, `1` part not evaluated (no rule can run for a
  lone CPU).
- Build gaps rendered once each, reason then action:
  `A motherboard is required. Select a motherboard from this snapshot.`,
  `Memory is required. Select memory from this snapshot.`,
  `A power supply is required. Select a power supply from this snapshot.`,
  `A case is required. Select a case from this snapshot.`,
  `Startup storage is required. Select startup storage from this snapshot.`,
  `No cooler is selected and sourced bundled-cooling evidence is absent. Select a cooler, or record \`bundled_cooler=yes\` on the CPU with its source.`
  No graphics-card gap appears because the CPU declares `igpu=yes`.
- No sentence contains `is required from this snapshot`.

Captures: `docs/assets/verdict-check-desktop.png`,
`docs/assets/verdict-check-mobile.png`.

### `/builds/check` — the seeded AM5 build (rule matrix)

Selected the CPU, motherboard, memory, graphics card, PSU, case, cooler and the
1 TB drive and ran the check. The real report returned `12 compatible · 0
conflicts · 2 unknown`, `Complete build`, `Parts not evaluated 0`.

- The matrix rendered **14 rows**, one per rule, in category then rule-id order.
  `cpu-bios` shows `Unknown`, the CPU model value `AMD Ryzen 7 7800X3D` with
  `Catalog · manufacturer@2026-10-31`, the AMD URL and the observation time,
  `bios_supports`/`bios` as `Not provided · No source recorded`, and the
  follow-up `Confirm the motherboard's installed BIOS version supports AMD Ryzen
  7 7800X3D.`
- `cpu-socket` shows `Compatible`, both `AM5` values with the AMD and ASUS
  references, and `No follow-up; the rule reached a verdict.`
- `psu-capacity` shows `Unknown` with the supply wattage, CPU and GPU nominal
  draw, `peak_power` `Not provided`, and the sourced-peak follow-up.

Captures: `docs/assets/verdict-check-matrix-desktop.png`,
`docs/assets/verdict-check-matrix-mobile.png`.

### `/diagnose` — an 8-line paste (real diagnosis)

Pasted the whole seeded build (CPU, motherboard, memory, graphics card, PSU,
case, cooler, storage). The match review reported **8 matched, 0 ambiguous, 0
without a match, 0 needing a fix**, all exact candidates preselected. The
diagnosis returned:

- Overall `Unknown`; `0` parts must be replaced, `3` should be verified first,
  `5` are fine as is, `0` were not evaluated; upgrade plan `not needed`.
- Coverage is stated **once**: `Every described part was checked by at least one
  rule.` The old second sentence and the `8 of 8 … were checked` fraction are
  not rendered.
- The four group cards rendered as
  `Must replace (0)`, `Should upgrade — verify first (3)`, `Fine as is (5)` and
  `Not verified — no rule evaluated (0)`, each with a symbol-and-word chip.
- The described-build table, every rule disclosure, cost line and plan state
  stayed present.

Captures: `docs/assets/verdict-diagnosis-desktop.png`,
`docs/assets/verdict-diagnosis-mobile.png`.

## Measurements

| Measurement | 1440×900 | 390×844 |
| --- | --- | --- |
| `/builds/check` `scrollWidth` / `clientWidth` (result, 14-row matrix) | 1440 / 1440 | 390 / 390 |
| `/builds/check` matrix caption width × height | 1182 × 32 px | 332 × 56 px |
| `/diagnose` `scrollWidth` / `clientWidth` (result) | 1440 / 1440 | 390 / 390 |
| `/diagnose` evidence caption width × height | 1182 × 32 px | 332 × 56 px |
| `.console-summary` position | `sticky`, `top: 0` (measured top 0 while scrolled) | `static` |

No horizontal overflow was measured on either page at either width, with either
result state. Long provenance URLs wrap inside the mobile card layout
(`overflow-wrap: anywhere`), so the matrix does not reintroduce page overflow.

Focus: after a fresh `/diagnose` load, the first `Tab` focused the skip link with
`outline: 3px solid rgb(34, 211, 238)`, `outline-offset: 2px`,
`:focus-visible` true, rect top 8 / bottom 52 px. A rule-evidence link inside an
advice card measured the same ring. The summary, group cards and matrix rows add
no focusable surface of their own.

## Lighthouse

Chrome DevTools MCP `lighthouse_audit`, navigation mode, production build:

| Route | Device | Accessibility | Best practices | SEO | Agentic browsing | Audits |
| --- | --- | --- | --- | --- | --- | --- |
| `/builds/check` | desktop | 100 | 100 | 100 | 100 | 53 passed, 0 failed |
| `/builds/check` | mobile | 100 | 100 | 100 | 100 | 53 passed, 0 failed |
| `/diagnose` | desktop | 100 | 100 | 100 | 100 | 50 passed, 0 failed |
| `/diagnose` | mobile | 100 | 100 | 100 | 100 | 50 passed, 0 failed |

Reports: `docs/evidence/verdict-lighthouse/check-desktop/`,
`check-mobile/`, `diagnose-desktop/` and `diagnose-mobile/` (`report.json` and
`report.html`).

## Privacy and scope

- No hardware catalog, AI provider or network service was consulted by either
  page; every value came from the local SQLite snapshot. The provider key field
  was not opened.
- No stock was reserved and no snapshot was mutated by a check or diagnosis.
- The change adds no colour literal, inline style, dependency, webfont or
  animation; the measured contrast record keeps its 40 pairs from the brand
  token layer.

## Not done

- Lighthouse performance was not measured (the tool's categories here are
  accessibility, best practices, SEO and agentic browsing).
- No visual-diff baseline, no non-Chrome browser and no manual screen-reader
  pass; the accessibility tree, Lighthouse and keyboard focus were the checks.
- The engine's own `step(s)` wording in the plan summary and the API's
  `unit(s)` stock error are still engine/export text; only the page-composed
  strings use the plural helper.

# UI foundation browser evidence

Package 1, `dark-brand-token-layer`: the measured dark token layer, native-UI
and focus legibility, the two shell-chrome fixes and the app icon. No page was
redesigned, no API, route, copy string asserted by e2e or engine behaviour
changed.

## How it was run

- Built: `cargo build`, `npm run build` (`dist/assets/index-BSJWpQ3q.css`,
  `dist/assets/index-CsPSQQfF.js`).
- Served: `target/debug/rigcraft` on `127.0.0.1:8171` with
  `RIGCRAFT_DATA_DIR=/tmp/opencode/rigcraft-ui-foundation`.
- Data: the shipped sample was downloaded from `GET /api/imports/sample` and
  committed through `POST /api/imports` + `POST /api/imports/{id}/commit`
  (10 records, revision 1, `US$2,120.90`, snapshot
  `3e5b9989…318e`). Every route therefore rendered loaded state, not an empty
  shell.
- Browser: Chrome DevTools MCP, one headless page. Measurements were read with
  `evaluate_script` (`getBoundingClientRect`, `getComputedStyle`) and by the
  Playwright suite; the audits by the Chrome DevTools MCP `lighthouse_audit`
  tool; the captures by `take_screenshot`.

## Lighthouse accessibility, dark theme, all five routes

Mode `navigation`, device `desktop` and `mobile`. Every category reported 100
and no audit failed.

| Route | Viewport | Accessibility | Best practices | SEO | Agentic browsing | Audits passed / failed | Report |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `/inventory` | desktop | 100 | 100 | 100 | 100 | 52 / 0 | `docs/evidence/ui-foundation-lighthouse/inventory/` |
| `/catalog` | desktop | 100 | 100 | 100 | 100 | 50 / 0 | `docs/evidence/ui-foundation-lighthouse/catalog/` |
| `/builds/check` | desktop | 100 | 100 | 100 | 100 | 53 / 0 | `docs/evidence/ui-foundation-lighthouse/builds-check/` |
| `/recommend` | desktop | 100 | 100 | 100 | 100 | 51 / 0 | `docs/evidence/ui-foundation-lighthouse/recommend/` |
| `/diagnose` | desktop | 100 | 100 | 100 | 100 | 50 / 0 | `docs/evidence/ui-foundation-lighthouse/diagnose/` |
| `/inventory` | mobile | 100 | 100 | 100 | 100 | 52 / 0 | `docs/evidence/ui-foundation-lighthouse-mobile/inventory/` |
| `/catalog` | mobile | 100 | 100 | 100 | 100 | 50 / 0 | `docs/evidence/ui-foundation-lighthouse-mobile/catalog/` |
| `/builds/check` | mobile | 100 | 100 | 100 | 100 | 53 / 0 | `docs/evidence/ui-foundation-lighthouse-mobile/builds-check/` |
| `/recommend` | mobile | 100 | 100 | 100 | 100 | 51 / 0 | `docs/evidence/ui-foundation-lighthouse-mobile/recommend/` |
| `/diagnose` | mobile | 100 | 100 | 100 | 100 | 50 / 0 | `docs/evidence/ui-foundation-lighthouse-mobile/diagnose/` |

Each directory holds `report.json` and `report.html`. Performance was not
measured; this package changes no script, markup structure or data volume.

## Measured effective theme

`node scripts/check-contrast.mjs` resolved the ordered sources
(`vendor/tokens.css` then `brand.css`, later wins), recorded both hashes and
measured 40 pairs: 28 text pairs at 4.5:1 and 12 UI pairs at 3:1 (three control
border pairs and nine focus-ring pairs — the page, panel, raised surface and all
six chip tints). All 40 pass.

- Source hashes in `docs/evidence/token-contrast.json`: vendor
  `7ca6b7956b3f…c19b` (equal to `.project.json` `pin_sha256`) and brand
  `fc1fc93211cf…fbd`.
- Lowest text pair: error text on raised, `#f87171` on `#1b2740`, **5.38:1**
  (4.5:1 required).
- Lowest UI pair: control border on raised, `#6b7a99` on `#1b2740`, **3.45:1**
  (3:1 required).
- Decoration is listed and not measured: `--color-border-subtle` is
  `color-mix(in srgb, #6b7a99 38%, transparent)` and carries no text.
- Measured live: `getComputedStyle(document.documentElement).colorScheme` was
  `dark`, `accentColor` `rgb(6, 182, 212)`, body background
  `rgb(10, 18, 36)` and body text `rgb(230, 237, 247)`.

The full table is in `docs/branding.md`.

## Focus ring

Measured after pressing Tab once on `/recommend` (Chrome DevTools MCP
`press_key` + `evaluate_script`):

```
element  A.skip-link "Skip to recommendation"
rect     top 8, bottom 52 (fully inside the viewport)
outline  3px solid rgb(34, 211, 238)
offset   2px
```

The ring colour is `#22d3ee`, measured against the surface `#141e33` at
**9.20:1**, against the page at 10.33:1, against the raised surface at 8.24:1
and between 7.68:1 and 9.07:1 against every chip tint. Capture:
`docs/assets/ui-foundation-focus-desktop.png`.

## Shell chrome

### Table caption at 390 px

`/inventory`, revision 1 opened, `table.responsive-table > caption`:

| Viewport | Rendered width | Rendered height | Required |
| --- | --- | --- | --- |
| 1440×900 | 1,182 px | 24 px | full width of the table block |
| 390×844 | **332 px** | 56 px (two lines) | > 150 px |

Before the fix the same caption rendered **92.58 px** wide and stacked one word
per line; the canary below reproduces that.

### Skip link at rest

`getBoundingClientRect()` of `a.skip-link` while unfocused:

| Viewport | Height | Top | Bottom edge |
| --- | --- | --- | --- |
| 1440×900 | 44 px | −82 px | **−38 px** |
| 390×844 | 44 px | −80 px | **−36 px** |

The rule is `transform: translateY(calc(-100% - var(--space-4)))` on the
element's own height, not a fixed `-2rem`; `:focus` sets `top: var(--space-2)`
and `transform: none`.

### No horizontal overflow

`document.documentElement.scrollWidth === clientWidth` on every route at every
width tried (a scratch Playwright script using the installed Chrome):

| Route | 320 | 390 | 768 | 1440 |
| --- | --- | --- | --- | --- |
| `/inventory` | 320 = 320 | 390 = 390 | 768 = 768 | 1440 = 1440 |
| `/catalog` | 320 = 320 | 390 = 390 | 768 = 768 | 1440 = 1440 |
| `/builds/check` | 320 = 320 | 390 = 390 | 768 = 768 | 1440 = 1440 |
| `/recommend` | 320 = 320 | 390 = 390 | 768 = 768 | 1440 = 1440 |
| `/diagnose` | 320 = 320 | 390 = 390 | 768 = 768 | 1440 = 1440 |

### Icon

`GET /favicon.ico` → 200 `image/x-icon`; `GET /favicon.svg` → 200
`image/svg+xml`. `apps/web/public/favicon.svg` is the source and the 32×32
`favicon.ico` is its fallback; `index.html` links both. `e2e/shell.spec.ts`
also listens for icon responses during a real page load and asserts none fails.

## Canaries — the guards are load-bearing

1. `--brand-slate-200` was temporarily changed from `#b9c5da` to `#3a4557`.
   `node scripts/check-contrast.mjs` printed `5 token pair(s) below the WCAG 2.2
   AA minimum` and exited **1**. The file was restored and the record
   regenerated (40/40 pass).
2. The caption rule and the skip-link `translateY` were temporarily reverted to
   the pre-change CSS and the bundle rebuilt. `e2e/shell.spec.ts` failed both
   assertions with **92.578125 px** caption width and a skip link bottom edge of
   **12 px** — exactly the 12 px peek the change removes. The CSS was restored,
   the bundle rebuilt and the three shell tests passed.

## Conformance and the vendored pin

- `sha256sum apps/web/src/styles/vendor/tokens.css` =
  `7ca6b7956b3f6b475254b4a5d0cf599724e4e1af5f0f88aba4fbccb9f38fc19b`, equal to
  `.project.json → ui.shared_layer.pin_sha256`.
- `python3 ../forge/modules/workspace-governance/scripts/ui_conformance.py .`
  → `PASS: rigcraft ui_files=58 errors=0 review_required=0 baselined=0`.
- `.project.json → ui.scope.exclude` names the two token files (the vendored kit
  and the brand layer). The census counts 58 UI files (the committed Lighthouse
  `report.html` files included); content rules read the 25 under the declared
  scope, so a colour literal outside the token layer still fails. `brand.css` is also verified by
  the contrast checker to declare `:root` custom properties only.
- No new dependency, no webfont, no inline style added, no `!important` outside
  the existing reduced-motion block.

## Full Gate

`/home/paul/code/forge/target/debug/forge gate . --profile full` — aggregate
**passed**, 9 checks (build, security, governance-quality, placeholder,
product-code-boundary, repository, source-file-size, tests, ui-conformance),
32/32 files under the 1,000-line ceiling. The Gate labels the working-tree base
revision `3f2170c275d5`; its persisted evidence is
`.forge/gate/rigcraft/evidence.json`.

## Captures

All 1440×900 and 390×844, Chrome DevTools MCP `take_screenshot` against the
production build. Listed in `docs/assets/capture-plan.md`.

| Capture | Route | Viewport | What it shows |
| --- | --- | --- | --- |
| `ui-foundation-inventory-desktop.png` | `/inventory` | 1440×900 | Page substrate, panels, form controls, nav active rule |
| `ui-foundation-inventory-records-desktop.png` | `/inventory` | 1440×900 | Surface vs raised records and chip tints |
| `ui-foundation-focus-desktop.png` | `/recommend` | 1440×900 | Focused skip link with the 3 px cyan ring |
| `ui-foundation-catalog-desktop.png` | `/catalog` | 1440×900 | Native file input and checkbox in dark mode |
| `ui-foundation-check-desktop.png` | `/builds/check` | 1440×900 | Radios, facts disclosure and panel tones |
| `ui-foundation-recommend-desktop.png` | `/recommend` | 1440×900 | Warning rule, `? Unknown` chip, raised candidate |
| `ui-foundation-diagnose-desktop.png` | `/diagnose` | 1440×900 | The unchanged 1–5 skeleton |
| `ui-foundation-inventory-records-mobile.png` | `/inventory` | 390×844 | The full-width two-line caption and stacked rows |
| `ui-foundation-catalog-mobile.png` | `/catalog` | 390×844 | Stacked upload and empty registry |
| `ui-foundation-check-mobile.png` | `/builds/check` | 390×844 | Stacked part selection |
| `ui-foundation-recommend-mobile.png` | `/recommend` | 390×844 | Summary, warning rule and chip stacked |
| `ui-foundation-diagnose-mobile.png` | `/diagnose` | 390×844 | Stacked 1–5 skeleton |

## What this does not cover

- Lighthouse performance, visual-diff baselines and browsers other than Chrome.
- A full brand mark or a light theme. Dark is the only shipped mode and a light
  theme would need its own measured table before it can exist.
- Automated audits are not a manual screen-reader pass; no NVDA/VoiceOver
  session was run.

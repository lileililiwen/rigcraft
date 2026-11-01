# Design program: reference-led, token-safe

## Why this document

The Stitch project `OpenPCBuilder` (design system **Kinetic Silicon**) is a visual
reference for RigCraft: a dark instrument panel, cyan interactive traces, emerald
verified states, rust warnings, monospaced telemetry and a dense workbench. It is
a *reference*, not a source. Copying its markup or its palette verbatim would
bypass the vendored token contract, the measured contrast evidence and the
ui-conformance gate, and would import product scope RigCraft has deliberately
declined (crawlers, ETL, AI-by-default).

This document records what is taken, what is refused, the rules that may not
break, and six delivery packages. Each package is a separate OpenSpec change;
only one is active at a time (`AGENTS.md`), in the order below.

## The reference

- Project: <https://stitch.withgoogle.com/projects/1235343658642058702> (public).
- Screens: `智能装机工作台与兼容校验` (workbench), `硬件规格录入与反爬 ETL 维护`
  (DataOps), `系统架构与技术选型总览` (architecture overview).
- Theme: dark `#0b1326`, primary `#06b6d4`, secondary `#10b981`, tertiary
  `#f97316`, Inter + JetBrains Mono, 4 px radii, 4 px spacing scale.

## Take / decline

| Take | Why it fits RigCraft |
| --- | --- |
| Three tonal surfaces on a dark substrate | Separates page, panel and nested module without shadows |
| Cyan = interactive, emerald = verified, rust = warning/conflict | Maps onto the existing `primary` / `success` / `warning` / `danger` semantics |
| 1 px hairline borders, 4/8/12 px radii | Already the shared radius scale; no new shape language |
| Monospaced numeric and specification readouts with tabular numerals | Prices, watts, millimetres and rule counts stop jittering between rows |
| Dense workbench layout for tool pages | Fits the existing three-question workflow (constraints → options → verdict) |
| Caps micro-labels and status chips | Useful labels, provided the meaning is also in text and the size stays legible |

| Decline | Why |
| --- | --- |
| 2560 px fixed four-column canvas | RigCraft is fluid 320–1440 px with one 76 rem column today; the reference cannot collapse |
| 10 px label/body sizes | Below a comfortable reading size; WCAG does not forbid it but our own contrast and zoom evidence assumes ≥12 px |
| Chinese marketing copy, fake telemetry (“18 ms”, “100 %”, mock KPI rows) | Claims must be measured or removed; RigCraft shows only real report values |
| Crawler, anti-bot and ETL screens | Conflicts with `docs/data-sources.md`; the catalog is operator-supplied only |
| AI-by-default framing (“AIR-BYOK ON”, LLM picks the model) | RigCraft keeps the assistant optional; the deterministic engine decides |
| `¥`-only pricing and per-row currency switching | One currency per snapshot is a deliberate contract |
| Glow, pulse and canvas connector animation | Decorative, costs paint, and fights `prefers-reduced-motion` |
| The architecture page as product UI | Belongs in `README.md` / `docs/`, not a fifth route |
| Any Stitch markup, class names or hex literals in components | Bypasses the pinned kit and the conformance scan |

## Guardrails — what may not break

1. **Vendored tokens are immutable.** `apps/web/src/styles/vendor/tokens.css` is
   generated from `ui/tokens/tokens.json` and pinned by
   `.project.json → ui.shared_layer.pin_sha256` (`7ca6b795…`). No edit, no
   re-pin. Brand values live in a new app-owned file imported *after* it.
2. **No colour literals outside the brand token file.** Components keep using
   variables only (`docs/branding.md`). The ui-conformance scan reads
   `.project.json → ui.scope`.
3. **Contrast is measured, not asserted.** `npm run check` runs
   `scripts/check-contrast.mjs`, which resolves the ordered sources (vendored
   then brand), records both file hashes and measures 40 effective dark pairs.
   A pair below AA fails the build. A text pair must be a flat value;
   `color-mix()` and alpha are decoration and are never scored as text.
4. **Status is never colour alone.** Icon + word + border
   (`docs/branding.md`). Chips carry `✓ / ✕ / ? / ⇄` and a text label.
5. **No new runtime dependency and no downloaded webfont.** The app ships no
   font files today; phones should not pay for one. Use a system UI stack and a
   system monospace stack.
6. **Existing copy is contract.** e2e asserts strings such as
   `Gaming requires a discrete graphics card`,
   `1 materially different build found`, `no option was padded to reach three`,
   `Skip to …`, the `1–5` diagnosis skeleton and every API label. A redesign may
   restyle them, not rename them.
7. **No new product scope.** No crawler, ETL, sync, third-party fetch, cloud
   storage or AI default enters through a visual change.
8. **Motion is optional.** Transitions ≤ 160 ms, no infinite animation; the
   global reduced-motion rule stays.

## How to improve the CSS without breaking the contract

- **One brand file, one direction.** `styles/brand.css` declares only
  `--brand-*` primitives and the semantic aliases (`--color-background`,
  `--color-surface`, `--color-foreground`, `--color-primary`, …). Components
  never import a primitive directly. Import order:
  `tokens.css → brand.css → app.css`.
- **Text pairs stay computable.** The contrast checker can only resolve flat
  values and `var()` aliases. Text-on-surface pairs therefore use literal
  brand tokens. `color-mix()` and alpha are allowed only for decoration —
  borders, dividers, focus offsets, hover washes — never for a value that
  carries text.
- **Elevation by tone, not shadow.** Three surfaces
  (`--color-background`, `--color-surface`, `--color-surface-raised`) plus
  `--color-border-subtle`. Shadows are reserved for overlays.
- **`color-scheme: dark`** on `:root` so native selects, scrollbars, checkbox
  ticks and autofill are legible; set `accent-color` for native controls.
- **Tabular numerals.** `font-variant-numeric: tabular-nums` plus a system mono
  stack (`ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace`)
  for prices, specs and rule counts.
- **Type floor.** Body 16 px (unchanged), meta 13–14 px, never below 12 px for
  content; caps labels are redundant decoration and always repeated in the
  accessible text.
- **Fluid layout.** Keep the 76 rem column as the default; add a wider
  `--layout-wide` (88 rem) only on workbench pages, with a named-area grid that
  collapses to one column below 1100 px. No fixed pixel canvas.
- **Focus survives every surface.** 3 px ring, 2 px offset, ≥3:1 against page,
  surface, raised surface and chip tint.
- **Prefer native semantics.** Real `<table>`, `<details>`, `<fieldset>`,
  `<button>`; no div-with-role replacements for the new chrome.
- **No `!important`** outside the existing reduced-motion block; no inline
  styles; flat specificity.

## Package map

| # | Change | Single outcome | Depends on |
| --- | --- | --- | --- |
| 1 | `dark-brand-token-layer` | The measured dark token set and a defect-free shell | — |
| 2 | `workbench-recommendation` | `/recommend` as the workbench reference | 1 |
| 3 | `verdict-console` | `/builds/check` and `/diagnose` as a rule console | 1 |
| 4 | `dataops-console` | `/inventory` and `/catalog` as an operator console | 1 |
| 5 | `service-correctness` | API and client-route correctness, safe CSV | — |
| 6 | `delivery-hardening` | Compression, cache and security headers | — |

Packages 5 and 6 are independent of the design work and may be taken first as
quick, low-risk wins. Packages 2–4 each touch one page family and can be
reviewed separately.

---

## Package 1 — `dark-brand-token-layer`

**Outcome:** a measured dark token set, a legible shell and no regressions in
the light evidence that still applies. No page is redesigned.

**Requirements**

- R1 A single app-owned token file declares the dark palette and semantic
  aliases; no component gains a colour literal.
- R2 `scripts/check-contrast.mjs` reads the vendored file **and** the brand
  file, measures the effective dark pairs (existing 16 plus muted-on-raised,
  chip text on tint, primary/secondary/tertiary on all three surfaces, border
  and focus on all three), writes both hashes, and fails below AA.
- R3 `color-scheme: dark`; native controls, autofill and scrollbars are legible;
  focus ring ≥3:1 on every surface.
- R4 Shell defects fixed while the rules are rewritten: table captions are full
  width at ≤760 px; the skip link is fully off-screen when unfocused and fully
  visible on focus.
- R5 A favicon exists (`/favicon.ico` 200, `<link rel="icon">`), served with a
  cache policy.
- R6 `docs/branding.md` records the decided dark set, the measured table and the
  light/dark position; `docs/design-program.md` records take/decline.
- R7 Evidence: Lighthouse accessibility 100 in dark on all five routes
  (desktop + mobile), captures at 1440/390, Playwright assertions that the
  caption width > 150 px at 390 px and the skip link's bottom ≤ 0 when
  unfocused.

**Oracle:** `npm run check`, `npm test`, `npm run build`, `npm run test:e2e`,
`./verify-local.sh`; every existing e2e assertion untouched.

**Delivered 2026-11-01.** `apps/web/src/styles/brand.css` (imported between the
kit and `app.css`) declares the dark primitives and aliases; the vendored file is
unchanged at pin `7ca6b795…`. `scripts/check-contrast.mjs` resolves both sources,
writes both hashes and measures 40 pairs (lowest text 5.38:1, lowest UI 3.45:1)
into `docs/evidence/token-contrast.json`. `color-scheme: dark` and
`accent-color` are on `html`. The caption rule and skip-link offset are fixed, and
`e2e/shell.spec.ts` fails if either regresses (canary: caption measured
92.58 px and the skip link peeked 12 px before the fix). The icon is
`apps/web/public/favicon.svg` plus a 32×32 ICO, linked from `index.html`.
Evidence: `docs/branding.md`, `docs/evidence/ui-foundation-browser.md`,
`docs/evidence/ui-foundation-lighthouse*/` and the `ui-foundation-*` captures.
No page was redesigned; package 2 starts from this palette.

---

## Package 2 — `workbench-recommendation`

**Outcome:** `/recommend` reads like the reference workbench — constraints,
options and verdict visible together on a wide screen — without changing one
number the engine produces.

**Requirements**

- R1 ≥1100 px: named grid `constraints | options | verdict`, max 88 rem; below
  1100 px a single column; no horizontal overflow at 320/390/768/1440.
- R2 Each candidate is a spec module: category caps label (decoration), model,
  mono price/stock/score rows, status chip with icon + word.
- R3 A verdict rail shows status, budget delta, stock, score and the rule
  matrix summary; `Gaming requires a discrete graphics card` and the
  no-padding sentence keep their exact text.
- R4 The optional assistant panel is collapsed by default (`<details>`), keeps
  its no-key state and consent copy, and `Continue without AI` focuses the
  budget field.
- R5 Budget accepts grouping separators (`1,800` → 180000 minor units) and keeps
  the `budget_invalid` guidance for genuinely invalid input; Generate is
  disabled while the field is empty or unparsable.
- R6 A shared pluralisation helper replaces `(s)` on this page; money renders one
  way (`US$1,672.90`), and the “why” prose matches it.
- R7 `Options` / `Valid builds` are labelled so the no-padding rule explains
  itself (“2 valid candidates → 1 distinct build”).

**Oracle:** recommendation and keyboard e2e unchanged plus new tests for comma
budget, disabled Generate, collapsed panel and plural forms; a11y 100;
captures.

---

## Package 3 — `verdict-console`

**Outcome:** `/builds/check` and `/diagnose` present rules as a console, and the
two copy defects in their text are fixed at the source.

**Requirements**

- R1 Check page: rule matrix with verdict chip (icon + word), value pair,
  evidence reference and follow-up; summary stays sticky above the table.
- R2 Diagnosis page: group cards (must replace / verify first / fine / not
  verified), evidence table and upgrade plan in the same console language.
- R3 Backend gap copy is grammatical and non-duplicating. `follow_up` must not
  repeat the reason: “Memory is required. Select memory from this snapshot.”
  rather than “memory is required. Select memory is required from this
  snapshot.” Both pages render the reason once.
- R4 The coverage sentence no longer repeats itself when every part was checked
  (“7 of 7 … were checked by at least one rule. Every part was checked …”).
- R5 Token adds/missing lists are hidden for `exact` matches; they stay for
  `partial` and `ambiguous`.
- R6 The `/diagnose` 1–5 skeleton, ids and announced statuses are unchanged.

**Oracle:** compatibility and diagnosis e2e unchanged; a Rust unit test for the
gap sentences and a Vitest assertion that the duplicated string cannot render;
a11y 100; captures.

---

## Package 4 — `dataops-console`

**Outcome:** `/inventory` and `/catalog` read as an operator console: snapshot
state first, provenance visible in every row, actions guarded.

**Requirements**

- R1 Inventory: revision/records/stock-value summary bar, snapshot history as a
  card table, import and export/backup as distinct panels.
- R2 When a snapshot exists, export is available without an unexplained dead
  button: the latest revision opens automatically, or the copy names the
  control that does it.
- R3 Catalog: registry and search results as dense tables with licence,
  version, counts, active state and provenance chips.
- R4 Actions that need input are disabled until valid: Preview CSV, Search
  catalog, Match this list, Diagnose this build.
- R5 Placeholder examples use the shipped sample’s real parts, not
  `Fixture AM4 CPU 5600`; plural forms replace `(s)`.
- R6 Non-goals: no crawler, ETL, sync, third-party fetch or browser-side schema
  editor. The catalog stays an operator upload.

**Oracle:** inventory, catalog and portability e2e unchanged; a11y 100;
captures at 1440/390.

---

## Package 5 — `service-correctness`

**Outcome:** the HTTP surface tells the truth and exported CSV is safe to open.

**Requirements**

- R1 `GET /api` and `GET /api/` return the JSON error envelope with 404, never
  the SPA shell; every unknown `/api/*` stays JSON.
- R2 Only the five client routes and `/` return the shell. Any other
  extensionless path returns 404 with a not-found document (or a documented
  redirect), so a typo cannot masquerade as Inventory with status 200.
- R3 CSV export neutralises formula-leading cells (`=`, `+`, `-`, `@`, TAB, CR)
  with a leading apostrophe, documented as lossy; JSON export keeps the raw
  value. A test proves both.
- R4 A semicolon-delimited CSV refusal names the likely delimiter instead of
  only listing missing columns.

**Oracle:** curl matrix for `/api`, `/api/`, `/nope`, `/recomend`; cargo tests
for the CSV escape; e2e for a real 404; `docs/data-portability.md` and
`docs/inventory-import.md` updated.

---

## Package 6 — `delivery-hardening`

**Outcome:** the same bytes, sent efficiently and with a defensible header
policy for an app that holds private inventory and a BYOK key.

**Requirements**

- R1 Text responses negotiate `gzip`/`br` and declare `Vary: Accept-Encoding`.
- R2 Hashed `/assets/*` are `immutable` for a year; `index.html` and HTML routes
  are `no-cache`.
- R3 Every response carries `X-Content-Type-Options: nosniff`,
  `Referrer-Policy: no-referrer`, `X-Frame-Options: DENY` (or CSP
  `frame-ancestors 'none'`) and a CSP that allows the app bundle, no inline
  script, and the BYOK `connect-src` (https and loopback HTTP).
- R4 Lighthouse performance is measured once in dark and recorded; the result is
  a number in `docs/evidence/`, not a claim.

**Oracle:** header assertions in Rust tests; `npm run test:e2e` still green;
performance report committed.

---

## Findings disposition

| Finding from the review | Package |
| --- | --- |
| Table captions collapse at ≤760 px | 1 (R4) |
| Skip link peeks above the header | 1 (R4) |
| No favicon / 404 on every load | 1 (R5) |
| “Select memory is required…” duplication | 3 (R3) |
| “Every part was checked…” redundancy | 3 (R4) |
| Exact-match token noise | 3 (R5) |
| Assistant panel pushes the form below the fold | 2 (R4) |
| Budget rejects `1,800` | 2 (R5) |
| `unit(s)` / `part(s)` pseudo-plurals | 2 (R6), 4 (R5) |
| Fixture placeholders in the diagnosis form | 4 (R5) |
| Actions enabled with empty input | 2 (R5), 4 (R4) |
| Export dead after a snapshot exists | 4 (R2) |
| “Options 1 / Valid builds 2” ambiguity | 2 (R7) |
| `/api` serves HTML with 200 | 5 (R1) |
| Soft 404 on unknown paths | 5 (R2) |
| CSV formula injection | 5 (R3) |
| Semicolon CSV hint | 5 (R4) |
| No compression | 6 (R1) |
| No cache policy | 6 (R2) |
| No security headers | 6 (R3) |
| Password field outside a form (console warning) | 2 (R4) |
| Dark theme undecided / `color-scheme: light` | 1 (R3, R6) |

## Sequencing and evidence

1. Open one change at a time from this list; keep `current_spec` in
   `HANDOFF.md` pointed at it.
2. Each package must land a usable Web flow plus browser and accessibility
   evidence (`LIGHTHOUSE` desktop + mobile, captures at 1440/390, focus ring
   measured, `scrollWidth === clientWidth` at 390).
3. Each completed package gets exactly two commits: implementation/tests/archive,
   then a `HANDOFF.md`-only pointer/evidence commit (`AGENTS.md`).
4. Do not start package 2–4 before package 1 is archived; they consume its
   tokens and would otherwise fork the palette.

# Workbench recommendation browser evidence

Captured: 2026-11-01, against the working tree based on the commit
`Record dark brand token layer evidence`. The implementation commit and this
evidence are recorded in `HANDOFF.md`.

- Build under test: `npm run build` → `apps/web/dist`
  (`index-BQC-9zbY.js`, `index-CHcFK-Z7.css`), served by the Rust process from the
  same directory.
- Service: `RIGCRAFT_ADDR=127.0.0.1:52473`, `RIGCRAFT_DATA_DIR` a scratch
  directory outside the repository, `./target/debug/rigcraft`.
- Browser: Google Chrome through Chrome DevTools MCP (production build, not the
  Vite dev server). Desktop 1440×900 and mobile 390×844, plus 320 and 768 px
  overflow measurements.
- Data: the shipped sample seeded through `GET /api/imports/sample`,
  `POST /api/imports` (10 valid rows, 0 errors, 0 duplicate rows) and the commit
  for that preview. Snapshot `a9a959cd…e61e`, revision 1, 10 records, USD,
  2,120.90 stock value.
- No AI provider, no API key and no network service other than the local process.

## What this evidence answers

The change rebuilds `/recommend` as a three-region workbench. The measurement
that matters is that the constraints, the options and the verdict are laid out
together at ≥ 1100 px, that the same three regions stack in one column below it,
and that neither state overflows the document. It also proves that the layout
change did not cost the page a fact or a sentence.

## Captures

| File | Viewport | State proved |
| --- | --- | --- |
| `docs/assets/workbench-desktop.png` | 1440×900 | Three columns visible in one viewport: constraints (collapsed assistant + budget form), options (spec module with caps label, `US$1,672.90`, `? Unknown` chip), verdict rail (`2 valid candidates → 1 distinct build`, `Solved`). |
| `docs/assets/workbench-desktop-full.png` | 1440×900 full page | The whole workbench: every reason sentence, parts table, score breakdown, unknown follow-ups, rule evidence, comparison and exports in the options column; rule matrix and use-case rule in the rail. |
| `docs/assets/workbench-mobile-workbench.png` | 390×844 | The single-column workbench at the point where constraints end: the collapsed assistant is one disclosure line and the budget field, use case and part pickers stack at full width. |
| `docs/assets/workbench-mobile-full.png` | 390×844 full page | The same report stacked: constraints, options, verdict, no horizontal overflow. |
| `docs/assets/workbench-mobile.png` | 390×844 | Page top at phone width: header, title, live status, snapshot picker. |

## Layout measurements

Measured in the running page with a generated report
(`1. Best overall`, 1,672.90, `Unknown`).

| Viewport | `grid-template-columns` | `grid-template-areas` | Regions (`x`, `y` top) | `scrollWidth` / `clientWidth` |
| --- | --- | --- | --- | --- |
| 1440 | `336px 736px 304px` | `"constraints options verdict"` | constraints x 16, options x 368, verdict x 1120; all y 560.8 | 1440 / 1440 |
| 1100 (asserted by the Playwright suite) | three columns | `"constraints options verdict"` | x ascending | equal |
| 1099 and below | `736px` single track at 768 (`288px` at 320) | `none` | y ascending, same x | equal at 320, 768, 390 |
| 390 | `358px` | `none` | constraints y 784.8, options y 1835.8, verdict y 9599.1 | 390 / 390 |
| 320 | `288px` | `none` | stacked | 320 / 320 |
| 768 | `736px` | `none` | stacked | 768 / 768 |

- `main` measured 1408 px at 1440 — the 88 rem cap, not a fixed canvas.
- The parts table inside the spec module measured 668 px of content in a 668 px
  wrapper at 1440 (`scrollWidth === clientWidth`), so the numeric columns do not
  hide behind an inner horizontal scroll.
- `e2e/workbench.spec.ts` asserts the same invariants against real Chrome:
  three columns at 1100 and 1440, one column at 390, 320 and 768, and
  `document.documentElement.scrollWidth === clientWidth` at every width.

## Journey exercised

1. Opened `/recommend` at 1440×900 against the seeded sample. The page selected
   revision 1 and announced `Snapshot revision 1 loaded: 10 records, currency
   USD.` The assistant is a closed `<details>`; the tab order to the budget is
   10 stops, and the collapsed assistant contributes exactly one of them (its
   summary).
2. Entered `1,800` (a grouped amount, not `1800`). `Generate recommendations`
   became enabled; the same field kept it disabled while empty and while the
   invalid `1.234` was present, with the inline guidance and `budget_invalid`.
3. Generated. The live region announced
   `Recommendation complete: 1 materially different option within US$1,800.00.`
   and focus moved to the `Recommended builds` heading.
4. Read the verdict rail: `Distinct builds 1`, `Budget US$1,800.00`,
   `Valid candidates 2`, `Status Solved`, the counter
   `2 valid candidates → 1 distinct build`, the top option `1. Best overall`
   with `? Unknown`, `Budget delta US$127.10`, `Stock units 8`,
   `Score 21.36 partial`, and the rule matrix `✓ 12 compatible rules`,
   `✕ 0 conflicts`, `? 2 unknown rules`. The rule heading
   `Gaming requires a discrete graphics card` and the sentence
   `1 materially different build found; no option was padded to reach three.`
   are present with their exact text.
5. Read the options column: the spec module carries the caps label
   `BUILD SPEC · OPTION 1` (aria-hidden), the heading `1. Best overall`, the
   status chip `? Unknown`, and monospaced tabular rows
   `US$1,672.90`, `US$127.10`, `8`, `21.36 partial`. Every part row, every
   score dimension, all five reason sentences, both unknown follow-ups, the
   rule-evidence disclosure, the check report identifier (`Check report`,
   followed by the report UUID), the rule set (`compat-rules-v1`) and the
   snapshot revision are rendered.
6. Money renders one way in the page's own readouts (`US$1,672.90`,
   `US$1,800.00`); the engine's own sentences print their own form
   (`Total 1,672.90 USD leaves 127.10 USD under the budget …`), because that
   sentence is also what the JSON and Markdown exports carry and the e2e suite
   asserts it. The amount is the same; the two spellings are documented in
   `openspec/changes/archive/2026-11-01-workbench-recommendation/design.md` (Decision 5).
7. Opened the assistant: the summary is a keyboard stop with a
   `3px solid rgb(34, 211, 238)` outline at a 2 px offset, the panel expands,
   `#assistant-key` has `type="password"` and a form owner
   (`form.assistant-transport`), and `Continue without AI` moved focus to
   `#recommend-budget` without navigating. The console log was empty — the
   former “password field is not contained in a form” warning is gone.
8. Re-measured at 390, 768 and 320 px with the report on screen; resized back to
   1440 to confirm the three columns return. `scrollWidth === clientWidth` at
   every width.

## Keyboard focus

- First Tab: `Skip to recommendation`, outline `3px solid rgb(34, 211, 238)`,
  offset `2px`, `:focus-visible` true.
- The assistant summary, the budget field, the use-case select, both part
  pickers and `Generate recommendations` are the visible stops; the collapsed
  assistant contributes one stop, not eleven.
- `e2e/keyboard-responsive.spec.ts` drives the whole journey from the keyboard
  at 390 px (Tab to the budget, type, Enter) and still passes.

## Accessibility

Lighthouse via the Chrome DevTools MCP `lighthouse_audit` tool, production
build, `/recommend`:

| Run | Mode | Accessibility | Best practices | SEO | Agentic browsing | Passed / failed |
| --- | --- | --- | --- | --- | --- | --- |
| Desktop, navigation | 1440×900 | 100 | 100 | 100 | 100 | 51 / 0 |
| Mobile, navigation | 390×844 | 100 | 100 | 100 | 100 | 51 / 0 |
| Desktop, snapshot of the generated report | 1440×900 | 100 | 100 | 100 | 100 | 39 / 0 |

Reports: `docs/evidence/workbench-lighthouse/`,
`docs/evidence/workbench-lighthouse-mobile/` and
`docs/evidence/workbench-lighthouse-results/`.

The page adds no colour: the caps labels, spec module, rail and rule matrix use
the archived semantic aliases. `docs/evidence/token-contrast.json` keeps its 40
measured pairs; the lowest text pair is still 5.38:1 and the focus ring is
9.20:1 on the panel surface.

## Privacy review

The capture shows only the shipped sample's parts and the deterministic report.
No provider key was entered, no AI request was made, the assistant stayed
collapsed, and the scratch data directory lives outside the repository. The
console contained no messages.

## Not done

- Lighthouse performance was not measured (this change is layout and copy; the
  scores above are accessibility, best practices, SEO and agentic browsing).
- No visual-diff baseline and no non-Chrome browser.
- No manual screen-reader pass; the accessible names and focus order were
  verified through the accessibility tree, Lighthouse and the keyboard journey.
- The engine's own sentences still contain `unit(s)`/`conflict(s)`; the page
  composes none of them and this change deliberately leaves engine copy
  untouched (see Decision 5). The engine residual was closed later by
  `report-copy-plurals` (`docs/evidence/report-copy-plurals.md`).

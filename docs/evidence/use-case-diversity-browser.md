# Use-case integrity and option diversity browser evidence

Captured: 2026-11-01, against the working tree based on revision `1dc6f7e`
(`Record browser suite implementation evidence`). The implementation commit and this
evidence are recorded in `HANDOFF.md`.

- Build under test: `npm run build` → `apps/web/dist` (`index-GF6gAqO0.js`,
  `index-pkXIEeid.css`), served by the Rust process from the same directory.
- Service: `RIGCRAFT_DATA_DIR=/tmp/opencode/rigcraft-evidence RIGCRAFT_ADDR=127.0.0.1:8111 ./target/debug/rigcraft`
- Browser: Google Chrome through Chrome DevTools MCP (production build, not the Vite
  dev server). Desktop 1440×900 and mobile 390×844.
- Data: the shipped sample seeded through `GET /api/imports/sample` and
  `POST /api/imports` / commit (snapshot revision 1, ten records). The sample is one
  AM5 build whose CPU publishes `igpu=yes` and whose RTX 4070 is the only graphics
  card. It is the exact snapshot that exposed the two defects.
- No AI provider, no API key and no network service other than the local process.

## The defect this evidence answers

Before this change, a gaming request at 1,800.00 USD returned three candidates, all
without a graphics card:

| Rank | Objective | Total | Parts |
| --- | --- | --- | --- |
| 1 | best overall | 1,123.90 USD | cpu, motherboard, ram, psu, case, cooler, storage |
| 2 | alternative | 1,183.90 USD | the same seven parts with a 2 TB drive |
| 3 | alternative | 1,333.90 USD | the same seven parts with a 4 TB drive |

The CPU's sourced `igpu=yes` completed the build, and the three storage capacities
counted as three options. Both facts are wrong for a gaming request and for REQ-R02.

After this change, the same request returns one GPU-inclusive build at 1,672.90 USD
with the 1 TB drive. The 2 TB variant at 1,732.90 is the same machine and is collapsed;
the 4 TB variant is over the budget.

## Captures

| File | Viewport | State proved |
| --- | --- | --- |
| `docs/assets/use-case-gaming-gpu-desktop.png` | 1440×900 | Gaming at 1,800.00: one materially different build, `Gaming requires a discrete graphics card`, RTX 4070 in the parts table, 1,672.90 total, `? Unknown` badge, 2 valid builds. |
| `docs/assets/use-case-gpu-bound-desktop.png` | 1440×900 | Gaming at 1,500.00 below the 1,672.90 GPU-inclusive minimum: `Budget constraint`, 172.90 shortfall, the card named in the headline, a `use_case_requirement` blocker and the cheapest card. |
| `docs/assets/use-case-no-padding-desktop.png` | 1440×900 | Gaming at 1,900.00: all three drives fit (3 valid builds) but the page shows one option and says it was not padded to three. |
| `docs/assets/use-case-gaming-gpu-mobile.png` | 390×844 | The same GPU-inclusive result stacked at phone width; no horizontal overflow. |
| `docs/assets/use-case-gpu-bound-mobile.png` | 390×844 | The GPU-bound no-solution readable at phone width; no horizontal overflow. |
| `docs/assets/use-case-no-padding-mobile.png` | 390×844 | The one-option statement stacked at phone width; no horizontal overflow. |

## Journey exercised

1. Opened `/recommend` against the seeded sample. The page selected revision 1 and
   announced `Snapshot revision 1 loaded: 10 records, currency USD.` The use-case
   field was `Gaming` and its hint now says gaming requires a discrete graphics card.
2. Entered `1800` and generated. The live region announced
   `Recommendation complete: 1 materially different option within US$1,800.00.` Focus
   moved to the `Recommended builds` heading.
3. Read the result: `Options 1`, `Valid builds 2`, `Status Solved`. The report showed
   the rule heading `Gaming requires a discrete graphics card` and the text
   `Gaming requires a discrete graphics card in every option. A CPU with sourced
   igpu=yes does not complete a gaming build.` It then said
   `1 materially different build found; no option was padded to reach three.`
4. Read the option: `1. Best overall`, 1,672.90 USD, 127.10 under budget, 8 stock
   units, `? Unknown`, with the reason
   `Gaming requires a discrete graphics card; this build includes the NVIDIA GeForce
   RTX 4070 Founders Edition.` The parts table contains CPU-7800X3D, MB-B650E-F,
   RAM-FURY-32-6000, GPU-4070-FE, PSU-RM750E, CASE-4000D, COOL-PA120SE and
   SSD-MP600-1TB. The comparison table has one option column.
5. Changed the budget to `1500` and generated. The live region announced
   `No solution. The budget is below the cheapest gaming build with a discrete
   graphics card.` The summary showed `Options 0`, `Valid builds 0`, `Status No
   solution`. The no-solution block showed `Budget constraint`, the sentence
   `Gaming requires a discrete graphics card, so a CPU with sourced igpu=yes alone is
   not eligible. The cheapest parts recorded in this snapshot that can form a complete
   gaming build with a card add up to at least 1,672.90 USD, which is 172.90 USD over
   the 1,500.00 USD budget.`, `What blocked it` naming the RTX 4070 at 549.00 USD, and
   `Raise the budget to at least 1,672.90 USD.` No candidate was shown.
6. Changed the budget to `1900` and generated. All three storage capacities now fit
   (`Valid builds 3`), but the page still showed exactly one option and the same
   no-padding statement: a larger drive is not a different machine.
7. Repeated the three states at 390×844. `document.documentElement.scrollWidth ===
   clientWidth` (390) in every state; no horizontal page overflow.
8. Reloaded at 1440×900 and pressed Tab once. Focus landed on `Skip to recommendation`
   with a solid 3 px `rgb(29, 78, 216)` outline, unchanged from the earlier evidence.

## Accessibility

Lighthouse desktop and mobile (`docs/evidence/use-case-lighthouse/` and
`docs/evidence/use-case-lighthouse-mobile/`): accessibility 100, best practices 100,
SEO 100, agentic browsing 100, 50 audits passed, 0 failed. The new block reuses the
existing `diagnostic-block--unknown` and muted-text token pairs; no colour token was
added, so `docs/evidence/token-contrast.json` keeps its 16 measured pairs and its
lowest pair of 5.46:1. The rule block is a `role="note"` with a heading, the option
count is body text, and the no-solution block keeps its existing `role="group"` and
focus target. All results, announcements, table semantics, the keyboard path and the
390 px stacking are unchanged.

## Executable coverage

`npm run test:e2e` now includes:

- `e2e/recommendation.spec.ts` — a gaming request includes a discrete graphics card and
  states the rule; a budget below the cheapest GPU build names the card, the 1,672.90
  minimum and the 172.90 shortfall; one materially different build is not padded to
  three.
- `e2e/keyboard-responsive.spec.ts` — the keyboard-only gaming journey and the 390 px
  overflow assertion, still green with the GPU-inclusive result.

Rust tests cover gaming excluding an iGPU-only build, office accepting sourced
`igpu=yes`, creation stating that no card was required, the named GPU no-solution,
storage-only collapse, three genuinely distinct builds and determinism. Vitest covers
the rule sentence, the accurate option count and the one-option state.

## Deliberate decisions

- No new binding value: the gaming budget case reports `budget` with the card named,
  and a snapshot with no usable card reports `stock`, so the documented vocabulary
  stays `budget`, `stock`, `compatibility`, `search_limit`.
- Storage is excluded from the material signature because it is the one difference
  REQ-R02 calls out as not material; every other chosen part and quantity counts.
- The representative of a collapsed group is the cheapest drive variant, which is also
  the highest-scoring one under the published cost dimension.
- No request field, scoring weight, work limit, schema version, AI contract, BYOK
  panel or provider adapter changed.

## Not done

Lighthouse performance was not measured (the audited categories above are the ones the
earlier evidence documents report). There are no visual-diff baselines and no
non-Chrome browser.

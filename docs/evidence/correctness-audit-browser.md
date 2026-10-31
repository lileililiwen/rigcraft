# Correctness audit browser and accessibility evidence

Captured: 2026-10-31, against the working tree based on revision `db9b625`
(`Record diagnosis implementation evidence`). The implementation commit and this
evidence are recorded in `HANDOFF.md`.

- Build under test: `npm run build` → `apps/web/dist` (`index-DnRJH6pv.js`,
  `index-ByeIfCIF.css`), served by the same Rust process.
- Service: `RIGCRAFT_ADDR=127.0.0.1:8787 RIGCRAFT_DATA_DIR=/tmp/opencode/rigcraft-evidence ./target/debug/rigcraft`.
  Port 8787 was chosen because 8080, 8098 and 8099 were occupied by unrelated or
  leftover processes that were not touched.
- Browser: Google Chrome through Chrome DevTools MCP (production build, not the Vite
  dev server). Desktop 1440×900 and mobile 390×844.
- Data: `tests/fixtures/diagnosis/upgrade.csv` imported as ordinary user-owned
  inventory; every model name starts with `Fixture`. No licensed catalog data was
  used or implied. The unverified run described a CPU, a graphics card and memory by
  hand, each with specifications, so no rule involving them could run.
- No AI provider, API key or network service was involved.

## Captures

| File | Viewport | State proved |
| --- | --- | --- |
| `docs/assets/correctness-diagnose-skeleton-1440.png` | 1440×900 | Before a list is matched, the page renders sections 1, 2, 3, 4 and 5 in order. Section 3 is present with the empty state `Match a pasted list in step 2 to review its lines here…`; the number 3 is not skipped. |
| `docs/assets/correctness-diagnose-unverified-1440.png` | 1440×900 | CPU + GPU + RAM diagnosed with no applicable rule: summary `0 rules passed · 0 conflicts · 0 unknown · 3 parts not evaluated by any rule`, counters `Parts not evaluated 3`, and a fourth group `Not verified — no rule evaluated (3)`. Each card says the compatibility is not verified; `Fine as is (0)` stays empty. |
| `docs/assets/correctness-diagnose-unverified-390.png` | 390×844 | The same unverified result at phone width with no horizontal overflow. |
| `docs/assets/correctness-static-404-1440.png` | 1440×900 | `/robots.txt` returns a real 404 `Not found.` page, not the SPA shell. |

The audit's "before" state is quoted from the reproduced defect, not presented as a
capture: the same CPU + GPU + RAM diagnosis previously showed counters
`0 rules passed · 0 conflicts · 0 unknown` beside `Fine as is (3)`, and `/diagnose`
previously rendered 1, 2, 4, 5 until a list was matched. Both are after-fix captures
above; no fabricated before image is included.

## Journey exercised

1. Opened `/diagnose` with a committed revision (`Revision 1 · 10 records`). The
   visible headings were `1. Choose an inventory snapshot`, `2. Paste the parts
   list`, `3. Review the matches`, `4. Add a part by hand` and `5. Read the
   diagnosis`. Section 3 announced its empty state and did not take focus.
2. Added three hand-described owned parts through the form: `Fixture AM5 CPU 7600`,
   `Fixture DDR5 16GB Kit` and `Fixture PCIe4 GPU`, with their specifications.
3. Pressed `Diagnose this build`. The result was `Unknown` / `Incomplete build · 5
   missing`, and the summary said `0 rules passed · 0 conflicts · 0 unknown · 3
   parts not evaluated by any rule · 5 required parts are missing`.
4. Read the groups: `Must replace (0)`, `Should upgrade — verify first (0)`,
   `Fine as is (0)` and `Not verified — no rule evaluated (3)`. Each of the three
   cards carried `Not verified`, `Rules: none`, the reason `No rule that could run
   involves this part … Its compatibility is not verified`, and zero keep cost. The
   page also stated `0 of 3 described parts were checked by at least one rule. 3
   parts had no rule that could run and remain unverified.`
5. Repeated the result at 390×844: `document.documentElement.scrollWidth ===
   clientWidth === 390`. No horizontal page overflow.
6. Requested `/assets/nope.js`, `/robots.txt`, `/llms.txt` and `/ai-catalog.json`
   directly. Each returned HTTP 404 `text/plain` with the body `Not found.`; none
   returned `index.html`. `/diagnose` still returned HTTP 200 `text/html`, and
   `/api/nope` returned HTTP 404 `application/json` with `route_not_found`.
7. Read the rendered timestamp on the result: `Checked
   2026-10-31T12:44:37.995Z`, inside a `<time datetime="2026-10-31T12:44:37.995Z">`
   element. The snapshot option showed `2026-10-31T12:43:09.208Z`, also ISO 8601
   UTC. The Vitest locale test sets `navigator.language` to `en-US`, `zh-CN` and
   `ar-EG` and asserts the identical string for each; no browser-locale output
   remains.
8. Pressed Tab once from the loaded page: focus moved to `Skip to build diagnosis`
   with a visible `solid 3px rgb(29, 78, 216)` outline. The changed groups reuse the
   same card, badge and focus structure as the previously keyboard-tested page.

## Accessibility

- Lighthouse desktop (`docs/evidence/correctness-diagnose-lighthouse/`):
  accessibility **100**, best practices **100**, SEO **100**, agentic browsing
  **100**, 49 audits passed, 0 failed.
- Lighthouse mobile (`docs/evidence/correctness-diagnose-lighthouse-mobile/`):
  accessibility **100**, best practices **100**, SEO **100**, agentic browsing
  **100**, 49 audits passed, 0 failed.
- These are recorded as measured, not inferred. The earlier diagnosis run reported
  SEO 91 and agentic browsing 50 because `robots.txt`, `llms.txt` and
  `ai-catalog.json` were served as the SPA shell; those paths now return 404 and
  Lighthouse no longer counts the masquerading HTML. No robots, llms or catalog file
  was invented to raise the score.
- Status remains symbol plus text (`— Not verified`, `✕ Must replace`, `? Should
  upgrade`, `✓ Fine as is`). The new group reuses an existing measured token pair;
  the 15 measured pairs in `docs/evidence/token-contrast.json` pass, lowest 5.46:1
  (error text on surface) and focus ring 7.29:1 on surface.

## Privacy

- Matching and diagnosis consult only the selected local snapshot. No catalog,
  purchase order, provider key, AI request or network service is involved.
- The captured build and specifications are synthetic. Service logs contain request
  metadata only; no pasted line, specification or price is logged.

## Coverage and remaining limits

- The audit's browser findings are now executable: `e2e/regression.spec.ts` holds the
  404 paths, the `invalid_json` envelope, the stable 1–5 `/diagnose` numbering and the
  ISO 8601 UTC timestamp under a `de-DE` locale, and `e2e/diagnosis.spec.ts` holds the
  unverified group with `Fine as is (0)`. The states captured above were performed
  through Chrome DevTools MCP against the production build and remain the visual
  record. See `docs/evidence/browser-suite.md`.
- The Lighthouse runs do not measure performance; they cover accessibility, best
  practices, SEO and the agentic-browsing audits.

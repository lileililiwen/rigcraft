# Build diagnosis browser and accessibility evidence

Captured: 2026-10-31, against the working tree based on revision `6979a44`
(`Record BYOK planning implementation evidence`). The implementation commit and this
evidence are recorded in `HANDOFF.md`.

- Build under test: `npm run build` → `apps/web/dist` (`index-CC1dWSiW.js`,
  `index-ByeIfCIF.css`), served by the same Rust process. The captures below were
  taken against this bundle.
- Service: `RIGCRAFT_DATA_DIR=/tmp/opencode/rigcraft-diag/data RIGCRAFT_ADDR=127.0.0.1:8099 cargo run`;
  the empty-state capture used a second process with an empty data directory on
  port 8098.
- Browser: Google Chrome through Chrome DevTools MCP (production build, not the Vite
  dev server). Desktop 1440×900 and mobile 390×844.
- Data: synthetic fixtures in `tests/fixtures/diagnosis/` (`upgrade.csv`,
  `rejected.csv`, `no-replacement.csv`) imported as ordinary user-owned inventory.
  Every model name starts with `Fixture`; no licensed catalog data was used or
  implied.
- The journey involved no AI provider, no API key and no network service other than
  the local process.

## Captures

| File | Viewport | State proved |
| --- | --- | --- |
| `docs/assets/diagnosis-empty-desktop.png` | 1440×900 | No snapshot exists: the live region announces `No inventory snapshot is available. Import a CSV first.` and the page links to `/inventory`. |
| `docs/assets/diagnosis-entry-desktop.png` | 1440×900 | Snapshot loaded, paste field with the line grammar, currency bound to the snapshot, manual part entry, and the empty result instruction. |
| `docs/assets/diagnosis-review-desktop.png` | 1440×900 | 5 pasted lines: 1 ambiguous GPU with two candidates and no selection, 1 unmatched GPU, 1 exact CPU match preselected, and a repeated line marked `Duplicate — not added` naming its earlier line. |
| `docs/assets/diagnosis-review-resolved-desktop.png` | 1440×900 | The resolve state: GPU `Wind` chosen, the unmatched CPU line left out and added by hand with specifications, duplicate excluded, `7 from the list, 1 by hand`. |
| `docs/assets/diagnosis-result-desktop.png` | 1440×900 | `✕ Incompatible` diagnosis for the full build: 12 rules passed, 1 conflict, 1 unknown; `Must replace (1)` with `cpu-socket` evidence and 249.00 cost, `Fine as is (7)` including the board whose conflict the plan resolves, and `Upgrade plan ✓ Validated` with one step and a compatible whole-plan check. |
| `docs/assets/diagnosis-needs-evidence-desktop.png` | 1440×900 | Same build with a hand-described 240 mm AIO: `Must replace (1)`, `Should upgrade — verify first (2)` for the missing front-radiator clearance, `Fine as is (5)`, and `Upgrade plan ? Needs evidence` with the follow-ups and no validity claim. |
| `docs/assets/diagnosis-no-validated-plan-desktop.png` | 1440×900 | Two conflicts whose cheapest individual fixes contradict each other: 3 parts must replace, `Upgrade plan ✕ No validated plan`, zero steps, 3 substitutions evaluated, rejected candidates and their rules listed. |
| `docs/assets/diagnosis-review-mobile.png` | 390×844 | Match review stacks at phone width: radio candidates, added/missing tokens, the disambiguation text and the manual fallback. |
| `docs/assets/diagnosis-review-blocked-mobile.png` | 390×844 | Unresolved review blocked by a `role="alert"`: `Line 1 has no match … Line 4 is ambiguous …`, `review_incomplete`, focus on the alert. |
| `docs/assets/diagnosis-result-mobile.png` | 390×844 | Keyboard-only diagnosis at phone width: validated plan, cost line and group cards without horizontal overflow. |
| `docs/assets/diagnosis-no-validated-plan-mobile.png` | 390×844 | Rejected-plan state at phone width; no steps are presented as valid. |
| `docs/assets/diagnosis-focus-desktop.png` | 1440×900 | Skip link focused with a visible 3 px outline after one Tab from load. |

## Journey exercised

1. Opened `/diagnose` with an empty data directory: the live region announced
   `No inventory snapshot is available. Import a CSV first.` and the page linked to
   `/inventory`.
2. Against the empty service, the paste section, manual form and diagnosis action are
   not offered; the empty state is a real branch, not a blank panel.
3. With `upgrade.csv` selected (`Revision 1 · 10 records`), pasted eight lines:
   six exact matches, one ambiguous GPU and one CPU absent from the snapshot. The
   live region announced `Match review ready: 8 line(s). 6 matched, 1 ambiguous, 1
   without a match, 0 needing a fix. Resolve every line before diagnosing.`
4. Read the review. The ambiguous line `gpu: Fixture GeForce RTX 4070` had no
   selected radio; both candidates were listed with `adds twin` / `adds wind` and
   the disambiguation said `The closest records add tokens you did not paste: twin,
   wind. Confirm the full model, add one of those tokens, or paste the SKU.` The
   exact CPU line was preselected as a suggestion. A repeated input line was marked
   `Duplicate — not added` with `Line 4 already describes this part`.
5. Pressed `Add it by hand instead` on the unmatched CPU line: the manual form was
   prefilled with the category and model, focus moved to the category select, and
   after entering `socket=AM4;tdp=65W;peak_power=88W;igpu=no` the part appeared as
   `CPU · Fixture AM4 CPU 5600 · specifications recorded`.
6. Chose `Wind` for the GPU and diagnosed. The result was `✕ Incompatible`: 12 rules
   passed, 1 conflict (`cpu-socket`: `CPU declares Socket AM4 but the motherboard
   declares Socket AM5`), 1 unknown (`cpu-bios`). The groups separated
   `Must replace (1)` (the CPU, 249.00 replacement, `cpu-socket` evidence,
   `Compatibility evidence for OWNED-08`) from `Fine as is (7)`, including the
   motherboard whose conflict is resolved by replacing the CPU.
7. Read the plan: `✓ Validated`, one step `Fixture AM4 CPU 5600 (OWNED-08)` →
   `Fixture AM5 CPU 7600 (FIX-AM5-CPU-7600)` at 249.00, additional cost 249.00,
   `whole-plan check: compatible`, 2 substitutions evaluated. Kept parts showed
   `Keep cost US$0.00`, proving the retained parts are not charged again.
8. Replaced the matched air cooler with a hand-described 240 mm AIO whose radiator
   bay clearance the case does not record, and diagnosed again: `Must replace (1)`,
   `Should upgrade — verify first (2)` for the cooler and case with the exact
   clearance question, `Fine as is (5)`, and `Upgrade plan ? Needs evidence` with
   the follow-ups. The page never called that plan valid.
9. Switched to `rejected.csv` and pasted its eight parts. All matched exactly. The
   result was 3 parts must replace and `Upgrade plan ✕ No validated plan`: zero
   steps, zero additional cost, 3 substitutions evaluated, and a rejected-candidate
   list naming the rules that rejected each. The live region announced
   `... Upgrade plan: no valid plan.`
10. Rejected the empty and incomplete reviews: submitting with an unmatched and an
    ambiguous line produced the `role="alert"` `Resolve the match review first` with
    the two line messages and code `review_incomplete`, and focus moved to the
    alert. No diagnosis was stored for a rejected request.
11. Repeated the review, result, needs-evidence and rejected-plan states at 390×844.
    `document.documentElement.scrollWidth === clientWidth` (390) in every state; no
    horizontal page overflow. `diagnosis-review-blocked-mobile.png` records the
    390 px alert state.
12. Parsing, review and diagnosis each write only their own result: parsing writes
    nothing, and a diagnosis stores one `diagnoses` row without a `check_reports`
    row for the synthetic described build (asserted in `tests/diagnosis_api.rs`).

## Keyboard and focus

Real Tab, Shift+Tab, ArrowRight and Enter presses through the production build. The
first six Tabs from load reached the snapshot select; no pointer was used after the
page loaded. The sequence below was performed at 390×844.

| Step | Focused element | Result |
| --- | --- | --- |
| 1 | `Skip to build diagnosis` | visible focus: `solid 3px rgb(29, 78, 216)`, link moved to `top: 8px` |
| 2–6 | brand, Inventory, Compatibility, Recommend, Diagnose | visible 3 px focus ring |
| 7 | `Snapshot` select | ArrowDown twice changed the selection to `upgrade.csv` and loaded it |
| 8 | `Parts list` textarea | typed seven lines |
| 9 | `Currency for pasted prices` | unchanged `USD` |
| 10 | `Match this list`, Enter | review heading `3. Review the matches` received focus; live region announced the counts |
| 11–13 | line 1 checked radio, line 2 checked radio, line 3 first GPU radio | radio groups reached in order |
| 14 | line 3 GPU radio | ArrowRight selected `FIX-GPU-4070-WIND`; live region announced `Line 3 will use FIX-GPU-4070-WIND.` |
| 15–19 | remaining checked radios and the line-7 leave-out radio | Tab order follows the document |
| 20 | `Category` select in the manual form | reached after the review |
| 21 | `Model` input | typed `Fixture AM4 CPU 5600` |
| 22 | `Quantity` input | `1` |
| 23 | `Specifications you have confirmed` | typed `socket=AM4;tdp=65W;peak_power=88W;igpu=no` |
| 24 | `Add this part`, Enter | part added; status announced the addition |
| 25 | `Diagnose this build`, Enter | diagnosis ran |
| Result | `Build diagnosis` heading (`tabIndex=-1`) | focus moved to the result; live region announced `Diagnosis complete: incompatible. 1 part(s) must be replaced, 0 should be verified, 7 can be kept. Upgrade plan: validated.` |

- No pointer gesture was used to select the snapshot, fill the pasted list, resolve
  the ambiguous line, enter the owned part, or run the diagnosis.
- Each radio group has one tab stop; arrow keys move within it, as the browser
  expects. Every leave-out decision is a radio option, so a line is never dropped
  without an explicit keyboard-reachable choice.
- Focus moves to the match-review heading after matching, to the error alert after a
  blocked or failed request, and to the result heading after a completed diagnosis.

## Responsive layout

- Review lines stack into bordered fieldsets; candidate labels wrap the model,
  price, stock and the added/missing tokens instead of forcing a wide row.
- Advice groups are single-column cards; the plan table reuses the shared
  `responsive-table` stacking pattern with `data-label` cells.
- `scrollWidth === clientWidth === 390` in the review, blocked-review, result,
  needs-evidence and rejected-plan states. No horizontal overflow was found.

## Accessibility

- Lighthouse accessibility **100** with **0 failing accessibility audits** on
  desktop and mobile (`docs/evidence/diagnosis-lighthouse/` and
  `docs/evidence/diagnosis-lighthouse-mobile/`).
- The same runs reported SEO 91 and agentic browsing 50. Those are not
  accessibility results and are recorded as not clean rather than presented as
  passing: `robots.txt` is invalid (17 errors), `llms.txt` is absent, and
  `ai-catalog.json` is absent. None of them affects this page's accessible use.
- Status is symbol plus text (`✕ Incompatible`, `? Ambiguous`, `✓ Matched`,
  `— Nothing to replace`), never colour alone. Group headings carry counts.
- The page introduces no new colour token: the 15 measured token pairs in
  `docs/evidence/token-contrast.json` are unchanged, with the lowest pair 5.46:1
  (error text on surface) and the focus ring 7.29:1 on surface, both above WCAG 2.2
  AA.
- Reduced motion: the shared `prefers-reduced-motion` rule disables the status
  spinner animation; the page adds no transition of its own.

## Privacy

- Matching and diagnosis run only against the selected local snapshot. No catalog,
  purchase order, provider key, AI request or network service is involved.
- Pasted text is sent only to `/api/diagnoses/parse` and is not stored; it is not
  included in any diagnosis report. The stored report contains the described build,
  the matched SKUs with their provenance, the check evidence and the plan.
- Service logs contain request metadata only; no pasted line, specification or price
  is logged. The captured fixtures are synthetic.

## Coverage

- The browser journey is now executable: `e2e/diagnosis.spec.ts` replays the exact,
  ambiguous and unmatched lines, the blocked review and the unverified group with
  `Fine as is (0)` on every run; `e2e/regression.spec.ts` holds the 1–5 numbering
  and `e2e/keyboard-responsive.spec.ts` the 390 px width. The accessibility (Lighthouse) runs above were performed
  through Chrome DevTools MCP against the production build and remain the accessibility
  record. See `docs/evidence/browser-suite.md`.

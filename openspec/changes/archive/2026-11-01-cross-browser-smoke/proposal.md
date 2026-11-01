# Proposal: Firefox and WebKit smoke, and an assistive-technology review

## Why

The full browser suite has only ever run in Chrome. Every browser-evidence
document since the first package ends with the same residual: no non-Chrome
browser. A Chromium-only suite cannot see a WebKit focus quirk, a Firefox
download filename or a layout that overflows in a different engine, and the
committed pixel baselines are inherently Chrome's.

The same documents also say "no manual screen-reader pass". That pass needs a
human, but nobody has recorded the accessibility tree, the focus order or the
live-region announcements a human would start from. Without that record the
screen-reader gap cannot even be scoped.

## What Changes

- **A smoke subset runs outside Chrome.** `e2e/smoke.spec.ts` proves the six
  client routes serve the shell with their own titles, the skip link is the
  first Tab stop and moves focus to `main`, no route overflows at 390 px, a typo
  path is the real `text/plain` 404, a 1,800.00 recommendation generates from
  the seeded sample with the gaming rule, and the inventory CSV downloads with
  its revision-bearing filename. It uses the existing `seedSample`,
  `chooseSnapshot` and `expectNoHorizontalOverflow` helpers, so no state set-up
  is duplicated.
- **Two new Playwright projects.** `firefox` and `webkit` match only
  `smoke.spec.ts`; the `chromium` project keeps the whole suite, including the
  twelve committed visual baselines. `channel: 'chrome'` moves from the global
  `use` block into the chromium project so the other engines do not inherit it.
  No project is skipped and no Chrome journey is replaced.
- **The browsers are installed and proved.** `npx playwright install firefox
  webkit` downloads Firefox 157.0 and WebKit 27.2. WebKit's host libraries are
  installed user-locally (no sudo), so both engines genuinely run; the exact
  provisioning commands and the initial loader error are recorded.
- **An assistive-technology review.** `docs/evidence/assistive-technology-review.md`
  records, for each route, the accessibility tree, landmarks, heading order, the
  focus order through the primary journey, every live region and its
  announcement, and the role/name gaps found. It states at the top that it is
  **not** a screen-reader pass and lists the exact human script a NVDA, VoiceOver
  or JAWS pass still has to run.
- **Evidence.** `docs/evidence/cross-browser-smoke.md` records the browser
  matrix, the exact install and run output, the run times, the Chrome full-suite
  count and the smoke coverage list.

## Impact

- Affected specs: `browser-regression` gains one requirement (the Firefox/WebKit
  smoke projects and the honest unavailable verdict); `ui-foundation` gains one
  requirement (the assistive-technology review and its human hand-over). No
  existing requirement is modified or removed.
- Affected code: `e2e/smoke.spec.ts` (new), `playwright.config.ts` (two projects,
  `channel` moved), `.github/workflows/forge-gate.yml` (install the two browsers
  on the runner before `verify-local.sh`), `docs/evidence/cross-browser-smoke.md`
  and `docs/evidence/assistive-technology-review.md` (new), `README.md`,
  `docs/evidence/browser-suite.md`, `docs/gap-closure.md` status and
  `AGENTS.md` Project facts.
- Affected behaviour: none. No application source, colour, layout, copy, API,
  route, schema or dependency changes; no baseline is added for Firefox or
  WebKit.
- Non-goals: no cross-engine pixel baselines, no full Firefox/WebKit journey
  parity, no new dependency (`@playwright/test` already ships both engines), and
  no claim that the application has passed a screen reader.

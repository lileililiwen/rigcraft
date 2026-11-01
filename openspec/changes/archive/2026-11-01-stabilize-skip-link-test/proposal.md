# Proposal: Stabilise the skip-link focus test

## Why

`./verify-local.sh` failed once on `e2e/landmarks.spec.ts` — “the first Tab stop is
the skip link” saw an inactive element — while the same test passed 32
consecutive isolated runs and two full-suite runs. The skip link itself is
correct: the test presses Tab immediately after `page.goto`, before the new page
is guaranteed to be the frontmost target, so the first key event can be delivered
to a page that is still closing. A keyboard assertion that can fail for a
harness-focus reason is not evidence, and a flaky accessibility test is worse than
none because it teaches the reader to rerun it.

## What Changes

- Bring the page to front and wait for the rendered `h1` before the first
  `keyboard.press`, so the key event targets a settled, frontmost page.
- Keep the assertions unchanged: the skip link is the first Tab stop, and Enter
  moves focus to the named `main` region.
- Record the repetition evidence (50 targeted runs and two full-suite runs) in
  `docs/evidence/`.

## Impact

- Affected specs: `browser-regression`.
- Affected code: `e2e/landmarks.spec.ts`, evidence, `HANDOFF.md`.
- No application code, API, style, route or dependency changes.

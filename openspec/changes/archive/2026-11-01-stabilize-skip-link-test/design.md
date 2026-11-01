# Design: Stabilise the skip-link focus test

## Context

The failure was reported by `verify-local.sh` in a full run (61 passed, 1
failed). The same test passed 12 targeted repeats and 20 repeats of the whole
`landmarks.spec.ts` file, and two full 62-test runs afterwards. The failing
assertion was `toBeFocused()` on `<a class="skip-link">`, which resolved and
stayed inactive for the full 10 s timeout.

The skip link is the first focusable element in the DOM, and the application does
not move focus on `/inventory` load (the auto-open sets `suppressFocus` before
the request and skips the post-commit focus). No application defect was found or
is claimed.

## Decision — make the setup deterministic, not the assertion

```ts
await page.goto("/inventory");
await expect(page.getByRole("heading", { name: "Inventory", level: 1 })).toBeVisible();
await page.bringToFront();
await page.keyboard.press("Tab");
```

- Waiting for the `h1` proves the SPA rendered; it does not assume any database
  state.
- `bringToFront()` makes the page the active target before the first key event,
  which is the condition a real keyboard user is in. Playwright creates a page per
  test; the previous page can still be closing when the first key is dispatched.
- The assertions stay exactly as they were; nothing is retried, loosened or
  skipped.

## Rejected alternatives

- **Retry the Tab press until it focuses.** That hides a real focus failure and
  would pass even if the page stole focus back.
- **Seed a snapshot first.** The skip link does not depend on inventory data;
  adding a seed would couple an unrelated fixture to the assertion.
- **Remove or skip the test.** It is the only proof that the first Tab stop is the
  skip link; a skip would retire the question.

## Evidence

50 targeted repeats of the skip-link test and two full-suite runs, recorded in
`docs/evidence/stabilize-skip-link-test.md`.

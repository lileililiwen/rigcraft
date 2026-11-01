# Skip-link focus stabilisation evidence

## The failure

`./verify-local.sh` reported one failure in a 62-test run:

```
[chromium] › e2e/landmarks.spec.ts:76 › the skip link still lands in the named main region
Error: the first Tab stop is the skip link
Received: inactive
```

The locator resolved to the real `<a class="skip-link" href="#main">` and the
element stayed inactive for the full 10 s timeout. The page did not steal focus:
`/inventory` sets `suppressFocus` before the initial snapshot request and skips the
post-commit focus move, and no application code focuses on load.

## Bounding it

| Run | Result |
| --- | --- |
| Skip-link test alone, `--repeat-each=12` | 12 passed |
| Whole `landmarks.spec.ts`, `--repeat-each=5` | 20 passed |
| Full suite before the change, twice | 62 passed twice |
| Full suite at the failing rehearsal | 61 passed, 1 failed |

A failure that appears once across four full runs and never in 32 isolated runs is
a harness-focus race, not a repeatable application defect.

## The change

Before the first key event the test now waits for the rendered `h1` and calls
`page.bringToFront()`:

```ts
await page.goto("/inventory");
await expect(page.getByRole("heading", { name: "Inventory", level: 1 })).toBeVisible();
await page.bringToFront();
await page.keyboard.press("Tab");
```

The assertions are unchanged: the skip link is focused after one Tab, and Enter
moves focus to `main` named by the page heading. There is no retry, no loosened
matcher, no skip and no data seed.

## After the change

| Run | Result |
| --- | --- |
| Skip-link test, `--repeat-each=50` | **50 passed** (52.0 s) |
| Full suite run 1 | **62 passed** (46.1 s) |
| Full suite run 2 | **62 passed** (47.5 s) |
| `./verify-local.sh` | **all local checks passed** |

Commands:

```sh
node e2e/run.mjs e2e/landmarks.spec.ts --project=chromium --grep "skip link" --repeat-each=50
npm run test:e2e
./verify-local.sh
```

No visual baseline was regenerated; no application file changed.

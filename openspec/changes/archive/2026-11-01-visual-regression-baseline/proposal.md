# Proposal: Pixel baselines that fail on an unintended visual change

## Why

Every prior package recorded the same residual: "no visual-diff baseline". The
browser suite asserts structure, text and layout numbers, but nothing compares
the rendered pixels, so a token edit, a spacing regression or an accidentally
hidden control can pass every assertion. A screenshot that is regenerated on
every run would not help either: it has to be committed, deterministic, and
compared.

## What Changes

- **Twelve committed baselines.** `e2e/visual.spec.ts` captures `/inventory`,
  `/catalog`, `/builds/check`, `/recommend`, `/diagnose` and the real 404 at
  `/nope`, at 1440×900 and 390×844, with Playwright's `toHaveScreenshot()`.
  Screenshots are viewport captures, not full-page, so a baseline is small and a
  long records table cannot make it brittle.
- **State is seeded immediately before each capture.** Each test commits the
  shipped sample through the existing `seedSample` helper or resets the catalog
  registry through the documented API and uploads the synthetic fixture. A fresh
  import is always revision 1, so the baseline does not carry a growing revision
  number. A second run in the same database produces the same image.
- **Dynamic values are masked by locator.** Snapshot and report identifiers,
  committed/checked timestamps, the sha256 fragment and the `<select>` label that
  embeds a timestamp are covered by mask locators on block-level containers.
  There is no blanket tolerance standing in for a mask.
- **A bounded tolerance only for antialiasing.** `maxDiffPixelRatio` is `0.01`,
  `threshold` is `0.2`, animations are disabled and the caret is hidden.
- **The visual spec runs inside the suite.** `npm run test:e2e` runs it with the
  rest of the journeys; there is no opt-in flag and no skipped project. A changed
  pixel beyond the bound fails with the diff artifact.
- **Deliberate updates are documented.** `npm run test:e2e -- --update-snapshots`
  regenerates the baselines, and the README and evidence document state that this
  is legitimate only for an intended visual change, reviewed by a human, in the
  same change.

## Impact

- Affected specs: `ui-foundation` gains the interface-evidence requirement (the
  baseline set, the mask policy and the bounded threshold); `browser-regression`
  gains the suite requirement (the visual spec runs in the declared command and
  fails with an artifact). No existing requirement is modified or removed.
- Affected code: `e2e/visual.spec.ts` (new), `e2e/fixtures.ts` (the synthetic
  catalog builder and the registry reset move or are added), the committed
  `e2e/visual.spec.ts-snapshots/*.png`, `README.md`,
  `docs/evidence/visual-regression.md` and `docs/evidence/browser-suite.md`.
- Affected behaviour: none in the product. No application source, copy, colour,
  layout, route, API, schema or dependency changes; the baselines are generated
  from the unmodified production build.
- Non-goals: no full-page baselines, no cross-browser baselines (WebKit and
  Firefox are package 4), no visual review service and no new dependency —
  `@playwright/test` already provides `toHaveScreenshot()`.
- Known limitation: baselines are platform-sensitive. The committed set was
  generated on Linux with real Chrome; a different OS, GPU or font stack needs
  its own baseline set, and the filename carries the `chromium-linux` suffix.

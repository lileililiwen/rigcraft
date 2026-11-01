# Proposal: Measured dark token layer and a defect-free shell

## Why

`docs/branding.md` records the open decision plainly: “Until a dark token set
exists and is measured, the UI declares `color-scheme: light` rather than
guessing dark values.” The Stitch reference (`Kinetic Silicon`) now supplies a
credible direction — a dark instrument panel with cyan interaction, emerald
verification and rust warnings — but it cannot be pasted in. The vendored token
file is generated and hash-pinned, the contrast checker reads only that file, and
the UI conformance scan expects no colour literals outside the token layer.

Two shell defects also survived the last audit because no gate looks at them: at
≤ 760 px every table caption collapses to a word-per-line column (93 px wide at
390 px), and the skip link is positioned at `-2rem` inside a 44 px box, so 12 px
of it stays visible as an empty rounded rectangle above the header on every
route. Both live in the same stylesheet this change rewrites.

This change lands the dark token set as a *measured app-owned layer* on top of
the untouched kit, fixes those two defects where the rules are rewritten, and
gives the app an icon. It deliberately does not redesign a page so the palette
can be reviewed and measured before any screen depends on it.

## What Changes

- Add `apps/web/src/styles/brand.css`, imported after the vendored
  `tokens.css` and before `app.css`, declaring `--brand-*` primitives and the
  semantic aliases for a dark substrate with three tonal surfaces, a cyan
  primary, emerald success and rust warning, plus subtle borders.
- Declare `color-scheme: dark` and set `accent-color`, so native controls,
  scrollbars and autofill remain legible.
- Extend `scripts/check-contrast.mjs` to resolve the effective theme across
  **both** stylesheets, record both file hashes in
  `docs/evidence/token-contrast.json`, and measure the new dark pairs: muted text
  on page/surface/raised, chip text on each tint, primary/secondary/tertiary on
  each surface, borders and the focus ring on each surface. Any pair below its
  WCAG 2.2 AA minimum fails the check.
- Fix the shell rules while they are being rewritten:
  - `.responsive-table caption` becomes a block full-width caption inside the
    ≤ 760 px card layout.
  - `.skip-link` is fully off-screen when unfocused (its own height plus an
    offset, not a fixed `-2rem`) and returns to `top: var(--space-2)` on focus.
- Add a favicon (SVG plus ICO fallback) under `apps/web/public/`, link it from
  `index.html`, and serve it with a cache policy so `/favicon.ico` stops 404ing.
- Update `docs/branding.md` with the decided set, the measured table and the
  light/dark position, and record the reference take/decline boundary in
  `docs/design-program.md`.
- Add browser evidence and two Playwright assertions that would have caught the
  shell defects: the caption's rendered width at 390 px, and the skip link's
  bottom edge while unfocused.

No API, no schema, no dependency, no route, no engine behaviour and no visible
copy change.

## Impact

- Affected specs: `ui-foundation` (new).
- Affected code: `apps/web/src/styles/brand.css` (new),
  `apps/web/src/styles/app.css`, `apps/web/src/main.tsx` (import order),
  `apps/web/index.html`, `apps/web/public/` (new), `scripts/check-contrast.mjs`,
  `docs/branding.md`, `docs/design-program.md`, `docs/evidence/token-contrast.json`,
  `e2e/keyboard-responsive.spec.ts` (or a new shell spec), `docs/assets/`.
- Unchanged: `apps/web/src/styles/vendor/tokens.css` and its `pin_sha256`;
  every component; every API; every e2e text assertion.

# RigCraft branding baseline

RigCraft is the provisional product name. The product consumes the vendored shared design tokens in `apps/web/src/styles/vendor/tokens.css`; no brand-specific palette has been introduced.

## Decided now

- Type: the token stack `Outfit, system-ui, sans-serif`, with token body/small/large/title sizes.
- Color: the shared slate, blue, red, green and amber token values with semantic aliases (`background`, `surface`, `foreground`, `muted`, `primary`, `danger`, `success`, `warning`, `border`, `focus-ring`). The UI uses variables only; it adds no color literals.
- Controls: token minimum heights (2.5 rem) and radii.
- Status language: text plus symbol (`✓ Valid`, `✕ Error`, `⇄ Duplicate SKU`, `? Unknown`), never color alone.

## Measured contrast

`node scripts/check-contrast.mjs` measures every pair the inventory UI uses and writes `docs/evidence/token-contrast.json`. All pairs pass WCAG 2.2 AA; the lowest is error text on surface at 5.46:1 (required 4.5:1), and the focus ring is 7.29:1 on surface (required 3:1).

## Still open

A final brand mark, product renaming, a dark theme and any marketing palette remain undecided. Until a dark token set exists and is measured, the UI declares `color-scheme: light` rather than guessing dark values.

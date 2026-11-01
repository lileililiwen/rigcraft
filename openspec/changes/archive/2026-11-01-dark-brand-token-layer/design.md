# Design: Measured dark token layer and a defect-free shell

## Context

`apps/web/src/styles/vendor/tokens.css` is generated from `ui/tokens/tokens.json`,
imported by every route and pinned in `.project.json`:

```json
"shared_layer": { "adopted": true,
  "consume_paths": ["apps/web/src/styles/vendor/tokens.css"],
  "pin_sha256": "7ca6b7956b3f6b475254b4a5d0cf599724e4e1af5f0f88aba4fbccb9f38fc19b" }
```

It exposes primitives (slate/blue/red/green/amber, spacing, radii, type) and one
light semantic set plus a `[data-theme="dark"]` set. `scripts/check-contrast.mjs`
reads only that file, resolves `var()` aliases, measures 16 pairs and writes
`docs/evidence/token-contrast.json`; `npm run check` fails below AA.

## Decision 1 — a brand layer, not an edit

Add `apps/web/src/styles/brand.css` and import it in this order:

```
vendor/tokens.css → brand.css → app.css
```

`brand.css` may only declare custom properties: `--brand-*` primitives and the
semantic aliases the components already consume (`--color-background`,
`--color-surface`, `--color-foreground`, `--color-muted`,
`--color-muted-foreground`, `--color-primary`, `--color-primary-hover`,
`--color-on-primary`, `--color-border`, `--color-danger`, `--color-success`,
`--color-warning`, `--focus-ring`) plus the new surface step
(`--color-surface-raised`) and a decorative border
(`--color-border-subtle`). It declares no selectors other than `:root`.

The vendored file stays byte-identical so `pin_sha256` and the conformance scan
keep their meaning. A future kit bump is a separate governance change.

## Decision 2 — dark is the brand mode; light stays the kit fallback

The reference is dark-only and the product is used in a single-session
instrument-panel context. `:root` therefore declares `color-scheme: dark` and the
dark aliases. The kit's `[data-theme="dark"]` block becomes redundant but is not
removed (it is generated). No theme toggle ships: an unmeasured second theme is
how contrast claims rot. `docs/branding.md` states the position and records that
a light theme would need its own measured table before it can return.

## Decision 3 — measure the effective theme

`scripts/check-contrast.mjs` changes from one file to an ordered list:

```
apps/web/src/styles/vendor/tokens.css
apps/web/src/styles/brand.css
```

Later declarations win, exactly as in the cascade. The report gains one entry per
file (`{file, sha256}`) and keeps the pair schema. Text-bearing pairs keep flat
values so the resolver can compute them; `color-mix()` and alpha are reserved
for decoration (`--color-border-subtle`, hover washes, glows) and are never
measured as a text pair.

Pairs to add (all AA, 4.5:1 for text, 3:1 for UI):

- foreground on background / surface / raised
- muted-foreground on background / surface / raised
- primary, success, warning, danger on surface and raised
- chip text on each chip tint (verified, warning, unknown, neutral)
- border-subtle and focus ring on background / surface / raised
- primary button label on primary and on primary-hover

## Decision 4 — shell fixes in the same stylesheet

Both defects are rule-level and independent of the palette:

| Defect | Current rule | New rule |
| --- | --- | --- |
| Caption collapse | `.responsive-table { display: block }` leaves `<caption>` shrink-wrapped | `.responsive-table caption { display: block; width: 100%; text-align: left; }` inside the same media query |
| Skip link peek | `top: calc(-1 * var(--space-8))` on a 44 px box leaves 12 px visible | `top: calc(-1 * (2 * var(--space-4) + 2.75rem))` or `translateY(calc(-100% - var(--space-4)))`, full height off-screen, restored on `:focus` |

A Playwright assertion pins each fix so a later refactor cannot reintroduce it:

```ts
const caption = page.locator("caption").first();
expect((await caption.boundingBox())!.width).toBeGreaterThan(150);
const skip = page.getByRole("link", { name: /Skip to/ });
expect((await skip.boundingBox())!.y + (await skip.boundingBox())!.height).toBeLessThanOrEqual(0);
```

## Decision 5 — favicon

`apps/web/public/favicon.svg` (simple cyan-on-carbon mark, no text) plus a 32×32
ICO fallback generated from it, linked from `index.html`. Vite copies `public/`
into `dist/`; the existing static fallback serves it, and `delivery-hardening`
later adds the immutable cache policy. Until then the response inherits the
asset behaviour, which is enough to remove the 404 and the console error.

## Alternatives considered

- **Edit the vendored file.** Rejected: breaks the pin and the generated-file
  contract; the next kit sync overwrites it.
- **Ship a full dark *and* light table now.** Rejected: doubles the measured
  surface, and the reference gives no light direction; the kit fallback remains
  for future work.
- **Put brand values in `app.css`.** Rejected: the contrast checker and the
  conformance scan need one obvious token file; mixing values into component
  rules is how literals leak back in.
- **Use `color-mix()` for every surface.** Rejected for text pairs: the checker
  cannot resolve it, so the pair would be documented rather than measured.
- **Ship JetBrains Mono.** Rejected for this package: it is a downloaded font
  the app does not currently need; the system mono stack renders the same
  telemetry with tabular numerals.

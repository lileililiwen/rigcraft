# RigCraft branding baseline

RigCraft is the provisional product name. The product consumes the vendored
shared design tokens in `apps/web/src/styles/vendor/tokens.css` (generated and
hash-pinned) and layers one app-owned brand token file on top:
`apps/web/src/styles/brand.css`. Import order is
`vendor/tokens.css → brand.css → app.css`, so a brand declaration wins by the
cascade while the vendored file stays byte-identical to
`.project.json → ui.shared_layer.pin_sha256`
(`7ca6b7956b3f6b475254b4a5d0cf599724e4e1af5f0f88aba4fbccb9f38fc19b`).

`brand.css` declares `:root` custom properties only — `--brand-*` primitives and
the semantic aliases the components consume. A component never references a
primitive, and no component stylesheet contains a colour literal.

## Decided now

### Type

| Role | Stack |
| --- | --- |
| UI | `system-ui, -apple-system, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif` |
| Numeric readouts, SKUs, specifications | `ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace` with `font-variant-numeric: tabular-nums` |

Body stays 16 px, meta is 13–14 px, and no content text is 10 px. A caps
micro-label (`BUILD SPEC · OPTION 1`, `CONSTRAINTS`) is 12 px decoration: it is
`aria-hidden` and the heading beside it always carries the same meaning, so no
reading depends on it. No webfont is
downloaded and no font file ships, so the first paint does not wait on a network
font and phones pay nothing for it.

### Dark substrate and three surfaces

Elevation is tonal, not a shadow. The page is darkest; a panel is one step up; a
module nested inside a panel is one step above that.

| Semantic alias | Value | Role |
| --- | --- | --- |
| `--color-background` | `#0a1224` | Page substrate |
| `--color-muted` | `#0e1729` | Recessed strips: status line, summary cells, disabled controls |
| `--color-surface` | `#141e33` | Panel, header, card |
| `--color-surface-raised` | `#1b2740` | Module nested in a panel |
| `--color-border` | `#6b7a99` | 1 px control and table border (measured UI pair) |
| `--color-border-subtle` | `color-mix(in srgb, #6b7a99 38%, transparent)` | Decorative separator only; never carries text |
| `--color-foreground` | `#e6edf7` | Body text |
| `--color-muted-foreground` | `#b9c5da` | Secondary text, labels |

### Interaction and status

Cyan is interaction, emerald is verified, rust is warning and red is an error.
A status is always icon + word + border; the colour is an reinforcement, never
the only signal.

| Semantic alias | Value | Role |
| --- | --- | --- |
| `--color-primary` | `#06b6d4` | Link, primary button, active nav rule |
| `--color-primary-hover` | `#22d3ee` | Hover and focus-adjacent accent |
| `--color-on-primary` | `#04121c` | Primary button label |
| `--color-success` | `#34d399` | Valid / confirmed / compatible / keep |
| `--color-warning` | `#fb923c` | Warning, duplicate, unknown, should-upgrade |
| `--color-danger` | `#f87171` | Error, incompatible, must-replace |
| `--focus-ring` | `#22d3ee` | 3 px outline with a 2 px offset |

Chip tints are flat, not translucent, so the text on each one is measurable:
`--color-chip-neutral` `#202c47`, `--color-chip-verified` `#10291f`,
`--color-chip-warning` `#2c2114`, `--color-chip-unknown` `#1f2739`,
`--color-chip-danger` `#2e1a1d`, `--color-chip-primary` `#0e2a33`.

### Native UI

`html` declares `color-scheme: dark` and sets `accent-color` to the brand
primary, so native selects, scrollbars, checkbox ticks and autofill render
dark-legible and a checked control is not a light-theme widget.

## Measured contrast

`node scripts/check-contrast.mjs` resolves the effective theme across the
ordered token sources (vendored, then brand; the later declaration wins),
records the SHA-256 of each source in `docs/evidence/token-contrast.json`, and
measures 40 pairs. Any pair below its WCAG 2.2 AA minimum exits non-zero, and
`npm run check` binds that command. A measured pair must resolve to two flat
6-digit hex values: `color-mix()` and alpha are decoration, listed in the report
as decoration and never scored as a text pair.

Sources in the record: `vendor/tokens.css`
`7ca6b7956b3f…c19b` and `brand.css` `fc1fc93211cf…fbd`.

All 40 pairs pass. The lowest text pair is error text on the raised surface at
5.38:1 (4.5:1 required); the lowest UI pair is the control border on the raised
surface at 3.45:1 (3:1 required). Focus is at least 7.68:1 against every surface
and chip tint.

| Pair | Resolved | Ratio | Required | Result |
| --- | --- | --- | --- | --- |
| Body text on page | `#e6edf7` on `#0a1224` | 15.84:1 | 4.5:1 | Pass |
| Body text on surface | `#e6edf7` on `#141e33` | 14.11:1 | 4.5:1 | Pass |
| Body text on raised | `#e6edf7` on `#1b2740` | 12.63:1 | 4.5:1 | Pass |
| Secondary text on page | `#b9c5da` on `#0a1224` | 10.72:1 | 4.5:1 | Pass |
| Secondary text on surface | `#b9c5da` on `#141e33` | 9.55:1 | 4.5:1 | Pass |
| Secondary text on raised | `#b9c5da` on `#1b2740` | 8.55:1 | 4.5:1 | Pass |
| Secondary text on muted | `#b9c5da` on `#0e1729` | 10.28:1 | 4.5:1 | Pass |
| Primary button label | `#04121c` on `#06b6d4` | 7.80:1 | 4.5:1 | Pass |
| Primary button hover label | `#04121c` on `#22d3ee` | 10.48:1 | 4.5:1 | Pass |
| Link on surface | `#06b6d4` on `#141e33` | 6.85:1 | 4.5:1 | Pass |
| Link on raised | `#06b6d4` on `#1b2740` | 6.13:1 | 4.5:1 | Pass |
| Error text on surface | `#f87171` on `#141e33` | 6.01:1 | 4.5:1 | Pass |
| Error text on raised | `#f87171` on `#1b2740` | 5.38:1 | 4.5:1 | Pass |
| Error text on muted | `#f87171` on `#0e1729` | 6.47:1 | 4.5:1 | Pass |
| Success text on surface | `#34d399` on `#141e33` | 8.65:1 | 4.5:1 | Pass |
| Success text on raised | `#34d399` on `#1b2740` | 7.74:1 | 4.5:1 | Pass |
| Success text on muted | `#34d399` on `#0e1729` | 9.31:1 | 4.5:1 | Pass |
| Warning text on surface | `#fb923c` on `#141e33` | 7.35:1 | 4.5:1 | Pass |
| Warning text on raised | `#fb923c` on `#1b2740` | 6.58:1 | 4.5:1 | Pass |
| Warning text on muted | `#fb923c` on `#0e1729` | 7.91:1 | 4.5:1 | Pass |
| Warning text on page | `#fb923c` on `#0a1224` | 8.25:1 | 4.5:1 | Pass |
| Catalog badge on muted | `#06b6d4` on `#0e1729` | 7.37:1 | 4.5:1 | Pass |
| Chip text on verified tint | `#34d399` on `#10291f` | 8.04:1 | 4.5:1 | Pass |
| Chip text on warning tint | `#fb923c` on `#2c2114` | 6.95:1 | 4.5:1 | Pass |
| Chip text on unknown tint | `#fb923c` on `#1f2739` | 6.59:1 | 4.5:1 | Pass |
| Chip text on neutral tint | `#b9c5da` on `#202c47` | 7.97:1 | 4.5:1 | Pass |
| Chip text on danger tint | `#f87171` on `#2e1a1d` | 5.92:1 | 4.5:1 | Pass |
| Chip text on primary tint | `#06b6d4` on `#0e2a33` | 6.19:1 | 4.5:1 | Pass |
| Control border on page | `#6b7a99` on `#0a1224` | 4.33:1 | 3:1 | Pass |
| Control border on surface | `#6b7a99` on `#141e33` | 3.85:1 | 3:1 | Pass |
| Control border on raised | `#6b7a99` on `#1b2740` | 3.45:1 | 3:1 | Pass |
| Focus ring on page | `#22d3ee` on `#0a1224` | 10.33:1 | 3:1 | Pass |
| Focus ring on surface | `#22d3ee` on `#141e33` | 9.20:1 | 3:1 | Pass |
| Focus ring on raised | `#22d3ee` on `#1b2740` | 8.24:1 | 3:1 | Pass |
| Focus ring on verified tint | `#22d3ee` on `#10291f` | 8.55:1 | 3:1 | Pass |
| Focus ring on warning tint | `#22d3ee` on `#2c2114` | 8.71:1 | 3:1 | Pass |
| Focus ring on unknown tint | `#22d3ee` on `#1f2739` | 8.26:1 | 3:1 | Pass |
| Focus ring on neutral tint | `#22d3ee` on `#202c47` | 7.68:1 | 3:1 | Pass |
| Focus ring on danger tint | `#22d3ee` on `#2e1a1d` | 9.07:1 | 3:1 | Pass |
| Focus ring on primary tint | `#22d3ee` on `#0e2a33` | 8.31:1 | 3:1 | Pass |

## Dark and light

Dark is the brand mode and the only mode that ships. There is no theme toggle:
an unmeasured second theme is how contrast claims rot, and the reference gives
no light direction. The generated `[data-theme="dark"]` block in the vendored
kit is now redundant but is not removed, because that file is generated and
hash-pinned.

A light theme can return only with its own measured table, its own surface
ramp and its own chip tints in the same record; until then the kit's light
semantic set is a fallback for future work, not a shipped mode.

## Conformance scope

`ui.scope.exclude` names the two token files —
`apps/web/src/styles/vendor/tokens.css` and `apps/web/src/styles/brand.css`.
A token layer is where colour literals are declared, so scanning it for a
token-reachable literal is a category error. Every other UI file under the
declared scope (`apps/web/src`, 25 files at this revision) stays in scope, which
is the rule that matters: no colour literal outside the token layer. The
contrast checker enforces the other half — that the brand file declares custom
properties only.

## Still open

A full brand mark beyond the app icon (`apps/web/public/favicon.svg` and the
32×32 `.ico` fallback), product renaming and any marketing palette remain
undecided. A light theme remains unshipped and unmeasured.

# Gap closure program

The six design-program packages are complete. This document closes the residuals
recorded in `HANDOFF.md` after them. Each package is a separate OpenSpec change,
implemented on `main`, one active change at a time, with two commits each.

| # | Change | Single outcome |
| --- | --- | --- |
| 1 | `report-copy-plurals` | Every report, export and error sentence is grammatical |
| 2 | `performance-evidence` | A measured performance record and a deterministic byte budget |
| 3 | `visual-regression-baseline` | Pixel baselines that fail on an unintended visual change |
| 4 | `cross-browser-smoke` | Firefox/WebKit journeys and an assistive-technology review |

The manual screen-reader pass is **not** claimed by any package. A real pass needs
a human with a screen reader and a script; package 4 produces the accessibility
tree, the roles/names inventory and the keyboard traversal that such a pass
starts from, and records it as an assistive-technology review rather than a
screen-reader result.

## 1. `report-copy-plurals`

**Outcome:** the `(s)` pseudo-plural is gone from the service's user-visible copy.

**Requirements**

- R1 A shared Rust helper produces `0 units`, `1 unit`, `2 units` from a count;
  every user-facing sentence in `src/recommend/*`, `src/diagnosis/*`,
  `src/portability.rs` and `src/compatibility/mod.rs` uses it or an equivalent
  singular/plural branch.
- R2 No `(s)` remains anywhere under `src/` or `apps/web/src` (a test scans the
  sources and fails on one).
- R3 API text, Markdown export and JSON export carry the grammatical sentence;
  identifiers, error codes, schema and structure are unchanged.
- R4 Tests that asserted the old text are updated, not deleted; docs that quote
  it are updated.

**Oracle:** the guard test, the updated export round-trip tests, and a captured
recommendation and diagnosis showing `1 unit` / `2 units`, `1 step` / `2 steps`.

## 2. `performance-evidence`

**Outcome:** performance is a number, and the bundle has a budget that a gate
enforces.

**Requirements**

- R1 Lighthouse performance (mobile + desktop) is measured once against the
  production build for all six routes; the reports and the LCP, CLS, TBT, FCP,
  SI and transfer-size table are committed under `docs/evidence/`.
- R2 A deterministic byte budget for the built JS and CSS is enforced by
  `npm run check`; a bundle that grows past the ceiling fails the build with the
  file and the delta.
- R3 Any unambiguous finding (missing dimensions, render-blocking asset,
  avoidable duplicate payload) is fixed; anything left is recorded with the
  measured number and the reason.

**Oracle:** the budget check fails on a deliberately enlarged fixture, and the
recorded report matches the shipped asset hashes.

## 3. `visual-regression-baseline`

**Outcome:** an unintended visual change fails the browser suite.

**Requirements**

- R1 `toHaveScreenshot()` baselines exist for `/inventory`, `/catalog`,
  `/builds/check`, `/recommend`, `/diagnose` and the 404 at 1440×900 and 390×844,
  generated from the production build with the same seeded data the suite uses.
- R2 Dynamic values (snapshot id, timestamps, generated check/recommendation ids)
  are masked by locator, not by a blanket pixel tolerance; the threshold covers
  antialiasing only.
- R3 The visual spec runs inside `npm run test:e2e`; a changed pixel beyond the
  threshold fails with the diff artifact path.
- R4 A documented update command regenerates baselines deliberately
  (`npm run test:e2e -- --update-snapshots`), and the README/evidence says when
  that is legitimate.

**Oracle:** a one-pixel/one-token mutation fails the suite; the unmodified tree
passes twice in a row.

## 4. `cross-browser-smoke`

**Outcome:** the critical journeys are proved outside Chrome, and the assistive
technology review is recorded.

**Requirements**

- R1 Firefox runs a smoke subset (server-rendered routes, keyboard skip link,
  390 px overflow, 404, recommendation generate, inventory export) through the
  same fixtures; WebKit runs the same subset when its host dependencies install.
- R2 The Chrome full suite is unchanged; the smoke subset is added as separate
  projects and does not silently replace a failing Chrome journey.
- R3 The assistive-technology review records, per route, the accessibility tree
  (roles, names, landmarks, heading order), the focus order through the primary
  journey, and the live-region announcements, and states plainly that it is not a
  screen-reader pass.
- R4 A browser that cannot run on this host is recorded as unavailable with the
  exact dependency error, not omitted silently.

**Oracle:** the Firefox smoke passes; the review document names every route and
every skipped browser with its reason.

## Follow-up: the role and name gaps

`assistive-tech-gaps` (2026-11-01) fixes four of the five gaps the review found:
every form is a named landmark, the export/backup sentence is a note rather than
a live status, the "Catalog accepted" heading is addressable as
`#catalog-result-title`, and every route's `main` is named by its `h1`. The
`aria-live="off"` selection counter stays a human decision. It adds no visible
copy, layout or colour change, and the twelve pixel baselines pass unchanged.
The human screen-reader pass remains the only open gap.

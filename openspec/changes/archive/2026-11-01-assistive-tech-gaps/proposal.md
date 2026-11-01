# Proposal: Name the forms, the main regions and the result heading

## Why

`docs/evidence/assistive-technology-review.md` recorded five role/name gaps from
the running production build and deliberately did not fix them, because
`cross-browser-smoke` was tests and documentation only. Four of the five are
mechanical and unambiguous:

1. Every `<form>` on `/inventory`, `/catalog`, `/recommend` and `/builds/check`
   is unnamed, so landmark navigation cannot tell the import form from the search
   form.
2. "Exports and backups are created by this local service only." is static text
   inside a `role="status"` region, so a screen reader may announce it on load
   although nothing changed.
3. The focused "Catalog accepted" heading has no `id`, unlike every other result
   heading (`#result-title`, `#recommend-result-title`,
   `#diagnosis-result-title`, `#review-title`, `#snapshot-title`), so it cannot
   be addressed by a test or a skip target.
4. `main` has no accessible name, so the landmark is announced as an unnamed
   main region on all six routes.

The fifth gap — the `aria-live="off"` selection counter — is a judgement call
about chattiness, not a defect. It stays as it is and stays open for the human
screen-reader pass.

## What Changes

- **Every form is a named landmark.** The inventory import panel becomes a real
  `<form>` (it renders only inputs and a guarded button today) named by its
  "Import a CSV" heading. The catalog upload and search forms take their names
  from "Upload a catalog" and "Search". The recommendation budget form takes
  "3. Budget and constraints", and the assistant transport form takes
  "Assistant provider settings" because no visible heading names it. The
  compatibility-check parts form takes "2. Select parts". The four forms that
  appear only after an interaction — restore inspection, restore confirmation,
  preview-row correction and snapshot-record correction — are named the same
  way, with a heading `id` where a heading introduces them and a label that is
  not displayed where none does.
- **The static hint is a note, not a live status.** The export/backup sentence is
  rendered in a `role="note"` line with its text, styling and position
  unchanged, and genuine action results stay announced through a live region
  beside it.
- **The catalogue result heading is addressable.** It carries
  `id="catalog-result-title"` and keeps its existing focus move.
- **Each route's `main` is named.** Every page `h1` gets a stable `id` and the
  route's single `main` points at it with `aria-labelledby`, so the landmark is
  announced with the visible page name. No second landmark is introduced.
- **The selection counter is left alone.** `aria-live="off"` keeps its current
  behaviour and the evidence records it as a human decision with its reason.
- **Evidence.** `docs/evidence/assistive-tech-gaps.md` records the two
  `ariaSnapshot()` captures (`/inventory` and `/recommend`) that show the named
  `form` and `main`, and the review document marks the fixed gaps and keeps the
  screen-reader refusal and the human script.

## Impact

- Affected specs: `ui-foundation` gains five requirements (named form landmarks,
  a static hint that is not a live region, a named `main` per route, addressable
  result headings, and the recorded decision about the silent counter) plus the
  aria-snapshot evidence requirement. No existing requirement is modified or
  removed, and the collapsed-assistant requirement keeps its transport-form
  ownership unchanged.
- Affected code: `apps/web/src/components/ImportForm.tsx`, `CatalogPage.tsx`,
  `CheckPage.tsx`, `RecommendPage.tsx`, `DiagnosePage.tsx`,
  `InventoryPage.tsx`, `AssistantPanel.tsx`, `DataPortability.tsx`,
  `PreviewReview.tsx`, `SnapshotPanel.tsx`; component tests; one new
  `e2e/landmarks.spec.ts`; `docs/evidence/assistive-technology-review.md` and
  the new `docs/evidence/assistive-tech-gaps.md`.
- Affected behaviour: landmark and accessible names only. No visible copy,
  layout, colour, spacing, route, API, schema, engine or dependency change, and
  the twelve committed visual baselines are not regenerated.
- Non-goals: no screen-reader pass and no claim of one; no change to the silent
  selection counter; no change to any live-region text that reports a real
  action result.

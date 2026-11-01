# Design: Deterministic pixel baselines inside the one browser suite

## Context

`e2e/` holds 28 Playwright tests in 12 spec files. `playwright.config.ts` runs
one worker, no retries, `channel: 'chrome'`, `locale: en-US`, `timezoneId: UTC`,
and starts `target/debug/rigcraft` on a free loopback port with a fresh scratch
data directory. Spec files run in alphabetical order against one service and one
SQLite file. `e2e/fixtures.ts` already owns `seedSample` (download the shipped
sample, preview it, commit every row) and `chooseSnapshot`.

A Playwright screenshot is only useful if it is committed and reproducible. The
hard part is not the assertion; it is removing every source of legitimate change
so that a failure means an unintended one.

## Decision 1 — one test per route per viewport, seeded immediately before capture

The visual spec cannot rely on what ran before it. Every test therefore:

1. seeds the state it needs through the documented API;
2. opens the route;
3. drives the real interaction when the route has a result (Check, Generate,
   Match + Diagnose);
4. scrolls to a fixed evidence anchor;
5. captures the viewport.

A fresh import is always revision 1 (`snapshot_from_preview` sets
`revision: 1`), so the revision number in the console bar, the report metadata
and the `<select>` label is stable even as the database accumulates snapshots.
The side effect is that `/inventory`'s history list grows across runs; it is
below the captured viewport at both sizes, so the baseline stays stable.

`/catalog` has no delete-everything endpoint, but it has
`GET /api/catalog/datasets` and `DELETE /api/catalog/datasets/{id}?confirm=remove`.
Removing a dataset cascades only to its own parts and no inventory table refers
to it (the schema and the refusal copy say so), so the registry can be reset
safely and the synthetic fixture uploaded again. The registry is then exactly
one dataset with three parts on the first run and on the second.

## Decision 2 — viewport captures, not full-page captures

A full-page baseline of `/inventory` grows with every record and with every
history entry, and a full-page baseline of `/recommend` is thousands of pixels
tall. Both would be large, slow and brittle. Each baseline is 1440×900 or
390×844. To keep the interesting part in frame, each route has one scroll anchor:

| Route | Anchor |
| --- | --- |
| `/inventory` | `.console-bar` (the open snapshot's state) |
| `/catalog` | the registry panel |
| `/builds/check` | `#result-title` after a real check |
| `/recommend` | `.workbench` after a real generation |
| `/diagnose` | `#diagnosis-result-title` after a real diagnosis |
| `/nope` | scroll position 0 (a plain-text 404 document) |

The scroll is computed from the anchor's document offset, not from
`scrollIntoView`, so the same offset is used at both widths and on both runs.

## Decision 3 — mask block containers, never an inline text run

Playwright draws each mask over the element's bounding box. A mask over an inline
`<time>` would inherit that text's exact width, and two timestamps can round to
different widths. Each mask therefore targets a block-level container whose width
is set by layout:

- the revision cell of the inventory console stats;
- the inventory console meta line (committed time);
- the `Snapshot ID`, `Committed` and `Source file` cells of the record meta grid
  (the id, the timestamp and the sha256 fragment);
- the registry dataset cell (the import time);
- the check and diagnosis `Checked …` meta line inside the verdict line;
- every `<time>` in a result — catalogue provenance observation times are recorded
  at import time, not in the fixture;
- the recommendation candidate's `Check report …` meta line;
- the comparison table's `Check report` row;
- the `<select>` snapshot choice blocks on check, recommend and diagnose (the
  selected option label embeds a timestamp and a short id);
- the native file control on `/catalog`, whose label is supplied by the browser
  rather than the application and therefore follows the browser UI language.

Everything else — headings, controls, counts, badges, tables, the rule matrix,
the verdict rail — stays in the baseline.

## Decision 4 — a bounded tolerance, not a substitute for masking

`maxDiffPixelRatio: 0.01` and `threshold: 0.05` are passed to every capture, with
`animations: 'disabled'` and `caret: 'hide'`. The 1% bound absorbs a stray
antialiased glyph edge; it does not absorb a changed surface colour or a moved
control. The threshold is deliberately below Playwright's default `0.2`: with
`0.2`, a subtler surface change (`#0a1224` → `#2a1244`, a delta of about 166
against a maximum of 1409) is treated as the same colour and the suite would pass.
At `0.05` that change is 27,014–299,130 differing pixels depending on the route
and the suite fails. Identical renders on the same host are bit-identical, so the
non-zero threshold only guards a glyph edge.

## Decision 5 — baselines are platform-specific and that is stated

The file names Playwright generates carry the project and platform
(`…-chromium-linux.png`). The committed set was generated on Linux x86_64 with
the real Chrome channel and the host's system font stack. A different OS, GPU
or fonts needs its own baseline set; the evidence document states this rather
than pretending the images travel.

## Decision 6 — one deliberate update command

`npm run test:e2e -- --update-snapshots` regenerates the baselines through
`e2e/run.mjs`, which forwards the arguments to the Playwright CLI. Updating is
legitimate only when the visual change is intended, a human reviewed the new
images, and the new images ship in the same change. The README and the evidence
document say this; the suite never updates by itself.

## Verification shape

`e2e/visual.spec.ts` passes twice in a row against the same database and against
a fresh one, the full suite still passes with the visual spec included, and a
one-token mutation of a brand value fails with a reported diff ratio and a
written diff artifact. The restored tree passes again and the baselines are the
original bytes.

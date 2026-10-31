# Proposal: Add a checked-in browser regression suite

## Why

Seven browser-evidence documents under `docs/evidence/` record the same limitation:
the journeys, keyboard paths and 390 px checks were driven through Chrome DevTools
MCP once, and no stored browser test re-runs them. Six of those documents close with
"Not done: a checked-in Playwright suite". The consequence is not theoretical. Two of
the eight defects in the correctness audit were only visible in a real browser:

- `/diagnose` labelled parts that no rule could evaluate under "Fine as is", which
  reads as a green check mark for an unverified part.
- `/diagnose` numbered its sections 1, 2, 4, 5 because the review section vanished
  before a list was matched.

Both are fixed and covered by Vitest at the component level, but the page is rendered
by the Rust service from a built bundle; nothing before this change would fail if the
rendered page regressed. The UI is now stable — the five implementation packages are
archived and the sample is one verified AM5 build — so the journeys can be asserted
against the seeded data instead of synthetic fixtures.

## What Changes

- Add `playwright.config.ts` and an `e2e/` suite that drives the production build with
  real Chrome (`channel: 'chrome'`, no browser download), one project, one worker, no
  retries, JSON and list reporters, and traces only on retry.
- The suite owns its runtime: `npm run test:e2e` picks a free loopback port, starts
  `./target/debug/rigcraft` through Playwright's `webServer` with a fresh
  `RIGCRAFT_DATA_DIR` under the OS temp directory, waits for `/api/health`, and removes
  the scratch directory when the run ends. It never touches port 8080 or an existing
  data directory, and it needs no network.
- Assert the seeded sample end to end in a browser: download it from
  `/api/imports/sample`, preview and commit it through `/inventory`, read revision 1
  and its ten records, and prove the provenance split — `model` and `specs` carry
  `catalog:manufacturer@2026-10-31` with the manufacturer URL while `sku`, `quantity`
  and `price` stay operator-entered.
- Assert the compatibility verdict the seed actually produces: 12 compatible,
  0 conflicts, 2 unknown, with `cpu-bios` and `psu-capacity` named and their
  follow-ups stated — not a green pass.
- Assert recommendation on the seed: a budget that fits returns a ranked option under
  budget carrying an honest `Unknown` label, and a budget below the cheapest complete
  build returns the named binding constraint and its minimum.
- Assert diagnosis: one exact, one ambiguous and one unmatched pasted line are not
  coerced; the diagnosis is blocked until every line is resolved; a part-list build
  with no evaluable rule shows "Not verified — no rule evaluated" and "Fine as is (0)"
  and never a check mark for the unverified part.
- Add regression guards for the audited fixes: `/assets/nope.js`, `/robots.txt` and
  `/llms.txt` return 404 rather than the SPA shell; a malformed JSON POST returns
  `application/json` with `invalid_json`; `/diagnose` renders sections 1–5 before any
  list is matched; a timestamp renders as ISO 8601 UTC under a `de-DE` locale.
- Add one keyboard-only journey and a 390 px assertion that
  `scrollWidth === clientWidth` on every page the suite touches.
- Wire `npm run test:e2e` into `./verify-local.sh`, and replace the stale "Not done"
  lines in the seven browser-evidence documents with what the suite now covers.

## Package Boundary and Split Assessment

Single outcome: the documented browser behaviour is executable. The suite depends on
the shipped seed, the four Web pages and the current API contracts; it adds no product
behaviour, no rule, no endpoint and no stored fixture data of its own. Splitting the
config, the journeys and the wiring would leave a suite that no gate runs — which is
the defect this change exists to remove, because a verifier that no gate runs does not
gate. The four journeys share one server, one port policy and one seed, so they ship as
one package.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence | Compatibility and ownership | Decision |
| --- | --- | --- | --- |
| Playwright (`@playwright/test` 1.64.1) | `npm view @playwright/test`, installed from the registry | The only maintained cross-browser driver that records traces and provides a `webServer` lifecycle; Chrome 154 is present at `/usr/bin/google-chrome`, so `channel: 'chrome'` needs no browser download. | adopt as a root devDependency |
| Vitest + Testing Library | `apps/web/package.json`, `apps/web/src/**/*.test.tsx` | Component and state coverage in jsdom; it cannot render the Rust-served bundle, observe a real download, or measure `scrollWidth`. | keep, unchanged |
| Chrome DevTools MCP evidence | `docs/evidence/*-browser.md` | One-off session against a running binary; no stored, re-runnable assertion. | keep the documents, replace the limitation |
| Forge Gate runtime | `.forge/gate/rigcraft/evidence.json`, `.ai-gate/gate.yaml` | Runs `cargo test`, build, security and UI conformance; does not drive a browser. | unchanged; the suite runs from `verify-local.sh` |

## User Experience and Interface Impact

No interface changes. The suite exercises the existing surfaces: `/inventory` import,
review, commit and provenance disclosure; `/builds/check` selection and verdict;
`/recommend` request, ranked options, no-solution explanation and export-adjacent
state; `/diagnose` sections 1–5, match review, blocked review and report groups. It
also asserts the keyboard path (skip link first, then a form submitted from the
keyboard) and the absence of horizontal overflow at 390 px. Nothing is mocked and no
test-only route, flag or fixture is added to the product.

## BFS Impact Map

New files: `playwright.config.ts`, `e2e/start-server.mjs`, `e2e/run.mjs`,
`e2e/fixtures.ts`, five spec files. Changed: root `package.json` (script + devDependency),
`package-lock.json` (from `npm install --save-dev`), `verify-local.sh`,
`.gitignore` (test artifacts), `AGENTS.md` Project facts, README Runtime, the seven
browser-evidence documents, and `HANDOFF.md`. No Rust source, SQLite schema, API
contract, token, deployment flag or sibling repository changes.

## Capabilities

- `browser-regression` (added): the suite's execution contract, the journeys it must
  assert against the seeded sample, the regression guards, and the gate wiring.

## Non-goals

Visual-diff baselines, cross-browser projects, a CI-only configuration, mocked API
responses, retries that hide a flake, a live network or AI provider call, and any
change to product behaviour or test-only endpoints.

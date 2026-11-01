# Design: Firefox and WebKit smoke, and an assistive-technology review

## Context

`playwright.config.ts` runs one headless project named `chromium` on
`channel: 'chrome'` with one worker, no retries, `locale: en-US`,
`timezoneId: UTC`, and a `webServer` that starts `target/debug/rigcraft` on a
free loopback port with a scratch data directory. `e2e/run.mjs` is the single
entry point and owns the port and cleanup. `e2e/fixtures.ts` owns `seedSample`,
`chooseSnapshot` and `expectNoHorizontalOverflow`. Twelve `toHaveScreenshot()`
baselines are committed for Chrome only, under
`e2e/visual.spec.ts-snapshots/*-chromium-linux.png`.

## Decision 1 — one smoke spec, three projects

`e2e/smoke.spec.ts` is a normal spec with no special casing. The chromium
project runs it with every other journey because it matches the default pattern.
The `firefox` and `webkit` projects set
`testMatch: /smoke\.spec\.ts/`, so each engine runs exactly six tests. That keeps
the projects honest in both directions:

- Firefox/WebKit cannot silently stand in for a Chrome journey, because they
  never run one.
- The Chrome suite cannot lose a journey to a "browser-specific" project,
  because the smoke file is additive.

`channel: 'chrome'` was a global `use` value. It is now declared only on the
chromium project, because Firefox and WebKit have no Chrome channel and would
otherwise be told to launch one. Headless, locale, timezone, downloads and the
trace policy stay global.

## Decision 2 — the smoke subset asserts what a second engine can differ on

The six tests are deliberately the ones an engine, not the application, can
break:

| Test | What a second engine could get wrong |
| --- | --- |
| shell + title per route | a bundle error, a route that falls back, a title set too late |
| skip link first, then `main` | focus order, `tabindex="-1"` focus after a fragment jump |
| 390 px overflow per route | a control, table or long token that widens the document |
| typo path 404 | a client-side fallback intercepting the navigation |
| seeded recommendation | the budget form, the fetch, the rendered gaming rule |
| inventory CSV download | the `download` event and the suggested filename |

It reuses `seedSample`, `chooseSnapshot` and `expectNoHorizontalOverflow`; no
fixture is reimplemented. It asserts no pixel and carries no baseline, because
the committed set belongs to Chrome and a cross-engine baseline needs its own
committed set and its own review.

## Decision 3 — WebKit's host dependencies, installed without sudo

`npx playwright install firefox webkit` downloaded Firefox 157.0 and WebKit 27.2.
Firefox launched immediately. WebKit's first launch failed through Playwright's
host validation:

```
Host system is missing dependencies to run browsers.
    sudo apt-get install libevent-2.1-7t64 libavif16 libmanette-0.2-0
```

The checkout that then failed was:

```
/home/paul/.cache/ms-playwright/webkit-2370/minibrowser-wpe/bin/MiniBrowser: error while loading shared libraries: libevent-2.1.so.7: cannot open shared object file: No such file or directory
```

Root is not available on this host, so the Debian packages were fetched with
`apt-get download` (no root needed), unpacked with `dpkg-deb -x` into a user
directory, and copied into the bundle's `minibrowser-wpe/sys/lib` — the directory
`minibrowser-wpe/MiniBrowser` puts on `LD_LIBRARY_PATH` itself. `ldd` then
reported no missing library, Playwright's own validation passed without an
opt-out, and WebKit 27.2 launched. The workaround is recorded with its commands
because a fresh browser cache needs it repeated; no `PLAYWRIGHT_SKIP…`
environment variable is set and no check is switched off.

The packages fetched were `libevent-2.1-7t64`, `libavif16`, `libmanette-0.2-0`,
`libbacktrace0`, `libgav1-2` and `libhidapi-hidraw0`.

CI runs `ubuntu-latest`, where the Playwright browsers are not preinstalled. The
workflow therefore installs Firefox and WebKit with `--with-deps` (the runner has
sudo) before `verify-local.sh`. That is runner provisioning, not an opt-out.

## Decision 4 — the assistive-technology review is produceable, not a vibe

The review is generated from the running production build with Playwright 1.64.1
on the Chrome channel: `locator.ariaSnapshot()` on `body` per route, the heading
and landmark lists read from the DOM, every `[aria-live]`, `[role=status]` and
`[role=alert]` element with its text, the Tab sequence from the document start,
and the programmatically focused element after each action. The seeded data is
the shipped sample.

The document is explicit about its ceiling: an accessibility tree is what an AT
*should* receive; only a human with a real screen reader and a script can say
what it *does* receive. The hand-over script names the browser and screen-reader
pairing, the route order, the exact keys, and what should be announced.

The document records the gaps it found rather than fixing them, because this
change is tests and documentation only:

- The three `<form>` elements (`/inventory`, `/catalog`, `/recommend`,
  `/diagnose`) have no accessible name, so they are not named landmarks.
- `/inventory` and `/` mark a static explanatory sentence
  (`Exports and backups are created by this local service only.`) as
  `role="status"`, so an AT may announce it on load even though nothing changed.
- The focused "Catalog accepted" heading carries no id, unlike the result
  headings on check, recommendation and diagnosis.

## Decision 5 — two owners, no contradiction

`browser-regression` already owns the suite, its runtime and its wiring; the
Firefox/WebKit projects are that suite. `ui-foundation` already owns the
interface evidence (contrast, focus, skip link, baselines); the
assistive-technology review is interface evidence. Neither requirement
contradicts an existing one, and nothing existing is modified.

## Verification shape

`npm run test:e2e` runs 46 Chrome tests (40 existing plus the 6 smoke) and 6 each
in Firefox and WebKit, all green on one service and one scratch database. The
Chrome visual baselines still match. `./verify-local.sh` and the Forge full Gate
pass, and the review document names all six routes and the exact human script.

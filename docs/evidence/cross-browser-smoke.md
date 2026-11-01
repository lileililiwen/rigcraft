# Cross-browser smoke and assistive-technology evidence

The browser suite now proves its critical journeys outside Chrome, and the
assistive-technology review is recorded in
[`assistive-technology-review.md`](assistive-technology-review.md). Neither is a
screen-reader pass: the review document says so at the top and hands over the
human script.

## Browser matrix

| Project | Specs | Tests | Result |
| --- | --- | --- | --- |
| `chromium` (real Chrome 154.0.8037.97) | every spec, including `visual.spec.ts` | 46 | 46 passed (25.1 s) |
| `firefox` (Playwright Firefox 157.0) | `smoke.spec.ts` only | 6 | 6 passed (5.9 s) |
| `webkit` (Playwright WebKit 27.2) | `smoke.spec.ts` only | 6 | 6 passed (5.3 s) |
| all three | — | 58 | 58 passed (36.2 s) |

The Chrome count is the previous 40 journeys plus the 6 smoke tests, so no Chrome
journey was replaced. The 6 Firefox and 6 WebKit tests are additional. Every run
above used the single entry point, `npm run test:e2e`, on a free loopback port
with a fresh scratch data directory; nothing was skipped and no retry ran.

```
$ npx playwright --version
Version 1.64.1

$ npm run test:e2e -- --project=firefox
Running 6 tests using 1 worker
  ✓  1 [firefox] › e2e/smoke.spec.ts:35:5 › every client route serves the shell with its own title (1.7s)
  ✓  2 [firefox] › e2e/smoke.spec.ts:49:5 › the skip link is the first Tab stop and moves focus to main (475ms)
  ✓  3 [firefox] › e2e/smoke.spec.ts:60:5 › no client route overflows horizontally at 390 px (945ms)
  ✓  4 [firefox] › e2e/smoke.spec.ts:70:5 › a typo path is the real 404 document, not the shell (291ms)
  ✓  5 [firefox] › e2e/smoke.spec.ts:78:5 › a recommendation generates from the seeded sample (671ms)
  ✓  6 [firefox] › e2e/smoke.spec.ts:96:5 › the inventory export downloads with its revision-bearing filename (569ms)
  6 passed (5.9s)

$ npm run test:e2e -- --project=webkit
Running 6 tests using 1 worker
  ✓  1 [webkit] › e2e/smoke.spec.ts:35:5 › every client route serves the shell with its own title (1.1s)
  ✓  2 [webkit] › e2e/smoke.spec.ts:49:5 › the skip link is the first Tab stop and moves focus to main (699ms)
  ✓  3 [webkit] › e2e/smoke.spec.ts:60:5 › no client route overflows horizontally at 390 px (916ms)
  ✓  4 [webkit] › e2e/smoke.spec.ts:70:5 › a typo path is the real 404 document, not the shell (345ms)
  ✓  5 [webkit] › e2e/smoke.spec.ts:78:5 › a recommendation generates from the seeded sample (911ms)
  ✓  6 [webkit] › e2e/smoke.spec.ts:96:5 › the inventory export downloads with its revision-bearing filename (743ms)
  6 passed (5.3s)

$ npm run test:e2e -- --project=chromium
  46 passed (25.1s)

$ npm run test:e2e
  58 passed (36.2s)
```

## Smoke coverage

`e2e/smoke.spec.ts` uses `seedSample`, `chooseSnapshot` and
`expectNoHorizontalOverflow` from `e2e/fixtures.ts`; no seeding logic is
duplicated and no application code is touched.

1. **Every client route serves the shell with its own title.** `/`,
   `/inventory`, `/catalog`, `/builds/check`, `/recommend`, `/diagnose` each
   return 200, one `#root`, their own `document.title` (RigCraft inventory /
   catalog / compatibility check / build recommendation / build diagnosis), and
   their `h1`.
2. **The skip link is the first Tab stop and moves focus to main.** One `Tab`
   focuses "Skip to inventory"; `Enter` focuses `main#main`.
3. **No client route overflows horizontally at 390 px.** `scrollWidth ===
   clientWidth` on all six routes.
4. **A typo path is the real 404 document.** `/recomend` is 404 `text/plain`
   with "404 Not found." and no `#root`.
5. **A recommendation generates from the seeded sample.** The shipped sample is
   committed through the API, the test's own snapshot selected, budget `1800`
   entered, Generate pressed, and the result carries "Recommendation complete",
   "Best overall" and the gaming rule heading.
6. **The inventory export downloads with its revision-bearing filename.**
   `rigcraft-inventory-snapshot-r1-*.csv`.

## Installing the browsers

```
$ npx playwright install firefox webkit
Downloading Firefox 157.0 (playwright firefox v1521)…
Firefox 157.0 (playwright firefox v1521) downloaded to /home/paul/.cache/ms-playwright/firefox-1521
Downloading WebKit 27.2 (playwright webkit v2370)…
WebKit 27.2 (playwright webkit v2370) downloaded to /home/paul/.cache/ms-playwright/webkit-2370
Playwright Host validation warning:
╔══════════════════════════════════════════════════════╗
║ Host system is missing dependencies to run browsers. ║
║ Please install them with the following command:      ║
║                                                      ║
║     sudo npx playwright install-deps                 ║
║                                                      ║
║ Alternatively, use apt:                              ║
║     sudo apt-get install libevent-2.1-7t64\          ║
║         libavif16\                                   ║
║         libmanette-0.2-0                             ║
║                                                      ║
║ <3 Playwright Team                                   ║
╚══════════════════════════════════════════════════════╝
```

Firefox launched on the first probe (`firefox: launched, title=ok, version=157.0`).
WebKit did not. Its exact first launch error was:

```
browserType.launch:
╔══════════════════════════════════════════════════════╗
║ Host system is missing dependencies to run browsers. ║
║ Please install them with the following command:      ║
║     sudo npx playwright install-deps                 ║
║ Alternatively, use apt:                              ║
║     sudo apt-get install libevent-2.1-7t64\          ║
║         libavif16\                                   ║
║         libmanette-0.2-0                             ║
╚══════════════════════════════════════════════════════╝
```

Bypassing only the validation (with `LD_LIBRARY_PATH` pointing at the extracted
libraries) exposed the loader error underneath:

```
/home/paul/.cache/ms-playwright/webkit-2370/minibrowser-wpe/bin/MiniBrowser: error while loading shared libraries: libevent-2.1.so.7: cannot open shared object file: No such file or directory
```

This host has no root, so the packages were downloaded and unpacked without sudo
and copied into the WebKit bundle's own library directory — the one its
`MiniBrowser` wrapper already puts on `LD_LIBRARY_PATH`:

```
$ mkdir -p /tmp/opencode/webkit-deps && cd /tmp/opencode/webkit-deps
$ apt-get download libevent-2.1-7t64 libavif16 libmanette-0.2-0 libbacktrace0 libgav1-2 libhidapi-hidraw0
$ mkdir -p root && for d in *.deb; do dpkg-deb -x "$d" root; done
$ cp -a root/usr/lib/x86_64-linux-gnu/. \
    /home/paul/.cache/ms-playwright/webkit-2370/minibrowser-wpe/sys/lib/
```

After that, `ldd` reports no missing library for
`minibrowser-wpe/bin/MiniBrowser`, Playwright's own host validation passes
without `PLAYWRIGHT_SKIP_VALIDATE_HOST_REQUIREMENTS`, and the probe printed:

```
firefox: launched, title=ok, version=157.0
webkit: launched, title=ok, version=27.2
```

**Verdict: WebKit 27.2 ran, 6/6 smoke tests passed.** It is not marked
unavailable. The provisioning is per browser cache: a fresh
`npx playwright install webkit` restores the stock bundle and the copy must be
repeated on a host whose distribution lacks these six packages. The committed
`.github/workflows/forge-gate.yml` installs both engines on the runner with
`npx playwright install --with-deps firefox webkit` (the runner has sudo), so CI
does not depend on this local workaround.

## Playwright configuration

`channel: 'chrome'` moved from the global `use` block to the `chromium` project,
because Firefox and WebKit have no Chrome channel. The two new projects are:

```ts
{ name: "firefox", testMatch: /smoke\.spec\.ts/, use: { ...devices["Desktop Firefox"] } },
{ name: "webkit",  testMatch: /smoke\.spec\.ts/, use: { ...devices["Desktop Safari"] } },
```

`chromium` keeps every spec. The smoke spec asserts no pixels, so the committed
snapshot directory still holds Chrome images only and no Firefox or WebKit
baseline is claimed.

## Not done

- No cross-engine pixel baselines and no full Firefox/WebKit journey parity; the
  six smoke journeys are the declared contract.
- No mobile browser engine (Android Chrome, iOS Safari) and no screen-reader
  output: the assistive-technology review is the starting record, not the pass.
- The WebKit host-library copy lives in the Playwright cache, outside the
  repository. It is recorded here because it is required to reproduce the run on
  this host, and CI installs the packages properly instead.

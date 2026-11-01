# Performance evidence

Date: 2026-11-01. Production build (`cargo build` plus `npm run build`):

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| `apps/web/dist/assets/index-DO5RYnOJ.js` | 307,491 | `9275341ae754b63ac672a8f89339c81d00de93faff3a560c44105fd833f12dbc` |
| `apps/web/dist/assets/index-B-c8V-0n.css` | 27,698 | `506961cd631879431b0444c9cb4d1e6a7b57f9ef8dc4d41ddc6ecc3a2c650e98` |
| `apps/web/dist/index.html` | 664 | — |

The service was `target/debug/rigcraft` on `127.0.0.1:33250` with the scratch data
directory `/tmp/opencode/perf-evidence/data`. It was seeded through the
documented API, not by touching SQLite:

- `GET /api/imports/sample` → `POST /api/imports` (multipart, `USD`) →
  `POST /api/imports/{previewId}/commit`: preview `{"total":10,"valid":10,"errors":0,"duplicateRows":0}`,
  snapshot `6165c7cb-c482-4d24-81b6-35e70960cb6f` revision 1, 10 records.
- `POST /api/catalog/datasets` with `{ "acknowledged": true, "catalog": … }`
  where the document is the `synthetic-fixture` used by `e2e/catalog.spec.ts`
  (three parts, `CC0-1.0`, `fixture-1`): HTTP 200, `partCount: 3`, active.

Lighthouse 13.6.0 ran headless through system Chrome 154.0.0.0 on Linux. The
JSON reports are committed; nothing below is retyped from a screenshot or
estimated by hand.

## Exact command

Mobile (Lighthouse's default form factor: mobile screen emulation with Lantern
simulated throttling at 4× CPU and 1.6 Mbps):

```sh
npx --yes lighthouse@13.6.0 http://127.0.0.1:33250<route> \
  --only-categories=performance \
  --chrome-flags="--headless=new --no-sandbox --disable-gpu" \
  --output=json --output-path=docs/evidence/perf-lighthouse/<route>-mobile/report.json
```

Desktop adds `--preset=desktop` (desktop screen emulation, no throttling) and
writes to `docs/evidence/perf-lighthouse/<route>/report.json`:

```sh
npx --yes lighthouse@13.6.0 http://127.0.0.1:33250<route> \
  --preset=desktop \
  --only-categories=performance \
  --chrome-flags="--headless=new --no-sandbox --disable-gpu" \
  --output=json --output-path=docs/evidence/perf-lighthouse/<route>/report.json
```

Route directory names: `home` for `/`, `inventory`, `catalog`, `build-check`,
`recommend`, `diagnose`; the mobile report of each is the same name with
`-mobile`. Twelve reports exist, one per route and device.

## Measured results

Scores are the Lighthouse performance category × 100. Times are the report's
numeric values; transferred bytes are `total-byte-weight` (the sum of
`transferSize`, compressed, including response headers).

| Route | Device | Score | FCP | LCP | TBT | CLS | SI | Transferred |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `/` | desktop | 100 | 203 ms | 203 ms | 0 ms | 0.005 | 203 ms | 99,867 B |
| `/` | mobile | 100 | 762 ms | 762 ms | 0 ms | 0.000 | 762 ms | 99,867 B |
| `/inventory` | desktop | 100 | 202 ms | 202 ms | 0 ms | 0.005 | 202 ms | 99,829 B |
| `/inventory` | mobile | 100 | 752 ms | 752 ms | 0 ms | 0.000 | 752 ms | 99,829 B |
| `/catalog` | desktop | 100 | 212 ms | 212 ms | 0 ms | 0.025 | 212 ms | 96,749 B |
| `/catalog` | mobile | 100 | 752 ms | 1,504 ms | 0 ms | 0.000 | 752 ms | 96,755 B |
| `/builds/check` | desktop | 100 | 204 ms | 366 ms | 0 ms | 0.019 | 230 ms | 99,834 B |
| `/builds/check` | mobile | 100 | 752 ms | 752 ms | 0 ms | 0.001 | 793 ms | 99,829 B |
| `/recommend` | desktop | 100 | 207 ms | 207 ms | 0 ms | 0.000 | 215 ms | 99,829 B |
| `/recommend` | mobile | 100 | 752 ms | 752 ms | 0 ms | 0.000 | 752 ms | 99,829 B |
| `/diagnose` | desktop | 100 | 205 ms | 205 ms | 0 ms | 0.019 | 205 ms | 99,829 B |
| `/diagnose` | mobile | 100 | 752 ms | 1,502 ms | 0 ms | 0.013 | 752 ms | 99,829 B |

One cold navigation per route and device, cache cleared by Lighthouse. Every
route transferred the same two hashed assets; the `/` report's network log:

| Request | Transfer bytes |
| --- | ---: |
| `/` document (664-byte shell, brotli) | 943 |
| `/assets/index-DO5RYnOJ.js` (brotli) | 88,176 |
| `/assets/index-B-c8V-0n.css` (brotli) | 5,997 |
| `/favicon.svg` | 877 |
| `/api/inventory/snapshots` | 770 |
| `/api/inventory/snapshots/6165c7cb…cb6f` | 3,104 |

`/catalog` transfers 3,118 bytes fewer because its registry request replaces the
snapshot requests; `/inventory` and `/` are the same page shell plus one snapshot
fetch. The 664-byte shell is not the whole document cost: the bundle is 88,176
bytes compressed and is the largest single transfer.

What is measured is each route's arrival state with seeded data: `/inventory`
auto-opens the newest snapshot, `/catalog` shows the registry, and
`/builds/check` and `/diagnose` show their selection forms. Producing a check,
recommendation or diagnosis needs a click and a request that a navigation
Lighthouse run does not perform, so no generated-result score is claimed here.

## Findings: nothing was changed

All twelve runs score 100. The audits that are not at 1.00 are listed with their
measured numbers; none is an unambiguous, safe fix, so the build was not
edited and the hashes above are the measured ones.

| Audit | Runs | Number | Why it is not changed |
| --- | ---: | --- | --- |
| `unused-javascript` | 12 | 51–58 KiB unused of the 87,458 bytes it transfers | The app is one React bundle. Removing it means route-level code splitting, which changes the module graph and adds chunks and round trips; it cannot raise a score that is already 100 and is not a duplicate payload. |
| `render-blocking-insight` | 12 | one stylesheet, 5,997 bytes, 150 ms estimated savings in some runs | It is the only stylesheet. Inlining it would move CSS into the `no-cache` shell and defeat the immutable asset cache; loading it asynchronously risks unstyled content and layout shift. The CSS arrives at 13 ms into the navigation. |
| `network-dependency-tree-insight` | 12 | document → bundle → snapshot API, longest chain 115 ms | This is the client-rendered shape of the app. Removing the chain means server rendering, which is a product change, not a performance fix. |
| `max-potential-fid` | 1 | 120 ms | A single-run estimate on one route; no corresponding TBT (0 ms) and no long task in any other run. Recorded as lab variance. |

Audits that would name a real defect are clean: `unsized-images` 1.00 (there are
no content images beyond the icon, and the `.ico` link declares
`sizes="32x32"`), `uses-responsive-images` and `modern-image-formats` absent (no
images), `font-display` absent (no webfont), `third-party-summary` absent (no
third-party origin), `duplicated-javascript` and `legacy-javascript` absent, and
`server-response-time` 0 ms on a loopback service. No service worker, CDN,
preload hint or dependency was added.

## Byte budget

`scripts/check-bundle-size.mjs` declares the ceilings and compares every built
asset against its type:

| Type | Ceiling | Current | Headroom |
| --- | ---: | ---: | ---: |
| JavaScript (`*.js`) | 330,000 B | 307,491 B | 22,509 B (7.3%) |
| CSS (`*.css`) | 32,000 B | 27,698 B | 4,302 B (15.5%) |

The check is wired into `npm run build` (after Vite writes the assets) and into
`npm run check` (which runs its own fixture test first). `./verify-local.sh` now
runs `npm run build` before `npm run check`, so the check always reads the fresh
build; no stage was removed.

Real run against the current build:

```text
PASS apps/web/dist/assets/index-B-c8V-0n.css 27698 bytes (CSS ceiling 32000, 4302 bytes headroom)
PASS apps/web/dist/assets/index-DO5RYnOJ.js 307491 bytes (JS ceiling 330000, 22509 bytes headroom)
All 2 assets within budget.
```

Canary: a copy of the build with one byte appended to the JavaScript asset,
measured with the same script (`--assets` pointed at the temporary copy, no
repository write):

```text
FAIL /tmp/rigcraft-budget-canary-7y3b8jbf/index-DO5RYnOJ.js 330001 bytes exceeds the JS ceiling 330000 by 1 byte
1 asset over budget.
exit 1
```

An absent or empty `apps/web/dist/assets` also exits 1 and says to run
`npm run build`, so a fresh checkout cannot pass by having nothing to measure.
`scripts/check-bundle-size.test.mjs` proves the same three cases with
`fs.mkdtemp` fixtures.

## Reading these numbers honestly

These are **localhost lab measurements**, not field data. The service and
Lighthouse share one machine, there is no network path, no real user traffic, no
cold cache beyond the one Lighthouse clears, and no population to average over.
The 100 scores say that the shipped build has no audit-level performance defect
under Lighthouse's model for these routes at this moment; they are not a Core
Web Vitals field result and do not predict a slow phone on a slow connection.
Mobile runs use Lighthouse's simulated throttling, which is a model, not a
measurement of a particular device.

Comparing across packages is also limited: `docs/evidence/delivery-hardening-browser.md`
reported a Chrome DevTools trace (LCP 135–137 ms, 97,326 transferred bytes on
`/recommend`), not a Lighthouse score. This document is the first Lighthouse
performance record in the repository.

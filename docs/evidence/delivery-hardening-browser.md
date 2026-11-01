# Delivery hardening browser and HTTP evidence

Date: 2026-11-01. Production build (`cargo build` plus `npm run build`,
`apps/web/dist/assets/index-DO5RYnOJ.js` 307,491 bytes,
`apps/web/dist/assets/index-B-c8V-0n.css` 27,698 bytes,
`apps/web/dist/index.html` 664 bytes), served by `target/debug/rigcraft` on
`127.0.0.1:33210` with the scratch data directory
`/tmp/opencode/delivery-hardening/data`. The data directory holds the shipped
sample as snapshot `fa559ef5-104b-49c1-a5d3-df299e58bbbf` revision 1 (10
records, 10 units, US$2,120.90). Header and byte numbers were read from the
running service with `curl`; performance numbers were read from the Chrome
DevTools MCP trace and the running page. Nothing below is estimated.

## Before the change

Measured with the same binary and data before the layers were added:

```text
GET /                    200 text/html                    664 bytes   (no Vary, no Cache-Control, no security headers)
GET /recommend           200 text/html; charset=utf-8     664 bytes   (none)
GET /assets/index-DO5RYnOJ.js 200 text/javascript     307,491 bytes   (none)
GET /assets/index-B-c8V-0n.css 200 text/css            27,698 bytes   (none)
GET /api/health          200 application/json              46 bytes   (none)
GET /nope                404 text/plain; charset=utf-8    118 bytes   (none)
GET /favicon.ico         200 image/x-icon               1,653 bytes   (none)
```

None of those responses carried `Content-Encoding`, `Vary`, `Cache-Control`,
`X-Content-Type-Options`, `Referrer-Policy`, `X-Frame-Options` or a
Content-Security-Policy.

## After the change — `curl -i` matrix

```text
GET /recommend
HTTP/1.1 200 OK
content-type: text/html; charset=utf-8
content-length: 664
vary: accept-encoding
x-content-type-options: nosniff
referrer-policy: no-referrer
x-frame-options: DENY
content-security-policy: default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'self' https: http://127.0.0.1:* http://localhost:*; frame-ancestors 'none'; base-uri 'none'; object-src 'none'; form-action 'self'
cache-control: no-cache

GET /api/health
HTTP/1.1 200 OK
content-type: application/json
x-content-type-options: nosniff
referrer-policy: no-referrer
x-frame-options: DENY
content-security-policy: default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'self' https: http://127.0.0.1:* http://localhost:*; frame-ancestors 'none'; base-uri 'none'; object-src 'none'; form-action 'self'
vary: accept-encoding
cache-control: no-store
content-length: 46

GET /nope
HTTP/1.1 404 Not Found
content-type: text/plain; charset=utf-8
content-length: 118
x-content-type-options: nosniff
referrer-policy: no-referrer
x-frame-options: DENY
content-security-policy: default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'self' https: http://127.0.0.1:* http://localhost:*; frame-ancestors 'none'; base-uri 'none'; object-src 'none'; form-action 'self'
vary: accept-encoding

GET /api/nope
HTTP/1.1 404 Not Found
content-type: application/json
x-content-type-options: nosniff
referrer-policy: no-referrer
x-frame-options: DENY
content-security-policy: default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'self' https: http://127.0.0.1:* http://localhost:*; frame-ancestors 'none'; base-uri 'none'; object-src 'none'; form-action 'self'
vary: accept-encoding
cache-control: no-store
content-length: 79
```

Cache policy, measured per response: `/assets/index-DO5RYnOJ.js` →
`public, max-age=31536000, immutable`; `/` and `/recommend` → `no-cache`;
`/favicon.ico` → `public, max-age=86400`; `/api/health` → `no-store`;
`/assets/missing.js` (404) and `/nope` (404) → no `Cache-Control` at all, so a
missing asset is never advertised as immutable.

## Compression, measured

`curl -s -o /dev/null -w '%{size_download}'`:

| Request | identity | `Accept-Encoding: gzip` | `Accept-Encoding: br` |
| --- | ---: | ---: | ---: |
| `/` | 664 | 397 | 333 |
| `/recommend` | 664 | 391 | 331 |
| `/assets/index-DO5RYnOJ.js` | 307,491 | 86,177 | 87,370 |
| `/assets/index-B-c8V-0n.css` | 27,698 | 5,115 | 5,358 |
| `/api/health` | 46 | 46 | 46 |
| `/nope` | 118 | 118 | 118 |
| `/favicon.ico` | 1,653 | 1,653 | 1,653 |

The 46-byte health reply and the 118-byte 404 are below the 256-byte floor and
stay identity; the icon is excluded as an image and never re-encoded.

Negotiated headers:

```text
GET /recommend with Accept-Encoding: gzip
HTTP/1.1 200 OK
content-type: text/html; charset=utf-8
vary: accept-encoding
content-encoding: gzip
content-security-policy: …same policy…
cache-control: no-cache

GET /assets/index-DO5RYnOJ.js with Accept-Encoding: br
HTTP/1.1 200 OK
content-type: text/javascript
vary: accept-encoding
content-encoding: br
cache-control: public, max-age=31536000, immutable
transfer-encoding: chunked
```

Neither compressed response carries `Content-Length`, so the stale length is
not advertised. The decompressed bytes are the same asset: SHA-256
`9275341ae754b63ac672a8f89339c81d00de93faff3a560c44105fd833f12dbc` for both
`Accept-Encoding: identity` and `curl --compressed` on
`/assets/index-DO5RYnOJ.js`.

## CSP and browser verdict

`npm run test:e2e` with the policy active: **28 passed in 13.1 s**, one worker,
no retries, real Chrome. The new `e2e/delivery.spec.ts`:

1. loads `/`, `/inventory`, `/catalog`, `/builds/check`, `/recommend` and
   `/diagnose`, asserts `main` is visible and the title is RigCraft on each, then
   `page.goto("/recomend")` returns status **404** with the text
   `404 Not found.` and no `#root`; the collected CSP console messages and page
   errors are **empty**.
2. starts a Node HTTP server on a free `127.0.0.1` port, fills the assistant's
   provider URL `http://127.0.0.1:<port>/v1`, key, model and sentence, ticks
   consent, clicks `Interpret sentence`, sees
   `Proposed request — review and confirm`, confirms it and reads
   `2000.00` back in the budget field. The stub received the provider request
   and the console showed no CSP violation.

The CSP is
`default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'self' https: http://127.0.0.1:* http://localhost:*; frame-ancestors 'none'; base-uri 'none'; object-src 'none'; form-action 'self'`.
`apps/web/index.html` contains no inline `<script>`; the only script is the Vite
module. Each allowance is justified in
[delivery-headers.md](../delivery-headers.md).

## Performance, measured once before and twice after

Method: Chrome DevTools MCP `performance_start_trace`, production build,
`/recommend`, 1440 × 900, no CPU throttling and no network throttling, with
`ignoreCache` for the cold runs. LCP and CLS are the trace tool's observed lab
metrics; transferred bytes are the sum of `transferSize` over the navigation and
resource entries in the running page.

| Run | LCP | CLS | Transferred bytes |
| --- | ---: | ---: | ---: |
| Before, cold (`ignoreCache`) | 69 ms | 0.01 | 364,831 |
| After, cold, run 1 | 135 ms | 0.01 | 97,326 |
| After, cold, run 2 | 137 ms | 0.01 | — |
| After, warm reload (cache enabled) | 119 ms | 0.02 | 3,998 |

Cold transferred bytes fall from 364,831 to 97,326 (−73.3%): the JavaScript
drops from 307,491 to 87,370 decoded-to-encoded bytes with `br` (86,177 with
`gzip`), the CSS from 27,698 to 5,358, and the 27,238-byte snapshot from 27,238
to 2,527. A warm reload transfers 3,998 bytes because the two hashed assets are
served from the browser cache (`transferSize` 0 for both) while the shell is
revalidated with `no-cache`.

Honest reading of the trace: the byte reduction is measured and large, but the
LCP numbers do not show an improvement — 69 ms before versus 135/137 ms after on
a single unthrottled run each, with TTFB 2–9 ms in all runs and render delay
dominating. LCP moved in the wrong direction by more than the change can
plausibly explain, which is what single-run lab variance looks like on a local
machine with no throttling; the change does not claim an LCP score or a
Lighthouse score it did not measure. No Lighthouse performance run was made for
this change.

## Privacy and limits

The scratch data is the shipped synthetic sample; no real inventory, provider
key or private file is in any measurement. The Node stub held no credential
(the key typed in the test was `sk-loopback-not-real`) and its process was
closed by the test. No capture was added: headers are invisible, so the visual
record is unchanged and the existing route captures still describe the pages.

Not done: no Lighthouse performance score, no visual-diff baseline, no
non-Chrome browser, no manual screen-reader pass. Compression CPU cost was not
benchmarked under load.

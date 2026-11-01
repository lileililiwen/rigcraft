# Design: Send the same bytes efficiently under a defensible header policy

## Context

Measured before the change on `127.0.0.1:33210` with the production build
(`index-DO5RYnOJ.js`, `index-B-c8V-0n.css`) and the shipped sample snapshot
`fa559ef5-104b-49c1-a5d3-df299e58bbbf` revision 1:

| Request | Status | Content type | Bytes | `Cache-Control` | Security headers |
| --- | --- | --- | --- | --- | --- |
| `/` | 200 | `text/html` | 664 | none | none |
| `/recommend` | 200 | `text/html; charset=utf-8` | 664 | none | none |
| `/assets/index-DO5RYnOJ.js` | 200 | `text/javascript` | 307,491 | none | none |
| `/assets/index-B-c8V-0n.css` | 200 | `text/css` | 27,698 | none | none |
| `/api/health` | 200 | `application/json` | 46 | none | none |
| `/nope` | 404 | `text/plain; charset=utf-8` | 118 | none | none |
| `/favicon.ico` | 200 | `image/x-icon` | 1,653 | none | none |

`build_router` mounted the API routes, applied `DefaultBodyLimit` and
`TraceLayer`, then fell back to `ServeDir` over the SPA fallback. No layer looked
at the response after routing, which is why every response above lacks even
`nosniff`.

## Decision 1 — two `tower-http` features, no new direct dependency

`Cargo.toml` keeps the single direct `tower-http = "0.6"` line and adds
`compression-gzip` and `compression-br`. `tower-http` already owns the layer
that reads `Accept-Encoding`, skips already-compressed responses, and removes a
`Content-Length` it can no longer vouch for; a hand-written encoder would
reimplement negotiation and body framing for no gain.

The lockfile gains seven transitive packages. Each is required by the enabled
features, and no algebraically unrelated crate arrives with them:

| Crate added | Version | Why it is there |
| --- | --- | --- |
| `async-compression` | 0.4.51 | Async encode/decode adapters `tower-http` drives |
| `compression-codecs` | 0.4.46 | gzip/br codec wiring behind those adapters |
| `compression-core` | 0.4.34 | Shared traits/errors for the codecs |
| `brotli` | 9.0.0 | Brotli encoder |
| `brotli-decompressor` | 6.0.1 | Brotli decoder required by `brotli` |
| `alloc-no-stdlib` | 3.0.0 | Allocation shim used by `brotli` |
| `alloc-stdlib` | 0.3.0 | `std` allocation adapter for the shim |

`flate2`, `miniz_oxide` and `crc32fast` are already in the lockfile before this
change (they arrive through the existing dependency graph), so the gzip path
adds no new crate beyond the list above. `compression-zstd` is not enabled, so
no zstd crate and no zstd negotiation exist.

## Decision 2 — one policy layer over the finished router

The layers are applied after the `ServeDir`/SPA fallback is attached:

```rust
router
    .layer(CompressionLayer::new().compress_when(compressible))
    .layer(from_fn(delivery_policy))
```

Applying them to the returned `Router` is what makes the fallback service
covered; layering before `fallback_service` would leave static files and the
SPA untouched. The policy layer is outermost, so it reads the response after
compression has added its own `Vary` and does not append a duplicate.

`cache_control(path, content_type, status)` is the only place a cache decision
is made:

| Response | Policy | Reason |
| --- | --- | --- |
| successful `/assets/*` | `public, max-age=31536000, immutable` | Vite hashes the file name, so those bytes never change |
| `/favicon.ico`, `/favicon.svg` | `public, max-age=86400` | Stable icon, one day is enough |
| `text/html` response (the shell, including `/` and every client route) | `no-cache` | The shell names the current asset hashes and must be revalidated after a deploy |
| `/api` and `/api/*` | `no-store` | Private inventory and provider traffic must not be cached |
| anything else, including a missing asset or the 404 document | no header | A 404 or metadata document is not a cacheable representation |

The status is part of the decision so `/assets/missing.js` (404) never receives
the immutable policy. Content type is part of it so `index.html` requested
directly on disk gets the same `no-cache` as `/recommend`.

## Decision 3 — compress when the saving is real

`DefaultPredicate` compresses anything at or above 32 bytes. Measured on this
service, `/api/health` is 46 bytes plain and 69 bytes gzipped — framing made it
larger. The predicate is therefore:

```rust
SizeAbove::new(256)
    .and(NotForContentType::GRPC)
    .and(NotForContentType::IMAGES)
    .and(NotForContentType::SSE)
```

- 256 bytes is the floor where this service's savings stop being noise. The
  664-byte shell compresses to 331 bytes (br) / 391 (gzip); an 118-byte 404 and
  a 46-byte JSON reply stay identity.
- `NotForContentType::IMAGES` keeps `image/x-icon` and every other raster image
  out. `image/svg+xml` is the documented exception and is text, so the favicon
  SVG is eligible while `favicon.ico` is never re-encoded.
- `tower-http` never compresses a response that has `Content-Range` or
  `Content-Encoding`, so a range request keeps its range semantics and
  `Accept-Ranges` and a compressed body loses `Content-Length` rather than
  advertising a length that describes the wrong representation.

`Vary: Accept-Encoding` is added by the compression layer when it compresses.
When the client does not accept an encoding (or the payload is below the
floor), the policy layer adds the same `Vary` to a text response, because the
representation still depends on negotiation. There is exactly one `Vary`
header in either case.

## Decision 4 — the CSP, allowance by allowance

```text
default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline';
img-src 'self' data:; font-src 'self';
connect-src 'self' https: http://127.0.0.1:* http://localhost:*;
frame-ancestors 'none'; base-uri 'none'; object-src 'none'; form-action 'self'
```

- `default-src 'self'` is the fallback: no third-party origin is a default.
- `script-src 'self'` — `apps/web/index.html` has no inline `<script>`; Vite
  emits one external module (`/assets/index-*.js`) and the built shell loads it
  from the same origin. Adding `'unsafe-inline'` here would allow exactly the
  class of injected script this header exists to stop, and nothing needs it.
- `style-src 'self' 'unsafe-inline'` — the stylesheet is external, but React
  writes a `style` attribute through the DOM `style` property, which CSP treats
  as inline style. No component uses a `style` attribute today
  (`ui_conformance.py` also rejects one), so this is a compatibility margin
  rather than a present need; it cannot execute code.
- `img-src 'self' data:` — the icons are same-origin files. `data:` admits an
  image built in the DOM (a data-URI preview or icon) without granting a
  script-capable scheme. No `data:` image exists in the current source.
- `font-src 'self'` — the app uses the system UI and mono stacks and ships no
  webfont; only same-origin font files would load, so no external font can be
  silently introduced.
- `connect-src 'self' https: http://127.0.0.1:* http://localhost:*` — the
  browser's only request is same-origin (`/api/ai/parse`, `/api/ai/revise`, the
  inventory API). The assistant accepts a user-typed provider base URL, and
  `validate_provider_url` permits `https` everywhere and plain HTTP only for a
  loopback host, so the policy mirrors that rule instead of naming one vendor.
  HTTPS is deliberately broad because the provider name is the operator's
  choice; plain HTTP is restricted to the two loopback spellings the validator
  documents. `http://[::1]` is not listed, and the current transport is
  server-side, so no browser request is blocked by that omission.
- `frame-ancestors 'none'` and `X-Frame-Options: DENY` together stop both a
  modern and a legacy browser from framing the app.
- `base-uri 'none'` stops an injected `<base>` from retargeting relative URLs;
  `object-src 'none'` stops plugin embedding; `form-action 'self'` stops a form
  post leaving the origin.

The remaining headers are `X-Content-Type-Options: nosniff` (no MIME sniffing),
`Referrer-Policy: no-referrer` (no path or query leaks to any destination) and
`X-Frame-Options: DENY`. All four are inserted on every response, including an
API `400`, a static file and the plain-text 404, because a header that only
covers happy pages leaves the interesting responses unprotected.

## Decision 5 — measure, then state what the measurement says

`/recommend` was traced with the Chrome DevTools MCP `performance_start_trace`
tool, production build, no CPU or network throttling, on the scratch data
directory. The same trace tool reported CLS and LCP; transferred bytes were
read from `PerformanceResourceTiming` in the running page.

The honest result is in `docs/evidence/delivery-hardening-browser.md`: the
transfer falls from 364,831 to 97,326 bytes on a cold load and to 3,998 bytes
on a warm reload, while the single-run LCP numbers (69 ms before, 135/137 ms
after) are lab variance on one host under no throttling rather than a proven
LCP improvement. The change claims the byte reduction it measured, not a score
it did not.

## Risks and limits

- Compression spends CPU on responses above 256 bytes; the alternative is
  sending 307 KB of JavaScript on every cold visit.
- A CSP that blocks the bundle would be worse than no CSP. The Playwright
  journeys load all six routes, assert no CSP console violation and drive the
  BYOK loopback provider; the Rust tests assert the header matrix.
- `ServeDir` range requests are never compressed, so a compressed response
  gives up `Content-Length` but not byte-range semantics.
- No zstd, no CDN and no precompressed sidecar files are introduced; the
  service still serves the same files it serves today.

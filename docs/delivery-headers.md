# Delivery headers: compression, cache and security policy

The service applies one delivery policy to the finished router, so the API, the
built static files, the SPA fallback and the plain-text 404 all receive the same
treatment. The policy lives in `src/api.rs` (`delivery_policy`, `cache_control`,
`CONTENT_SECURITY_POLICY`) and is covered by `tests/delivery_headers.rs`,
`e2e/delivery.spec.ts` and the recorded run in
[evidence/delivery-hardening-browser.md](evidence/delivery-hardening-browser.md).

## Security headers

Every response, including a JSON `400`/`404`, a static file and the 404
document, carries:

| Header | Value |
| --- | --- |
| `X-Content-Type-Options` | `nosniff` |
| `Referrer-Policy` | `no-referrer` |
| `X-Frame-Options` | `DENY` |
| `Content-Security-Policy` | `default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'self' https: http://127.0.0.1:* http://localhost:*; frame-ancestors 'none'; base-uri 'none'; object-src 'none'; form-action 'self'` |

Why each allowance exists:

- `script-src 'self'` is enough because `apps/web/index.html` has no inline
  `<script>`; the Vite build emits one external module.
- `style-src 'self' 'unsafe-inline'` covers a React `style` attribute (set
  through the DOM `style` property, which CSP counts as inline). No component
  uses one today; it cannot execute code.
- `img-src 'self' data:` admits a DOM-built data-URI image; the shipped icons
  are same-origin files.
- `font-src 'self'` matches the system UI/mono stacks: the app ships no webfont.
- `connect-src 'self' https: http://127.0.0.1:* http://localhost:*` keeps the
  same-origin API and the operator-typed BYOK provider address that
  `validate_provider_url` accepts (HTTPS anywhere, plain HTTP only for loopback).
- `frame-ancestors 'none'` plus `X-Frame-Options: DENY` blocks framing in both
  modern and legacy browsers; `base-uri`, `object-src` and `form-action`
  restrict the remaining injected-content paths.

## Compression

`tower-http`'s `CompressionLayer` negotiates `gzip` and `br` for `text/*`,
`application/json`, `application/javascript` and `image/svg+xml` above 256
bytes. Below that floor the framing costs more than the saving. Images other
than SVG are excluded, so `favicon.ico` is never re-encoded. A compressed
response has no `Content-Length` and no `Accept-Ranges`; a response with
`Content-Range` is never compressed. `Vary: Accept-Encoding` is present on
every text response whether or not this request was compressed.

Only `tower-http`'s `compression-gzip` and `compression-br` features were
enabled; zstd is not compiled in.

## Cache policy

| Response | `Cache-Control` |
| --- | --- |
| successful `/assets/*` (Vite content hash in the name) | `public, max-age=31536000, immutable` |
| `index.html` and every `text/html` response | `no-cache` |
| `/favicon.ico`, `/favicon.svg` | `public, max-age=86400` |
| `/api` and `/api/*` | `no-store` |
| 404 document, missing asset, other files | none |

## Measured byte matrix

`curl -s -o /dev/null -w '%{size_download}'` against the production build on
`127.0.0.1:33210`:

| Request | identity | `Accept-Encoding: gzip` | `Accept-Encoding: br` |
| --- | ---: | ---: | ---: |
| `/` | 664 | 397 | 333 |
| `/recommend` | 664 | 391 | 331 |
| `/assets/index-DO5RYnOJ.js` | 307,491 | 86,177 | 87,370 |
| `/assets/index-B-c8V-0n.css` | 27,698 | 5,115 | 5,358 |
| `/api/health` | 46 | 46 | 46 |
| `/nope` | 118 | 118 | 118 |
| `/favicon.ico` | 1,653 | 1,653 | 1,653 |

`/api/health` and `/nope` are below the 256-byte floor and stay identity; the
icon is excluded as an image. A compressed asset response reads:

```text
HTTP/1.1 200 OK
content-type: text/javascript
vary: accept-encoding
content-encoding: br
x-content-type-options: nosniff
referrer-policy: no-referrer
x-frame-options: DENY
content-security-policy: default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'self' https: http://127.0.0.1:* http://localhost:*; frame-ancestors 'none'; base-uri 'none'; object-src 'none'; form-action 'self'
cache-control: public, max-age=31536000, immutable
transfer-encoding: chunked
```

There is no `content-length` on the compressed representation, which is what
keeps `Content-Length` honest.

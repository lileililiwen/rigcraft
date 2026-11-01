# inventory-import deltas

## ADDED Requirements

### Requirement: Text responses negotiate gzip or brotli and declare the negotiation
The service SHALL honour `Accept-Encoding` for text responses whose media type is
`text/*`, `application/json`, `application/javascript` or `image/svg+xml`, and SHALL
answer with `Content-Encoding: gzip` or `Content-Encoding: br` when the client asks
for one of those codings and the plain body is at least 256 bytes. A response that is
compressed SHALL NOT carry a `Content-Length` that describes the uncompressed body and
SHALL keep range semantics by never compressing a response that has `Content-Range`.
A response whose `Content-Encoding` is not negotiated SHALL be the identity bytes with
an accurate `Content-Length`. An `image/x-icon` response SHALL NOT be re-encoded even
when a coding is offered. Every text response SHALL declare `Vary: Accept-Encoding`,
whether or not this request was compressed.

#### Scenario: A page negotiates compression
Given a built Web app and a client that sends `Accept-Encoding: gzip`, when `/recommend` is requested, then the response has `Content-Encoding: gzip`, no stale `Content-Length`, and `Vary: Accept-Encoding`.

#### Scenario: A client that asks for nothing gets the plain bytes
Given a built Web app and a client with no `Accept-Encoding`, when `/recommend` is requested, then the response has no `Content-Encoding`, its `Content-Length` equals the body length, and it still declares `Vary: Accept-Encoding`.

#### Scenario: An icon is never re-compressed
Given a built Web app and a client that accepts `gzip` and `br`, when `/favicon.ico` is requested, then the response is the identity icon with `Content-Length` and no `Content-Encoding`.

### Requirement: Static assets and pages declare one cache policy
The service SHALL apply one cache policy to the whole delivery surface. A successful
`GET /assets/*` SHALL be `Cache-Control: public, max-age=31536000, immutable`.
`index.html` and every HTML response SHALL be `Cache-Control: no-cache`.
`/favicon.ico` and `/favicon.svg` SHALL be `Cache-Control: public, max-age=86400`.
Every response under `/api` SHALL be `Cache-Control: no-store`. A missing asset, a
404 document and any other response SHALL NOT receive the immutable policy. The policy
SHALL be decided in one place for every route rather than by an individual handler.

#### Scenario: A hashed bundle is immutable
Given a built Web app, when `/assets/index-abc123.js` is requested, then the response is `200` with `Cache-Control: public, max-age=31536000, immutable`.

#### Scenario: The shell is revalidated
Given a built Web app, when `/` or `/recommend` is requested, then the response is `200` `text/html` with `Cache-Control: no-cache`.

#### Scenario: A missing asset is not immutable
Given a built Web app, when `/assets/missing.js` is requested, then the response is `404` and carries no immutable `Cache-Control`.

#### Scenario: Inventory and assistant traffic is not stored
Given a running service, when `/api/health` is requested, then the response is `Cache-Control: no-store`.

### Requirement: Every response carries a baseline security header policy
Every response SHALL carry `X-Content-Type-Options: nosniff`,
`Referrer-Policy: no-referrer`, `X-Frame-Options: DENY` and
`Content-Security-Policy` with the value
`default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'self' https: http://127.0.0.1:* http://localhost:*; frame-ancestors 'none'; base-uri 'none'; object-src 'none'; form-action 'self'`.
This SHALL include an API response, a static asset and the 404 document, not only a
rendered page. The build SHALL contain no inline `<script>`, so `script-src 'self'`
loads the bundle. With the policy active, all six client routes SHALL render, the
assistant SHALL still reach an operator-supplied loopback provider through the service,
and the browser console SHALL report no CSP violation.

#### Scenario: A page carries the full policy
Given a built Web app, when `/recommend` is requested, then the response has `nosniff`, `no-referrer`, `X-Frame-Options: DENY`, and a CSP whose `script-src` is `'self'` without `'unsafe-inline'` or `'unsafe-eval'`.

#### Scenario: An API error carries the same policy
Given a running service, when `/api/nope` is requested, then the JSON 404 response also carries the four security headers.

#### Scenario: The 404 document carries the same policy
Given a built Web app, when `/recomend` is requested, then the plain-text 404 response also carries the four security headers.

#### Scenario: The policy does not break the app
Given the production build served with the policy, when each of `/`, `/inventory`, `/catalog`, `/builds/check`, `/recommend` and `/diagnose` is loaded and the assistant is pointed at a loopback provider, then every route renders, the provider request completes, and the console shows no CSP violation.

#### Scenario: A typo is still a real 404 under the policy
Given the production build served with the policy, when `/recomend` is visited, then the browser receives status `404` and the not-found document rather than the SPA shell.

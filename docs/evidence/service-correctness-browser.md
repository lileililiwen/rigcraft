# Service correctness browser evidence

Date: 2026-11-01. Production build (`cargo build` plus `npm run build`,
`apps/web/dist/assets/index-B-c8V-0n.css` / `index-DO5RYnOJ.js`), served by
`target/debug/rigcraft` on `127.0.0.1:33171` with the scratch data directory
`/tmp/opencode/service-correctness/data`. Captures and measurements were taken
with the Chrome DevTools MCP; every number below was read from the running
service or the running page, not estimated.

The data directory holds one committed snapshot,
`9e693fbb-c428-4d8b-becb-fc9d876046b5` revision 1, imported from a CSV whose model
is `=1+1`, so the same snapshot proves the CSV neutralisation and provides the
download used for the captures.

## Before the change

Measured against the unmodified build with the same commands:

| Request | Status | Content type | Body |
| --- | --- | --- | --- |
| `GET /api` | 200 | `text/html; charset=utf-8` | SPA shell `<title>RigCraft…</title>` |
| `GET /api/` | 200 | `text/html; charset=utf-8` | SPA shell |
| `GET /api/nope` | 404 | `application/json` | `route_not_found` |
| `GET /API/health` | 200 | `text/html; charset=utf-8` | SPA shell |
| `GET /nope` | 200 | `text/html; charset=utf-8` | SPA shell (renders Inventory) |
| `GET /recomend` | 200 | `text/html; charset=utf-8` | SPA shell (renders Inventory) |
| `GET /assets/missing.js` | 404 | `text/plain; charset=utf-8` | `Not found.` |

The exported CSV row was `X-1,cpu,=1+1,1,10.00,USD,socket=AM5,user,` — the model
cell was the raw `=1+1` (`0x3d 0x31 0x2b 0x31`), which a spreadsheet evaluates.

A semicolon-delimited upload returned `400 mapping_missing` with only
`Required columns are missing: sku, category, model, quantity, price.` and
`details.missing`; nothing named the delimiter.

## After the change — `curl -i` matrix

```text
GET /api                 404 application/json
GET /api/                404 application/json
GET /api/nope            404 application/json
GET /API/health          404 text/plain; charset=utf-8
GET /nope                404 text/plain; charset=utf-8
GET /recomend            404 text/plain; charset=utf-8
POST /api/health         405 (no body)
GET /                    200 text/html
GET /inventory           200 text/html; charset=utf-8
GET /inventory/          200 text/html; charset=utf-8
GET /catalog             200 text/html; charset=utf-8
GET /builds/check        200 text/html; charset=utf-8
GET /recommend           200 text/html; charset=utf-8
GET /diagnose            200 text/html; charset=utf-8
GET /assets/missing.js   404 text/plain; charset=utf-8
```

Bodies:

- `GET /api`, `GET /api/` and `GET /api/nope` return
  `{"error":{"code":"route_not_found","message":"This API route does not exist."}}`
  with `application/json`.
- `GET /nope`, `GET /recomend` and `GET /API/health` return
  `404 Not found.\nRigCraft has no page at this address. Open /inventory to import stock, or /catalog to manage datasets.`
  with `text/plain; charset=utf-8` and no `<title>`, no `id="root"` and no Inventory
  page.
- `GET /inventory//` is a 404 document as well; a single trailing slash is the one
  extra spelling accepted for each of the six routes.
- `POST /api/health` is still a 405, so a real route keeps its wrong-method answer.

## CSV neutralisation, measured

```text
$ curl -sD headers.txt -o downloaded.csv \
  'http://127.0.0.1:33171/api/inventory/snapshots/9e693fbb-…/export?format=csv'

HTTP/1.1 200 OK
content-type: text/csv; charset=utf-8
content-disposition: attachment; filename="rigcraft-inventory-snapshot-r1-9e693fbb.csv"
x-rigcraft-export-lossy: true
x-rigcraft-export-note: CSV export is lossy: it omits lot identity, snapshot metadata
  and per-field provenance references. Use format=json for the authoritative artifact.
content-length: 109
```

The file is 109 bytes, SHA-256
`ffc53c745b5434e80331f217d5256126c3165285f45a8b1fa2911c4ae6ff6890`, and reads:

```text
sku,category,model,quantity,price,currency,specs,source,sourceUrl
X-1,cpu,'=1+1,1,10.00,USD,socket=AM5,user,
```

The model cell is `'=1+1`: `0x27 0x3d 0x31 0x2b 0x31`, five characters where the
stored value has four. The JSON export of the same snapshot carries
`"modelRaw": "=1+1"`, so the apostrophe is the only difference between the two
artifacts and the JSON value is byte-for-byte the stored value.

## Delimiter hint, measured

```json
{
  "error": {
    "code": "mapping_missing",
    "details": { "delimiter": "semicolon", "missing": ["sku", "category", "model", "quantity", "price"] },
    "field": "sku",
    "message": "Required columns are missing: sku, category, model, quantity, price. The header looks semicolon-delimited; RigCraft expects comma-separated columns."
  }
}
```

The code, field and missing list are unchanged; the message names the cause and
does not echo the file's rows. A comma-separated file that genuinely omits
`category` returns and keeps a `mapping_missing` refusal with no `delimiter`
detail and no semicolon sentence (asserted in `tests/api.rs`).

## Captures

| Capture | Route | Viewport | Condition |
| --- | --- | --- | --- |
| `docs/assets/service-correctness-404-desktop.png` | `/recomend` | 1440×900 | Real 404 document, status 404, `text/plain` |
| `docs/assets/service-correctness-404-mobile.png` | `/recomend` | 390×844 | The same document at phone width |
| `docs/assets/service-correctness-csv-desktop.png` | downloaded CSV bytes | 1440×900 | Export of snapshot `9e693fbb…`, model cell `'=1+1` |
| `docs/assets/service-correctness-csv-mobile.png` | downloaded CSV bytes | 390×844 | The same bytes at phone width |

The 404 document measured `document.contentType === "text/plain"`, an empty
`document.title`, `document.getElementById("root") === null`, and
`scrollWidth === clientWidth` at both 1440 and 390 (390 / 390).

Chrome opens a `text/csv` attachment as a download, so the CSV captures show the
exact 109 downloaded bytes rendered as text rather than a browser-rendered table.
The rendering was produced from a `data:text/plain` URL built from those bytes, and
the byte content was confirmed in the page with `document.body.innerText` before the
screenshot; the file itself was never edited.

## Accessibility and Lighthouse

The 404 response is a plain-text document, not an HTML page. It has no document
structure, landmarks, headings, links or controls for Lighthouse's accessibility,
SEO or best-practice audits to evaluate, so no `lighthouse_audit` run is applicable
and none is reported as a score. The honest statement is that it is readable,
non-interactive text: the browser's own default styling (black text on white,
user-scalable) keeps it legible, it contains no colour literal, no HTML and no
script, and it does not wrap horizontally at 390 px.

No existing route was restyled, so the six page audits recorded in
`docs/evidence/dataops-lighthouse/`, `docs/evidence/verdict-lighthouse/` and
`docs/evidence/ui-foundation-lighthouse/` remain the accessibility evidence for the
pages themselves; `npm run test:e2e` still passes all 26 journeys, including the
workbench, verdict and dataops layout assertions.

## Privacy

The evidence shows one synthetic row (`X-1`, model `=1+1`, `socket=AM5`) imported
into a scratch data directory. No real inventory, catalog document, provider key or
private file is in any capture. The 404 body is a constant and never echoes the
requested path.

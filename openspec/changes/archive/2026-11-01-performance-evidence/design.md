# Design: A lab performance record and a budget the build cannot skip

## Context

The production build at the start of this change is two files:
`apps/web/dist/assets/index-DO5RYnOJ.js` (307,491 bytes, SHA-256
`9275341a…f12dbc`) and `index-B-c8V-0n.css` (27,698 bytes, SHA-256
`506961cd…650e98`), plus a 664-byte `index.html`. The service already negotiates
gzip or brotli, marks hashed assets immutable and sets `Vary` and the security
headers, so the delivered bytes are smaller than these on-disk sizes. The
App is a single React bundle; there is no router-level code splitting and one
stylesheet.

`./verify-local.sh` runs Node stages in the order `npm ci`, `check`, `typecheck`,
`test`, `build`, `test:e2e`. The bundle does not exist when `check` runs, so a
check that reads `dist/` placed in `check` before the build would either pass on
a stale build or fail on a fresh checkout.

## Decision 1 — the budget lives in `ui-foundation`, not `inventory-import`

`inventory-import` owns the HTTP delivery surface (compression, cache, security
headers). A bundle ceiling is not an HTTP policy: it constrains what the client
build may emit. The client-asset policy — the brand layer, the measured contrast
pair set, the shell chrome, the icon — is already `ui-foundation`. Both new
requirements are recorded there, so the spec keeps one owner for client assets
and the delivery requirements are neither duplicated nor weakened.
`inventory-import` is not modified.

## Decision 2 — explicit ceilings in the script, per file type

```js
export const CEILINGS = { js: 330_000, css: 32_000 };
```

The ceilings sit 7.3% and 15.5% above the current build. That is enough for
ordinary patch-level churn in React or Vite, and far below a change that adds a
dependency, a route bundle or a copied asset. The ceiling is per file type, not
per file name, so a renamed hash cannot evade it; a new chunk is measured too.
The numbers are declared in the script rather than read from a config file,
because a ceiling that can be supplied by the environment or edited in a build
tool is not a fixed budget. Raising it is a code change with a review.

The check fails when `apps/web/dist/assets` is missing or holds no `.js`/`.css`
file; a fresh checkout cannot pass by having nothing to measure.

## Decision 3 — enforce it in the build chain and in `check`, after a build

`npm run build` becomes
`npm --workspace @rigcraft/web run build && node scripts/check-bundle-size.mjs`.
The artifact-producing command checks its own output, so an over-budget build
fails the moment it is produced, in CI or locally.

`npm run check` also runs the check (and the checker's own fixture test), which
matches the package requirement that the budget is enforced by `npm run check`.
That means `check` needs a build, so `./verify-local.sh` reorders its Node
stages to `npm ci`, `build`, `check`, `typecheck`, `test`, `test:e2e`. No stage
is removed; only the order changes. A developer who runs `npm run check` on a
tree with no build gets a named failure that tells them to run `npm run build`,
not a silent pass. The alternative — put the check only in `check` and leave the
order alone — would make a green `npm run build` able to emit an over-budget
bundle, which is the outcome the requirement exists to prevent.

## Decision 4 — measurement method

The service is run from `target/debug/rigcraft` against a scratch data directory,
seeded through the documented API: the shipped sample previewed and committed
(10 records, revision 1, snapshot `6165c7cb…cb6f`) and the synthetic catalog
document used by `e2e/catalog.spec.ts` uploaded through
`POST /api/catalog/datasets` (`synthetic-fixture`, 3 parts, active). Lighthouse
13.6.0 is invoked once per route and form factor:

```sh
npx --yes lighthouse@13.6.0 <url> \
  --only-categories=performance \
  --chrome-flags="--headless=new --no-sandbox --disable-gpu" \
  --output=json --output-path=<path>
```

The default form factor is mobile (Lighthouse's mobile emulation plus Lantern
simulated throttling: 4× CPU, 1.6 Mbps). Desktop adds `--preset=desktop`.
Each report is written to `docs/evidence/perf-lighthouse/<route>/report.json`
(desktop) or `docs/evidence/perf-lighthouse/<route>-mobile/report.json`
(mobile). No score is edited or rounded; the table is read from the committed
JSON.

The arrival state of each route is measured as a user meets it with seeded data:
`/inventory` opens the newest snapshot automatically, `/catalog` shows the
registry, and `/builds/check` and `/diagnose` show their empty selection forms.
Generating a check, recommendation or diagnosis needs interaction, which a
navigation Lighthouse run cannot perform; that limit is stated in the evidence
document rather than hidden.

## Decision 5 — fix only an unambiguous finding

Baseline results are 100 in all twelve runs. The remaining non-perfect audits
are recorded with their numbers:

- `unused-javascript` estimates 51–58 KiB of the single React bundle unused on a
  route. Removing it means route-level code splitting, which changes the module
  graph and adds chunks; it does not raise a score that is already 1.00.
- `render-blocking-insight` names the one stylesheet (5,997 bytes transferred).
  Inlining it would move the CSS into the `no-cache` shell and defeat the
  immutable asset cache; loading it asynchronously would risk a flash of
  unstyled content and layout shift. Neither is an unambiguous improvement.
- `network-dependency-tree-insight` follows document → bundle → snapshot API,
  which is the client-rendered shape of the app. Server rendering is a product
  change, not a performance fix.

No image, font, third-party or duplicate-payload finding exists:
`unsized-images` is 1.00, there are no images beyond the icon, no webfont and no
third-party origin. The build is therefore not edited for performance, and the
asset hashes in the evidence are the ones measured.

## Verification shape

`scripts/check-bundle-size.test.mjs` proves the checker against temporary
fixtures. `npm run build` and `npm run check` run the real check against the
built assets. `./verify-local.sh` shows every previous stage still running in
its new order. The committed Lighthouse JSON is the measurement; the evidence
document is a transcription of it.

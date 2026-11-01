# Proposal: Measure performance and enforce a byte budget

## Why

Every prior package recorded the same residual: "Lighthouse performance was not
measured." The only performance numbers in the repository are a Chrome DevTools
trace of one route (`docs/evidence/delivery-hardening-browser.md`) and a
compression table. There is no measured score for the six client routes and no
ceiling that stops the JavaScript or CSS bundle from growing unnoticed. A
bundle budget that is a sentence in a document is not a budget.

## What Changes

- **One measured performance record.** The production build is served with the
  shipped sample snapshot and the synthetic catalog fixture, and
  `npx --yes lighthouse@13.6.0 … --only-categories=performance` is run against
  `/`, `/inventory`, `/catalog`, `/builds/check`, `/recommend` and `/diagnose`
  on the default mobile form factor and with `--preset=desktop`. Every JSON
  report is committed under `docs/evidence/perf-lighthouse/`, and
  `docs/evidence/performance-evidence.md` tabulates the score, FCP, LCP, TBT,
  CLS, speed index and transferred bytes per route and device, names the asset
  hashes and records the exact command.
- **One deterministic byte budget.** `scripts/check-bundle-size.mjs` reads every
  built `apps/web/dist/assets/*.js` and `*.css`, compares each file against an
  explicit per-type ceiling declared in the script, and exits non-zero naming
  the file, its size, the ceiling and the delta. A missing or empty build is a
  failure, not a pass.
- **The gate runs after the build.** `npm run build` ends by running the check,
  `npm run check` runs it too, and `./verify-local.sh` is reordered so
  `npm run build` precedes `npm run check`. The check therefore always reads the
  fresh build, never a stale or absent `dist/`.
- **A test proves the checker.** `scripts/check-bundle-size.test.mjs` builds two
  fixture directories in `fs.mkdtemp` and proves a within-budget tree passes and
  an over-budget file fails with the named file, size, ceiling and delta.
- **Findings are fixed or recorded.** A Lighthouse finding is fixed only when
  the fix is unambiguous and safe; anything left is recorded with the measured
  number and the reason, not chased with speculative preloading, a service
  worker, a CDN or a dependency.

## Impact

- Affected specs: `ui-foundation` gains two requirements — the measured
  performance record and the enforced client-asset byte ceiling. Both are
  client-asset policy, which is what `ui-foundation` already owns, so no
  requirement is modified or removed and no delivery requirement is
  contradicted.
- Affected code: `scripts/check-bundle-size.mjs` (new),
  `scripts/check-bundle-size.test.mjs` (new), `package.json` (`build` and
  `check`), `verify-local.sh` (Node stage order), `AGENTS.md` (verification
  order), `README.md` (what the checks cover), `docs/`.
- Affected behaviour: none in the product. No colour, layout, route, API shape,
  schema or token change; `apps/web/src/styles/vendor/tokens.css` is untouched.
  No direct dependency is added: Lighthouse is fetched transiently by `npx` for
  the measurement only.
- Non-goals: no service worker, no CDN, no webfont, no analytics, no
  code-splitting rewrite to chase `unused-javascript`, and no field-data claim.
  A localhost Lighthouse run is a lab measurement.

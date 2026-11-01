# Tasks: Measured dark token layer and a defect-free shell

## 1. BFS — Baseline and impact coverage
- [x] 1.1 Read `AGENTS.md`, `.ai-rules/`, `HANDOFF.md`, `ROADMAP.md`, `docs/branding.md`, `docs/design-program.md`, `.project.json`, the vendored tokens, `app.css`, `index.html` and `scripts/check-contrast.mjs`; confirm no other active change and set `current_spec`.
- [x] 1.2 Map impact across the token import order, every semantic alias a component reads, the contrast report, the ui-conformance scope, the five routes and the existing e2e assertions.
- [x] 1.3 Assess shared infrastructure by name (pinned kit `7ca6b795…`, no new dependency, no webfont) and record that the kit is consumed, not modified.

## 2. DFS — Brand token layer
- [x] 2.1 Add `apps/web/src/styles/brand.css` with `--brand-*` primitives, three surfaces, primary/success/warning/danger, borders and focus, and the semantic aliases; `:root` only.
- [x] 2.2 Import it between the vendored tokens and `app.css`; declare `color-scheme: dark` and `accent-color`.
- [x] 2.3 Remove any palette value that would duplicate a brand token from `app.css`, keeping component rules variable-only.
- [x] 2.4 Prove `vendor/tokens.css` is unchanged (`sha256` = `.project.json` pin).

## 3. DFS — Measured evidence
- [x] 3.1 Extend `scripts/check-contrast.mjs` to resolve the ordered file list and report one `{file, sha256}` per source.
- [x] 3.2 Add the dark text, status, chip, border and focus pairs; keep flat values for every text pair and decoration-only `color-mix`/alpha elsewhere.
- [x] 3.3 Fail the check below the AA minimum and regenerate `docs/evidence/token-contrast.json`.
- [x] 3.4 Update `docs/branding.md` with the decided set, the measured table and the dark/light position.

## 4. DFS — Shell defects
- [x] 4.1 Give `.responsive-table caption` a full-width block form inside the ≤ 760 px card layout.
- [x] 4.2 Move `.skip-link` fully off-screen when unfocused; restore `top: var(--space-2)` on `:focus`.
- [x] 4.3 Add the favicon assets under `apps/web/public/` and link them from `index.html`.
- [x] 4.4 Add Playwright assertions for the caption width at 390 px and the unfocused skip-link position, plus a visible-focus check.

## 5. BFS — Regression review
- [x] 5.1 Re-read `app.css` for any rule that assumed a light surface (shadows, `#fff`-style overlays, native control colours, `color-scheme`).
- [x] 5.2 Re-check every route at 320/390/768/1440 for overflow, contrast and focus visibility.
- [x] 5.3 Confirm no component gained a colour literal and the ui-conformance inputs still resolve.

## 6. Verification and evidence
- [x] 6.1 `cargo fmt --all -- --check`; `cargo clippy --all-targets --all-features -- -D warnings`; `cargo test`.
- [x] 6.2 `npm run check`; `npm test`; `npm run build`; `npm run test:e2e`; `./verify-local.sh`.
- [x] 6.3 Lighthouse accessibility 100 in dark on all five routes, desktop and mobile; captures at 1440 and 390; record the focus ring and `scrollWidth === clientWidth`.
- [x] 6.4 Write `docs/evidence/ui-foundation-browser.md`; refresh `docs/assets/capture-plan.md`; update README/token claims that changed.
- [x] 6.5 `node scripts/check-openspec-change-names.mjs`; `openspec validate dark-brand-token-layer --strict`; `openspec validate --all --strict`.
- [x] 6.6 Full Gate; archive with `openspec archive dark-brand-token-layer --yes`; two commits (implementation/archive, then `HANDOFF.md`).

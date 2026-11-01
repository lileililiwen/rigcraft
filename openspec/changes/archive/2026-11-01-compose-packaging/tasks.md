# Tasks: Package the service as a reproducible Compose container

## 1. BFS — Baseline, impact and contract
- [x] 1.1 Read `HANDOFF.md`, `AGENTS.md`, `.ai-rules/`, `docs/design-program.md`, `docs/delivery-headers.md`, `Dockerfile`, `forge.yaml`, `.project.json`, `.standard/compose/docker-compose.yaml`, `src/lib.rs`, `src/api.rs`, `package.json`, `apps/web/vite.config.ts`, `LICENSE`, `NOTICE`; confirm no active change and set `current_spec`.
- [x] 1.2 Record the broken baseline: the Dockerfile builds `target/release/app` while `Cargo.toml` names `rigcraft`, and no stage builds `apps/web`.
- [x] 1.3 Read Forge's contract: `COMPOSE_FILES`, the `docker-compose` kind and `DeploymentHealthMeta`; verify the pinned base-image tags with `docker manifest inspect` rather than assuming them.
- [x] 1.4 Map impact: build context, `.dockerignore`, runtime library needs, `Config::from_env` variables, `/api/health`, the SQLite file at `/data`, the Forge manifest and `.project.json`, `npm run check`, `.standard/**` and `verify-local.sh` (both pack-owned and not edited).

## 2. DFS — Image
- [x] 2.1 Rewrite `Dockerfile` as a Node stage (`npm ci` from the root workspace lockfile, Vite build), a pinned Rust stage (`cargo build --release --locked`) and a `debian:bookworm-slim` runtime.
- [x] 2.2 Install only `ca-certificates` and `curl`; copy the `rigcraft` binary to `/usr/local/bin/rigcraft` and the bundle to `/app/web`; copy `LICENSE` and `NOTICE`.
- [x] 2.3 Create uid/gid 10001, own `/data` and `/app/web`, set the three path variables, `EXPOSE 8080`, declare a `/api/health` `HEALTHCHECK`, run as `USER rigcraft`, and enter with `["rigcraft"]`.
- [x] 2.4 Add `.dockerignore` that excludes VCS data, `target`, both `node_modules` levels, `apps/web/dist`, test artefacts, `e2e`, `docs/assets`, `.forge`, `openspec`, `.standard` and secret-like files, while keeping the manifests, `src`, `apps/web`, `LICENSE` and `NOTICE`.

## 3. DFS — Compose topology
- [x] 3.1 Add root `compose.yaml` with one `rigcraft` service, local build, `image: rigcraft:local`, the loopback published port and `RIGCRAFT_PORT` override.
- [x] 3.2 Add the `rigcraft-data:/data` named volume (the backup unit), `restart: unless-stopped`, `init: true`, `stop_grace_period: 10s`.
- [x] 3.3 Add `read_only: true`, tmpfs `/tmp`, `cap_drop: [ALL]`, `no-new-privileges:true` and a healthcheck matching the image.
- [x] 3.4 Document in the file why loopback is the default and how a deliberate change is made, and name the volume as the backup unit.

## 4. DFS — Forge contract
- [x] 4.1 Replace `deployment.artifact: Dockerfile` / `kind: local` with `artifact: compose.yaml`, one `local` target of kind `docker-compose`, and an `http` health probe for `http://127.0.0.1:8080/api/health` at 30 s; leave `release:` unchanged.
- [x] 4.2 Set `"compose_file": "compose.yaml"` in `.project.json` and keep `"deployable": false`.
- [x] 4.3 Run `forge doctor .` and `forge deploy plan .`; record the target, kind, artifact and the exact remaining blocker instead of inventing a ready plan.

## 5. BFS — Gate
- [x] 5.1 Add `scripts/check-compose.mjs`: require `docker compose`, run `config --quiet`, assert the resolved JSON (service, build, healthcheck, `/data` volume, loopback port, `read_only`), and accept `--file`.
- [x] 5.2 Add `scripts/check-compose.test.mjs` proving a valid fixture passes and malformed fixtures fail through the real script.
- [x] 5.3 Wire both into `npm run check`; leave `.standard/**` and `verify-local.sh` untouched.

## 6. BFS — Runtime evidence
- [x] 6.1 `docker compose -p rigcraft-evidence up -d --build`; wait for healthy and prove `GET /api/health` 200, `/inventory` 200 and the UI loads.
- [x] 6.2 Import the shipped sample through the API into `/data`, `docker compose restart`, and prove the records survive.
- [x] 6.3 Prove uid 10001, no `cargo` in the image, the loopback publish (`docker port` or `ss -ltnp`), and record the image size from `docker image inspect`.
- [x] 6.4 `docker compose -p rigcraft-evidence down -v` (the evidence project only, never the user's volume) and capture `/inventory` at 1440 into `docs/assets/compose-*.png`.

## 7. BFS — Documentation
- [x] 7.1 Write `docs/deployment.md`: build/run/stop, environment variables, the volume as the backup unit, the loopback default and its reason, non-root/read-only posture, image contents, upgrade/rollback, and the non-goals.
- [x] 7.2 Update `README.md`, `AGENTS.md` Project facts/layout, and `docs/gap-closure.md`/`ROADMAP.md` only where a line is now stale.
- [x] 7.3 Write `docs/evidence/compose-packaging.md` with every command, number and verdict.

## 8. Verification and close-out
- [x] 8.1 `cargo fmt --all -- --check`; `cargo clippy --all-targets --all-features -- -D warnings`; `cargo test`; `npm run build`; `npm run check`; `npm test`; `npm run test:e2e`; `./verify-local.sh`.
- [x] 8.2 `node scripts/check-openspec-change-names.mjs`; `openspec validate compose-packaging --strict`; `openspec validate --all --strict`.
- [x] 8.3 Run the Forge full Gate and record its aggregate verdict and labelled revision.
- [x] 8.4 Archive with `openspec archive compose-packaging --yes` (no `--skip-specs`), commit everything except `HANDOFF.md`, then commit `HANDOFF.md` only; do not push.

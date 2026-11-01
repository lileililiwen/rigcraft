# Proposal: Package the service as a reproducible Compose container

## Why

The repository ships a `Dockerfile` that does not work. It builds
`./target/release/app`, but the crate's binary is `rigcraft`, so `docker build`
produces an image whose `CMD` names a file that does not exist. The build never
produces the Web bundle either, so even a corrected path would serve no UI:
`RIGCRAFT_WEB_DIR` defaults to `apps/web/dist`, which the image does not
contain. The base image is the unpinned `rust:1.78`, and the runtime stage is the
compiler image itself.

There is also no Compose file. Forge already defines the contract for one:
`COMPOSE_FILES = ["docker-compose.yml", "compose.yaml"]` in precedence order, a
Compose file selects target kind `docker-compose` and the file itself as
`deployment.artifact`, and `deployment.health` accepts an HTTP probe with a URL
and interval. `forge doctor` and `forge deploy plan` therefore still resolve the
artifact to `Dockerfile`, and nothing states how the service is meant to run.

The service has no authentication and holds private inventory. A container that
publishes it on every interface, or that runs as root with a writable root
filesystem, turns "run the demo" into a data-exposure decision. Packaging has to
make the safe posture the default, not the operator's memory.

## What Changes

- **A working multi-stage image.** A Node stage builds `apps/web` from the root
  workspace lockfile. A pinned Rust stage builds the `rigcraft` binary with
  `--release --locked`. A `debian:bookworm-slim` runtime stage receives only the
  binary and the built bundle, plus `LICENSE`/`NOTICE`, `ca-certificates` and
  `curl`. It runs as uid 10001, exposes 8080, and carries a `HEALTHCHECK`
  against `/api/health`. No compiler or package manager reaches the runtime.
- **A root `compose.yaml`.** One `rigcraft` service builds from the repository,
  publishes `127.0.0.1:${RIGCRAFT_PORT:-8080}:8080` so the unauthenticated
  service is not reachable off the host by default, keeps state in the named
  volume `rigcraft-data` at `/data`, restarts unless stopped, reaps children
  (`init: true`), waits up to 10 s on stop, and runs read-only with `/tmp` as
  tmpfs, every capability dropped and `no-new-privileges`. A matching
  healthcheck is declared; Compose may override it.
- **A checked build context.** `.dockerignore` excludes VCS data, build
  outputs, test artefacts, documentation captures, pack/vendor directories and
  secret-like files, and keeps the manifests, `src`, `apps/web`, `LICENSE` and
  `NOTICE`.
- **The Forge contract.** `forge.yaml` declares `deployment.artifact:
  compose.yaml`, one `local` target of kind `docker-compose`, and an HTTP health
  probe against `http://127.0.0.1:8080/api/health` every 30 seconds. `release:`
  is unchanged. `.project.json` records `compose_file: compose.yaml` and keeps
  `deployable: false`: packaging is not a release decision.
- **A repo-owned gate.** `scripts/check-compose.mjs` requires Docker and its
  Compose plugin, runs `docker compose config --quiet`, then asserts the resolved
  JSON: one service, a local build, a healthcheck, the `/data` volume, the
  loopback published port and `read_only: true`. It accepts `--file` for a
  fixture and is wired into `npm run check`; `scripts/check-compose.test.mjs`
  proves a valid fixture passes and malformed fixtures fail.

## Impact

- Affected specs: a new capability, `service-packaging`, because none of the
  existing capabilities owns how the built artifact is packaged and run.
  `inventory-import` owns the HTTP delivery surface (routes, import, headers,
  cache) and is not modified.
- Affected files: `Dockerfile`, `compose.yaml`, `.dockerignore`, `forge.yaml`,
  `.project.json`, `package.json` (the `check` script), `scripts/check-compose.mjs`,
  `scripts/check-compose.test.mjs`, `docs/deployment.md`, `README.md`,
  `AGENTS.md` Project facts, `docs/evidence/compose-packaging.md` and the
  `HANDOFF.md` pointer.
- Affected behaviour: none at runtime. No Rust or Web source, route, API, schema,
  engine value, colour, layout or copy string changes. The only operator-visible
  surface is the packaging itself.
- Non-goals: no TLS, no reverse proxy, no public exposure, no Kubernetes, no
  registry publication, no image signing, no `deployable: true`, no CI push step.

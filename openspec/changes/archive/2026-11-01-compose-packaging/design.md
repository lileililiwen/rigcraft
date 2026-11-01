# Design: Package the service as a reproducible Compose container

## Context

The current `Dockerfile` is five lines: `FROM rust:1.78 AS build`, copy
everything, `cargo build --release`, `CMD ["./target/release/app"]`. The crate
name in `Cargo.toml` is `rigcraft`, so the build produces
`target/release/rigcraft`; the `CMD` path is wrong. The file also never builds
`apps/web`, and `Config::from_env` defaults `RIGCRAFT_WEB_DIR` to
`apps/web/dist`, which the image does not have. The base tag floats within a
release line and the runtime is the compiler image.

`src/lib.rs` reads exactly three environment variables: `RIGCRAFT_ADDR`
(default `127.0.0.1:8080`), `RIGCRAFT_DATA_DIR` (default XDG data home) and
`RIGCRAFT_WEB_DIR` (default `apps/web/dist`). `src/api.rs` serves
`GET /api/health` as `{"status":"ok","service":"rigcraft-inventory"}`. Those are
the only knobs packaging needs.

Forge's Compose contract is `COMPOSE_FILES = ["docker-compose.yml",
"compose.yaml"]`; the first present file selects target kind `docker-compose`
and becomes `deployment.artifact`. `DeploymentHealthMeta` accepts
`kind: http`, `url` and `interval_seconds`.

## Decision 1 — a new `service-packaging` capability

`inventory-import` owns the running HTTP surface: routes, CSV import, provenance,
cache policy, compression and security headers. Packaging owns a different
lifetime and a different owner: the image, the Compose topology, the Forge
manifest declaration and the gate that validates them. A header change must not
also own a Rust base-image bump, and a base-image bump must not read as a change
to the import contract. The new capability states the packaging contract, and
`inventory-import` is left byte-identical.

## Decision 2 — two build stages, one runtime stage, pinned tags

The Web stage is `node:24-bookworm-slim`; the service stage is
`rust:1.99-bookworm`, the pinned line this toolchain checks in at
(`rustc 1.99.1`). Both tags were resolved with `docker manifest inspect` before
use, so the build does not float on `latest`. The runtime is
`debian:bookworm-slim`.

`npm ci` runs from the root workspace lockfile (`package.json`,
`package-lock.json`, `apps/web/package.json`), then `apps/web` is copied and
built with Vite. `cargo build --release --locked` refuses a `Cargo.lock` that
does not match `Cargo.toml`. The runtime copies only
`/build/target/release/rigcraft`, `/build/apps/web/dist`, `LICENSE` and `NOTICE`.

`ca-certificates` is installed because BYOK provider URLs are validated and
called over HTTPS. `curl` is installed because the healthcheck needs an HTTP
client. Nothing else is added, so the runtime image contains no compiler, Node
or npm installation, or source tree (the Debian base keeps `apt-get`/`dpkg`,
but no build toolchain).

## Decision 3 — non-root, read-only, one writable path

The image creates group and user `rigcraft` at uid/gid 10001, owns `/data` and
`/app/web`, sets `USER rigcraft`, and Compose runs the container with
`read_only: true`, `/tmp` as tmpfs, `cap_drop: [ALL]` and
`no-new-privileges:true`. The only writable path the service needs is `/data`,
where `Store::open` creates `inventory.sqlite3`; SQLite temporary files land on
the tmpfs. A named volume inherits the image's `/data` ownership on first use,
so the non-root user can write without a privileged entrypoint or a chown step.

## Decision 4 — loopback is the only published default

The service has no authentication. `127.0.0.1:${RIGCRAFT_PORT:-8080}:8080`
makes the default deployment reachable only from the host, which is the same
boundary as running the binary directly. `RIGCRAFT_PORT` moves the host port
without changing the container port. Reaching the service from another machine
is a deliberate edit (`"8080:8080"`) and belongs behind an authenticated proxy;
`docs/deployment.md` says so next to the port line, and `compose.yaml` repeats
it as a comment.

## Decision 5 — the Forge manifest declares Compose, the release stays closed

`forge.yaml` declares `artifact: compose.yaml`, `targets: [{name: local, kind:
docker-compose}]` and an `http` health probe. This is the contract Forge's deploy
adapter reads, so `forge deploy plan` resolves the artifact and kind from the
manifest rather than from a file-name heuristic. `release:` is untouched.
`.project.json` gains `compose_file: compose.yaml` and keeps `deployable:
false`: having a runnable package is not the same decision as releasing it, and
that decision is recorded separately.

Forge's doctor deployment probe is a sibling-owned file-name scan over
`Dockerfile`/`Containerfile` and hosted-provider files; it does not read the
manifest's `deployment.artifact`. The repository can declare Compose but cannot
change the probe without editing a sibling repository. The evidence records what
the installed Forge actually prints instead of claiming a label it does not
produce.

## Decision 6 — a check that runs the runtime

A YAML linter would accept a Compose file that never starts. `check-compose.mjs`
therefore requires `docker compose`, runs `config --quiet` (parse and
interpolation), then reads `config --format json` and asserts the resolved
topology: exactly one service named `rigcraft`, a local `build` with
`Dockerfile`, a healthcheck whose test names `/api/health`, a `volume`-type mount
at `/data`, every published port on `127.0.0.1` targeting 8080, and
`read_only: true`.

The runtime is a declared prerequisite, not an opt-out: without Docker the check
fails and names Docker Engine with the Compose v2 plugin. `--file` points the
same assertions at a fixture. `check-compose.test.mjs` builds fixtures in
`fs.mkdtemp` and drives the real script, so "valid passes, malformed fails" is
proved through the same `docker compose config` path as the repository file.
The check is wired into `npm run check`, so a topology regression fails the
declared gate; `.standard/**` and `verify-local.sh` stay untouched.

## Decision 7 — evidence is a running container, not a green build

The change is verified by building the image and running the Compose project
under a scratch project name (`rigcraft-evidence`), probing `/api/health`,
`/inventory` and the UI over the published loopback port, importing the shipped
sample into `/data`, restarting the service and proving the records survive,
then reading `id -u`, `command -v cargo`, `docker port`, the image size and
`docker compose down -v`. A build that succeeds is not evidence that the
container serves the UI or persists data; the commands and their real output go
into `docs/evidence/compose-packaging.md`.

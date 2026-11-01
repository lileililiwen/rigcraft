# service-packaging deltas

## ADDED Requirements

### Requirement: The repository builds a reproducible service image
The repository SHALL build the service image from a multi-stage `Dockerfile`. A Web
stage SHALL install from the root workspace lockfile with `npm ci` and build
`apps/web` with Vite. A service stage SHALL use a pinned Rust release line and build
with `cargo build --release --locked`. The runtime stage SHALL be
`debian:bookworm-slim` and SHALL receive only the `rigcraft` binary, the built Web
bundle, `LICENSE`, `NOTICE`, `ca-certificates` and `curl`; it SHALL contain no
compiler, Node/npm installation or source tree. The image SHALL create a non-root user with
uid 10001, own `/data` and `/app/web` by that user, set
`RIGCRAFT_ADDR=0.0.0.0:8080`, `RIGCRAFT_DATA_DIR=/data` and
`RIGCRAFT_WEB_DIR=/app/web`, expose port 8080, declare a `HEALTHCHECK` against
`http://127.0.0.1:8080/api/health`, run as that user and enter with `["rigcraft"]`.

#### Scenario: The image serves the built UI
Given the image built from the repository, when the container runs with no extra configuration, then it serves the built Web bundle from `/app/web` and `GET /api/health` returns 200.

#### Scenario: The runtime has no toolchain
Given the running container, when `command -v cargo` or `command -v node` is executed, then the command fails because no build toolchain is installed.

#### Scenario: The process is not root
Given the running container, when `id -u` is executed, then it prints `10001`.

#### Scenario: The binary is the crate's binary
Given the image, when the container starts, then the entrypoint is the release binary named `rigcraft` and the container stays running.

### Requirement: The Compose topology runs one hardened service on loopback
The repository SHALL contain a root `compose.yaml` with one service named `rigcraft`
that builds from the repository root with `Dockerfile` and tags the result
`rigcraft:local`. It SHALL publish
`127.0.0.1:${RIGCRAFT_PORT:-8080}:8080` by default, and the file SHALL state that the
service has no authentication and that publishing on a non-loopback interface is a
deliberate change. It SHALL mount the named volume `rigcraft-data` at `/data` and
SHALL name that volume as the backup unit. It SHALL set `restart: unless-stopped`,
`init: true`, `stop_grace_period: 10s`, `read_only: true`, a `/tmp` tmpfs,
`cap_drop: [ALL]`, `security_opt: [no-new-privileges:true]`, the three path
environment variables, and a healthcheck that probes `/api/health`.

#### Scenario: The default publish is loopback only
Given `compose.yaml`, when the published ports are resolved, then every published port binds `127.0.0.1` and targets container port 8080.

#### Scenario: State lives in one named volume
Given a running Compose project, when the inventory is written, then the SQLite file is created under `/data` on the `rigcraft-data` volume, and removing and restarting the container keeps it.

#### Scenario: The container filesystem is read-only
Given `compose.yaml`, when the service is resolved, then `read_only` is true, `/tmp` is a tmpfs and every capability is dropped.

### Requirement: The Forge manifest declares the Compose artifact and HTTP health
`forge.yaml` SHALL declare `deployment.artifact: compose.yaml`, one target named
`local` of kind `docker-compose`, and an `http` health check whose URL is
`http://127.0.0.1:8080/api/health` and whose interval is 30 seconds. The `release:`
block SHALL remain unchanged. `.project.json` SHALL record
`deployment.compose_file: compose.yaml` and SHALL keep `deployment.deployable` false,
because packaging is not a release decision.

#### Scenario: The manifest names Compose
Given `forge.yaml`, when the deployment block is read, then the artifact is `compose.yaml` and the only target has kind `docker-compose`.

#### Scenario: Deployment is still disabled
Given `.project.json`, when the deployment block is read, then `compose_file` is `compose.yaml` and `deployable` is false.

### Requirement: A gate validates the Compose topology with the container runtime
The repository SHALL provide a check that requires Docker Engine with the Compose v2
plugin and fails with a message naming that prerequisite when it is unavailable. The
check SHALL run `docker compose config --quiet` and then assert the resolved
configuration contains exactly one service named `rigcraft`, a local build using
`Dockerfile`, a healthcheck whose test names `/api/health`, a volume-type mount at
`/data`, at least one published port on `127.0.0.1` targeting 8080, and
`read_only: true`. The check SHALL accept `--file <path>` so a fixture can be
validated, SHALL be wired into `npm run check`, and SHALL have a test that proves a
valid fixture passes and a malformed fixture fails. No opt-out, exemption list or
environment variable SHALL disable it.

#### Scenario: A broken topology fails the gate
Given a Compose fixture that publishes on every interface and omits `read_only`, when the check runs against it, then it exits non-zero and names both failures.

#### Scenario: The repository file passes the gate
Given the repository, when `npm run check` runs, then `docker compose config` accepts `compose.yaml` and every asserted shape is present.

#### Scenario: A missing runtime is a named failure
Given a machine without Docker Compose, when the check runs, then it exits non-zero and states that Docker Engine with the Compose v2 plugin is required.

# Deployment: one Compose container

RigCraft ships as one container image and one Compose service. The image serves
the API and the built Web bundle; the Compose project keeps the SQLite inventory
in a named volume. There is no registry image, no orchestrator and no TLS
terminator in this repository.

`compose.yaml` is the Forge deployment artifact (`forge.yaml`,
`.project.json`). `Dockerfile` is the build it consumes.

## Build and run

Prerequisites: Docker Engine with the Compose v2 plugin.

```sh
docker compose up -d --build      # build the image and start the service
docker compose ps                 # one service, health status
docker compose logs -f rigcraft   # follow the service log
```

Then open `http://127.0.0.1:8080/inventory`. The first start creates
`inventory.sqlite3` on the volume; there is no seeded inventory.

```sh
docker compose stop               # stop, keep the container and volume
docker compose restart            # restart in place
docker compose down               # remove the container and network, keep the volume
docker compose down -v            # remove the volume too: this deletes the inventory
```

Compose's `init: true` makes the service reap orphaned children. `stop_grace_period: 10s` gives the process time to close SQLite before it is killed.

## Environment

| Variable | Container value | Meaning |
| --- | --- | --- |
| `RIGCRAFT_ADDR` | `0.0.0.0:8080` | Bind inside the container; loopback is enforced by the published port instead |
| `RIGCRAFT_DATA_DIR` | `/data` | Directory holding `inventory.sqlite3` and pre-restore copies |
| `RIGCRAFT_WEB_DIR` | `/app/web` | Built Web bundle |
| `RIGCRAFT_PORT` | unset → `8080` | Host-side port only; used by the Compose port mapping |

`RIGCRAFT_PORT=9090 docker compose up -d` serves the same container on
`127.0.0.1:9090`. The container port stays 8080.

## The volume is the backup unit

`rigcraft-data` is mounted at `/data` and is the only writable path. It holds
`inventory.sqlite3` and any `inventory.sqlite3.pre-restore-<UTC>` safety copy.
Back up the volume, not the container: a container is disposable, the volume is
the data.

```sh
# Backup the whole data directory to the current directory.
docker compose exec -T rigcraft tar czf - -C /data . > "rigcraft-data-$(date -u +%Y%m%dT%H%M%SZ).tgz"

# Restore it (stop the service first, then write the same volume).
docker compose stop
docker compose run --rm --entrypoint sh rigcraft -c 'tar xzf - -C /data' < rigcraft-data-<timestamp>.tgz
docker compose up -d
```

The application also exposes an inspect → export → restore flow at
`/inventory`; that flow is documented in
[`docs/data-portability.md`](data-portability.md). The volume archive is the
disaster-recovery unit, because it includes the pre-restore copies as well.

## Loopback only by default

`compose.yaml` publishes `127.0.0.1:${RIGCRAFT_PORT:-8080}:8080`. The service has
no authentication: any client that can reach the port can read and replace the
private inventory. Binding the published port to `127.0.0.1` keeps the same
boundary as running the binary directly — only the host reaches it.

Reaching the service from another machine is a deliberate change, not a default:

```yaml
# compose.yaml
ports:
  - "8080:8080"   # now every interface can reach an unauthenticated service
```

Only do that behind an authenticated reverse proxy or on a network you control,
and read [`docs/delivery-headers.md`](delivery-headers.md) first: the service's
own header policy is not an authentication layer.

## Posture inside the container

- Runs as uid/gid `10001` (`rigcraft`), never root.
- `read_only: true`: the root filesystem cannot be written. `/data` is the only
  writable mount and `/tmp` is a tmpfs.
- `cap_drop: [ALL]` and `no-new-privileges:true`: no Linux capability and no
  privilege gain.
- The container listens on `0.0.0.0:8080` internally so the published port can
  forward to it; the host side is loopback only.

## Image contents

- `/usr/local/bin/rigcraft` — the release binary built with `cargo build
  --release --locked`.
- `/app/web` — the Vite build of `apps/web`.
- `/app/LICENSE` and `/app/NOTICE`.
- `ca-certificates` (HTTPS BYOK provider validation) and `curl` (the
  healthcheck). No compiler, Node, npm or source tree is installed.
- `HEALTHCHECK` probes `http://127.0.0.1:8080/api/health` every 30 s; the
  Compose service declares the same probe.

## Upgrade and rollback

```sh
git pull
docker compose up -d --build     # rebuild from the checked-out revision
docker compose ps                # wait for healthy
```

The named volume is untouched by a rebuild. To roll back, check out the previous
revision, `docker compose up -d --build`, and confirm the health status. The
SQLite schema is versioned by the service; a downgrade across an incompatible
schema is not supported by this packaging — restore the volume archive taken
before the upgrade instead. No image is published to a registry, so a rollback
is a rebuild from a known revision, not an image tag.

## Non-goals

- **No TLS.** The service speaks plain HTTP on loopback. TLS belongs to a
  reverse proxy or a platform ingress, not to this image.
- **No reverse proxy.** Nothing authenticates, rate-limits or terminates TLS.
- **No public exposure.** The default publish is loopback only.
- **No release.** `.project.json` keeps `deployment.deployable: false`.
  Packaging makes the service runnable; enabling deployment is a separate
  release decision with its own evidence.

# Compose packaging evidence

Date: 2026-11-01. Docker Engine 29.8.2 (API 1.52, containerd, overlayfs on
Ubuntu 26.04.1 LTS) with Docker Compose v5.6.0, `rustc 1.99.1`, Node 24 on the
host, Chrome 154 for the capture. The Compose project was run under the
throwaway name `rigcraft-evidence`, never the operator's project.

Nothing below is estimated: every command and every number was read from the
build or the running container.

## Base images are pinned and resolvable

```text
$ docker manifest inspect rust:1.99-bookworm     -> OK
$ docker manifest inspect node:24-bookworm-slim  -> OK
$ docker manifest inspect debian:bookworm-slim   -> OK
```

The Rust line matches the checked-out toolchain (`rustc 1.99.1`). The build does
not use `latest` and does not float within a line.

## Build and start

```text
$ docker compose -p rigcraft-evidence up -d --build
...
#24 182.8     Finished `release` profile [optimized] target(s) in 3m 02s
#28 naming to docker.io/library/rigcraft:local done
Error response from daemon: failed to set up container networking: driver failed programming external connectivity
on endpoint rigcraft-evidence-rigcraft-1 (...): failed to bind host port 127.0.0.1:8080/tcp: address already in use
```

The image built. The first start failed because host port 8080 was already
occupied by another local process, which is exactly what the documented
`RIGCRAFT_PORT` override is for. The project was started with that override; the
container port stayed 8080:

```text
$ RIGCRAFT_PORT=18080 docker compose -p rigcraft-evidence up -d
 Container rigcraft-evidence-rigcraft-1 Started
$ docker compose -p rigcraft-evidence ps
NAME                           IMAGE            COMMAND      SERVICE    STATUS                   PORTS
rigcraft-evidence-rigcraft-1   rigcraft:local   "rigcraft"   rigcraft   Up (healthy)             127.0.0.1:18080->8080/tcp
```

`healthy` is the image's own `HEALTHCHECK` reaching `/api/health`.

## The container serves the API and the built UI

```text
GET /api/health   200 application/json  46 bytes  {"service":"rigcraft-inventory","status":"ok"}
GET /              200 text/html       664 bytes
GET /inventory     200 text/html       664 bytes
GET /catalog       200 text/html       664 bytes
GET /builds/check  200 text/html       664 bytes
GET /recommend     200 text/html       664 bytes
GET /diagnose      200 text/html       664 bytes
```

Response headers are the same policy the binary serves outside a container:
`x-content-type-options: nosniff`, `referrer-policy: no-referrer`,
`x-frame-options: DENY`, the CSP and, for `/api/health`, `cache-control:
no-store`. The HTML is the shell; the built bundle is present in the image:

```text
$ docker run --rm --entrypoint sh rigcraft:local -c 'ls /app; ls -l /usr/local/bin/rigcraft'
LICENSE
NOTICE
web
-rwxr-xr-x 1 root root 13954200 ... /usr/local/bin/rigcraft
```

The page in Chrome reported:

```json
{"title":"RigCraft inventory","innerWidth":1440,"innerHeight":900,
 "scrollWidth":1440,"clientWidth":1440,
 "status":"Opened the newest snapshot, revision 1: 10 records, US$2,120.90 stock value. Export and backup are ready below.",
 "rows":10}
```

Capture: `docs/assets/compose-inventory-desktop.png`, 1440×900, PNG 8-bit RGB,
157,337 bytes, SHA-256 `f622d91285d1e6916a44d9f36f01968a227f3458d42ca5394fb70b892de67116`.

## Import through the API, restart, and the records survive

The shipped sample was downloaded from the container's own
`/api/imports/sample`, previewed as multipart `file` + `currency=USD`, and
committed with every row accepted:

```text
preview 200 {"total":10,"valid":10,"errors":0,"duplicateRows":0} ea3627d7-c180-4210-8e2d-ec8fe80ab853
commit  200 028191e2-0f9b-4a58-b14a-a39fc5686493 revision 1 records 10
```

The service wrote into the named volume:

```text
$ docker compose -p rigcraft-evidence exec -T rigcraft ls -l /data
-rw-r--r-- 1 rigcraft rigcraft   4096 ... inventory.sqlite3
-rw-r--r-- 1 rigcraft rigcraft  32768 ... inventory.sqlite3-shm
-rw-r--r-- 1 rigcraft rigcraft 333752 ... inventory.sqlite3-wal
```

Then `docker compose restart` (the output read `Container ... Restarting` /
`Started`), `/api/health` answered 200 again, and the same snapshot was still
the newest:

```text
[{"id":"028191e2-0f9b-4a58-b14a-a39fc5686493","createdAt":"2026-11-01T09:27:39.304611421Z",
  "revision":1,"parentSnapshotId":null,"currency":"USD","sourceFilename":"sample.csv",
  "recordCount":10,"unitCount":10,"valueMinor":212090,"note":null}]
GET /api/inventory/snapshots/028191e2-... -> 200
```

`RestartCount` stayed 0 because `docker compose restart` is an operator
restart, not a failure restart; the restart itself is the `Restarting`/`Started`
line above. The volume, not the container, is the unit that carried the data.

## Non-root, no toolchain, read-only

```text
$ docker compose -p rigcraft-evidence exec -T rigcraft id -u
10001
$ docker compose -p rigcraft-evidence exec -T rigcraft id -g
10001
$ docker compose -p rigcraft-evidence exec -T rigcraft id
uid=10001(rigcraft) gid=10001(rigcraft) groups=10001(rigcraft)
$ docker compose -p rigcraft-evidence exec -T rigcraft cat /proc/1/status | grep ^Uid
Uid: 10001 10001 10001 10001
$ docker compose -p rigcraft-evidence exec -T rigcraft cat /proc/1/status | grep ^Name
Name: docker-init
```

`/proc/1` is `docker-init` because Compose runs `init: true`; the service runs
under it as uid 10001.

```text
$ docker run --rm --entrypoint sh rigcraft:local -c 'for c in gcc cc make python3 cargo rustc node npm; do command -v $c; done'
(no output: every one of those is absent)
```

The Debian base keeps `apt-get`/`dpkg`; no build toolchain, Node or npm is
installed. The runtime carries `curl 7.88.1` and 297 CA certificate files
(`ca-certificates`).

```text
$ docker compose -p rigcraft-evidence exec -T rigcraft sh -c 'touch /root-write'
touch: cannot touch '/root-write': Read-only file system
$ docker compose -p rigcraft-evidence exec -T rigcraft sh -c 'touch /tmp/probe && echo tmpfs-writable'
tmpfs-writable
$ docker inspect rigcraft-evidence-rigcraft-1 --format 'ReadonlyRootfs={{.HostConfig.ReadonlyRootfs}} CapDrop={{.HostConfig.CapDrop}} SecurityOpt={{.HostConfig.SecurityOpt}} User={{.Config.User}} Init={{.HostConfig.Init}} Restart={{.HostConfig.RestartPolicy.Name}}'
ReadonlyRootfs=true CapDrop=[ALL] SecurityOpt=[no-new-privileges:true] User=rigcraft Init=true Restart=unless-stopped
```

`/data` and `/app/web` are owned by `10001:10001`; the single mount is the named
volume:

```text
$ docker inspect rigcraft-evidence-rigcraft-1 --format '{{range .Mounts}}{{.Type}} {{.Name}} -> {{.Destination}} rw={{.RW}}{{end}}'
volume rigcraft-evidence_rigcraft-data -> /data rw=true
```

## Loopback publish and image size

```text
$ docker port rigcraft-evidence-rigcraft-1
8080/tcp -> 127.0.0.1:18080
$ ss -ltnp | grep 18080
LISTEN 0 4096 127.0.0.1:18080 0.0.0.0:*
$ docker image inspect rigcraft:local --format '{{.Id}} {{.Size}} bytes {{len .RootFS.Layers}} layers'
sha256:d4e21f7a53c81825c6f48aa3d9a86f8fdf43ec39321015397ae77670be19c952 154390850 bytes 6 layers
```

154,390,850 bytes is 147.2 MiB, six layers including the binary (13,954,200
bytes) and the Web bundle. The image was rebuilt once after a comment-only edit
to the `Dockerfile`; the instruction set is identical, the size and layer count
did not change, and the final tag was started again and answered `/api/health`
200 as uid 10001.

The published address is `127.0.0.1`, not `0.0.0.0`.

## Teardown

```text
$ docker compose -p rigcraft-evidence down -v
 Container ... Stopped
 Container ... Removed
 Volume rigcraft-evidence_rigcraft-data Removed
 Network rigcraft-evidence_default Removed
$ docker volume ls | grep rigcraft-evidence
(no match)
```

Only the evidence project's volume was removed. The operator's own volume was
never named by any command in this record.

## Gate output

```text
$ node scripts/check-compose.mjs
PASS /home/paul/code/rigcraft/compose.yaml: one service, local build, /api/health healthcheck, rigcraft-data at /data, 127.0.0.1:8080->8080, read_only
$ node --test scripts/check-compose.test.mjs
✔ a fixture matching the contract passes
✔ a fixture that binds every interface and drops read_only fails
✔ a fixture without a build or healthcheck fails
ℹ tests 3 / pass 3 / fail 0
$ docker compose -f compose.yaml config --quiet
(exit 0)
```

`npm run check` reported the three compose tests inside its 21 `node --test`
cases and the `PASS ... compose.yaml` line after the bundle budget; TypeScript
followed with no error.

## Forge verdicts

```text
$ /home/paul/code/forge/target/debug/forge doctor .
[PASS] deployment-config (manual)
  evidence: container (Dockerfile)
  detail: the project does not require a deployment configuration at its target maturity
[N/A] L2-deployment (L2): deployment automation is configured
  evidence: container (Dockerfile)
```

The installed Forge (`forge 0.1.1`, source `cbf6a9a`) detects deployment from a
file-name scan over `Dockerfile`/`Containerfile` and hosted-provider files; its
probe does not read `deployment.artifact`. The label therefore still says
`container (Dockerfile)` even though `forge.yaml` now declares Compose. Changing
that probe would mean editing a sibling repository, which this change is not
authorised to do. The manifest is the contract Forge's deploy adapter reads, and
that is what changed:

```text
$ /home/paul/code/forge/target/debug/forge deploy plan .
project: rigcraft
deploy_id: rigcraft-local-2601d354147c
target: local (docker-compose)
source_revision: f5b651c916587ca8f36c223958e7ee64ed76a7fa
artifact: compose.yaml
artifact_hash: 499aa302dc811f036c7eda38d51d188aec32290d2f01296a7b06690156ea14d1
health: http (url=http://127.0.0.1:8080/api/health)
ready: yes
summary: deploy plan ready for target `local` (kind `docker-compose`) at revision `f5b651c91658`
```

The plan's `source_revision` is the commit `forge.yaml` was edited on top of;
`artifact_hash` is the SHA-256 of `compose.yaml` at plan time. `ready: yes` is
Forge's own verdict — no blocker was invented. `forge deploy apply` was not run:
deployment is still disabled by the repository's release decision
(`.project.json` `deployable: false`), and this change does not make that
decision.

## Not claimed

- No image is published to a registry, signed or scanned.
- No TLS, reverse proxy, public exposure or orchestrator is configured.
- The capture proves the container serves the UI; it is not a new visual
  baseline and no pixel baseline was regenerated.
- No forced-colours or mobile container capture.

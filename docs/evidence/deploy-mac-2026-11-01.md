# Mac deployment evidence — 2026-11-01

## What was deployed

| Item | Value |
| --- | --- |
| Project | `rigcraft` |
| Revision | `2ce7528d42ae` (the ship built the revision it gated) |
| Image | `forge-rigcraft:2ce7528d42ae` |
| Image identity | `sha256:c0fd5bb227b9feb09f4b5f927367d8192256385b4ceba361809093cd9447a926` |
| Container | `rigcraft` — `Up (healthy)`, `127.0.0.1:13720->8080/tcp` |
| Route | `http://rigcraft.tooosall.uk → host.docker.internal:13720` |
| Caddyfile backup | `/Users/allen/production/platform/Caddyfile.pre-rigcraft-20261101223945` |
| Release manifest | `.forge/release/rigcraft/2ce7528d42ae.json` (generated, ignored) |
| Env file | `/Users/allen/production/secrets/rigcraft/.env` (created empty, `0600`; contents never read) |

Tooling used, all from this session:

- Forge `forge ship` (`mac-ship-facade`, commits `a9d23bd`, `11aae72`).
- Forge Dockerfile image builder (`dockerfile-image-builder`, commits `17e920a`, `333022c`).
- `jenkins-local` adapter load transport (`mac-runtime-image-load`, commits `46bd915`, `acea09b`) and health-argument conformance (`mac-runtime-executor-contract`, commits `6015a18`, `ee934d2`).
- Release decision: this repository's `.project.json` `deployment.deployable: true`, commit `931e8a9`.

## Verified

Through the Mac's production router, with the public `Host` header:

```
/            -> 200
/api/health  -> 200
/inventory   -> 200
```

Directly against the container: `GET http://127.0.0.1:13720/api/health` → `200`.

The container is healthy five minutes after start; the image identity on the Mac
equals the provenance digest; the route is present in the deployed Caddyfile; the
previous Caddyfile is backed up.

## Blocked: the public edge

`https://rigcraft.tooosall.uk` returns **Cloudflare Error 1033** (Argo Tunnel
error). The body is exactly `error code: 1033`. This is not specific to RigCraft:
`https://hypora.tooosall.uk` returns the same `530`, and local UDP DNS resolves
every `*.tooosall.uk` to the benchmark address `198.18.1.91` (fake-IP
interception). Public DNS over HTTPS returns the real Cloudflare edge
`172.67.146.218` / `104.21.79.181`; pinning those addresses with `--resolve`
still returns `530`, so the tunnel origin is down rather than the local resolver.

The Mac's `cloudflared` process has been running since 2026-09-26, but the tunnel
is not connected. Restoring it needs either root on the Mac (a system LaunchDaemon
restart requires `sudo`, and no passwordless sudo is available) or the Cloudflare
dashboard. That is an operator action, not a deployment defect.

The ship reports this honestly: every delivery step is `delivered`, the
observation is `failed` with `http_status: 530`, the overall verdict is `failed`
and the exit code is 1, with the note “the delivery completed but the public URL
observation failed”.

## What was not verified

- A public HTTPS request through the Cloudflare edge (blocked by 1033).
- A TLS certificate observation for the new hostname (the edge did not answer).
- `forge ship rollback` for this release: the ship verdict was `failed` at the
  public observation, so no delivered ledger was recorded; the operator stop path
  is `ssh mac /usr/local/bin/docker stop rigcraft` plus the Caddyfile backup above.

## Reproduce

```sh
cd ~/code/rigcraft
export FORGE_IMAGE_BUILDER_BIN=~/code/forge/adapters/image/forge-image-builder-docker
export FORGE_DEPLOYER_MAC_RUNTIME_BIN=~/code/jenkins-local/adapters/mac-runtime/forge_deployer_mac_runtime.py
~/code/forge/target/debug/forge ship . --confirm
```

A second run of the same revision reuses the provenance record and the loaded
image, reconciles the container, and reports the same URL and port.

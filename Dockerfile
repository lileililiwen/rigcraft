# syntax=docker/dockerfile:1
#
# The service and the Web bundle are built in separate stages, and the runtime
# stage receives only their outputs. No compiler, Node/npm installation or
# source tree reaches the image that runs.

# ---- Web bundle -----------------------------------------------------------
FROM node:24-bookworm-slim AS web
WORKDIR /build

# The root lockfile is the workspace lockfile; copying the manifests first
# keeps `npm ci` cached until a dependency actually changes.
COPY package.json package-lock.json ./
COPY apps/web/package.json apps/web/package.json
RUN npm ci

COPY apps/web apps/web
RUN npm --workspace @rigcraft/web run build

# ---- Service --------------------------------------------------------------
FROM rust:1.99-bookworm AS service
WORKDIR /build

COPY Cargo.toml Cargo.lock ./
COPY src src
RUN cargo build --release --locked

# ---- Runtime --------------------------------------------------------------
FROM debian:bookworm-slim

# curl is the healthcheck client; ca-certificates is what an operator's
# HTTPS BYOK provider is validated against. Nothing else is installed.
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

# A fixed non-root identity. `/data` is the only writable path the service
# needs; Compose mounts the named volume there.
RUN groupadd --gid 10001 rigcraft \
    && useradd --uid 10001 --gid 10001 --no-create-home --shell /usr/sbin/nologin rigcraft \
    && install -d -o 10001 -g 10001 /data /app/web

COPY --from=service /build/target/release/rigcraft /usr/local/bin/rigcraft
COPY --from=web /build/apps/web/dist /app/web
COPY LICENSE NOTICE /app/

ENV RIGCRAFT_ADDR=0.0.0.0:8080 \
    RIGCRAFT_DATA_DIR=/data \
    RIGCRAFT_WEB_DIR=/app/web

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD ["curl", "--fail", "--silent", "--show-error", "--output", "/dev/null", "http://127.0.0.1:8080/api/health"]

USER rigcraft
ENTRYPOINT ["rigcraft"]

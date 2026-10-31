# RigCraft

Open-source PC build planning from owned inventory, with traceable compatibility checks. See [requirements](requirement.md) and [roadmap](ROADMAP.md).

## Status

This is a Forge Rust service scaffold and governance bootstrap. Inventory import, compatibility checking, recommendations and Web UI are planned, not delivered.

Forge deterministic scaffold for profile `rust-web` (assets 0.1.0).
Ordinary source: builds and runs with its native toolchain; no Forge runtime required.

## Native commands

```sh
cargo build
cargo test
```

Axum/sqlx dependencies from the profile descriptor are resolved per-service; this scaffold ships dependency-free so `cargo build` works offline.
## Shared layer

Kit `none` (no version) on the `cargo` ecosystem.

Declared minimum consumption: 0 (a declared zero, not a met floor). Reason: no crate in rust-platform-libs has two external consumers (1 consumer: trailCrew); the 8 crates stay commented-out Cargo.toml entries

Referencing a kit package never grants the capability it implements. Infrastructure (identity, persistence, tenancy, caching, jobs, billing, mailing, storage, AI) is behind an explicit `--feature` request on profile `rust-web`.


## Forge

Created with `forge new --profile rust-web`. Rendering is verified by Forge;
native build/test evidence requires the stack toolchain (see `forge profile preflight rust-web`).

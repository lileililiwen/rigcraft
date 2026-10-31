# Proposal: Import and correct owned inventory

## Why
a versioned, private, usable inventory snapshot is required by the RigCraft requirement baseline (REQ-D01–D06).

## What Changes
Deliver CSV parsing, normalized part records, stock and price records, provenance, and the import/correction screen.

## Package Boundary and Split Assessment
Single outcome: a versioned, private, usable inventory snapshot. Depends on none. Excludes compatibility judgement, build generation, third-party catalog distribution. The interface and behavior share one acceptance journey; adjacent outcomes have separate packages in ROADMAP.md.

## Sibling and Shared Architecture Reconnaissance
| Candidate | Evidence | Compatibility and ownership | Decision |
| --- | --- | --- | --- |
| Forge Rust platform libraries | `../forge/modules/rust-platform-libs/Cargo.toml`, `forge.yaml` | Profile declares a zero-consumption kit; no verified two-consumer PC domain package. Versioned provider would require explicit adoption. | keep local |
| Forge design tokens | `apps/web/src/styles/vendor/tokens.css`, `.project.json` | Pinned artifact is already vendored; UI can consume without runtime sibling import. | adopt |
| Existing PC builder projects | References in `requirement.md` | Licenses, API stability and data rights are unverified; no source-level dependency in this package. | adapt through a generic adapter |

## User Experience and Interface Impact
Actor: shop operator or individual owner. Entry: /inventory from primary navigation. The UI covers empty, selecting, parsing, review, row correction, validation error, commit pending, committed, retry. Keep the primary action and status visible, use labelled fields, keyboard/focus order, live status announcements, non-color-only outcomes, desktop/mobile layouts and reduced motion. Verify the complete journey in a real browser and with accessibility checks.

## BFS Impact Map
Rust domain and HTTP API, local persistence, React/TypeScript Web flow, configuration, error handling, privacy, unit/API/browser tests are affected. External pricing, deployment and cross-repository source changes are unaffected. Browser evidence and local Gate are required before completion.

## Capabilities
- `inventory-import`: a versioned, private, usable inventory snapshot.

## Non-goals
compatibility judgement, build generation, third-party catalog distribution.

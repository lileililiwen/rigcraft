# Proposal: Recommend and compare inventory builds

## Why
up to three reproducible, budget- and stock-valid builds with comparison and export is required by the RigCraft requirement baseline (REQ-I01–I03, REQ-D05, REQ-R01–R05).

## What Changes
Deliver structured request form, deterministic candidate search, ranking, no-solution explanation, comparison and JSON export.

## Package Boundary and Split Assessment
Single outcome: up to three reproducible, budget- and stock-valid builds with comparison and export. Depends on evidence-backed-compatibility. Excludes live market pricing, purchase, reservation, FPS claims. The interface and behavior share one acceptance journey; adjacent outcomes have separate packages in ROADMAP.md.

## Sibling and Shared Architecture Reconnaissance
| Candidate | Evidence | Compatibility and ownership | Decision |
| --- | --- | --- | --- |
| Forge Rust platform libraries | `../forge/modules/rust-platform-libs/Cargo.toml`, `forge.yaml` | Profile declares a zero-consumption kit; no verified two-consumer PC domain package. Versioned provider would require explicit adoption. | keep local |
| Forge design tokens | `apps/web/src/styles/vendor/tokens.css`, `.project.json` | Pinned artifact is already vendored; UI can consume without runtime sibling import. | adopt |
| Existing PC builder projects | References in `requirement.md` | Licenses, API stability and data rights are unverified; no source-level dependency in this package. | adapt through a generic adapter |

## User Experience and Interface Impact
Actor: buyer or shop operator. Entry: /recommend from primary navigation. The UI covers form empty, editing, generating, results, unknown warning, no solution, stale snapshot, export success/error. Keep the primary action and status visible, use labelled fields, keyboard/focus order, live status announcements, non-color-only outcomes, desktop/mobile layouts and reduced motion. Verify the complete journey in a real browser and with accessibility checks.

## BFS Impact Map
Rust domain and HTTP API, local persistence, React/TypeScript Web flow, configuration, error handling, privacy, unit/API/browser tests are affected. External pricing, deployment and cross-repository source changes are unaffected. Browser evidence and local Gate are required before completion.

## Capabilities
- `build-recommendation`: up to three reproducible, budget- and stock-valid builds with comparison and export.

## Non-goals
live market pricing, purchase, reservation, FPS claims.

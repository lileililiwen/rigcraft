# Proposal: Diagnose an existing or proposed PC build

## Why
a sourced diagnosis of reusable parts, conflicts and an incremental replacement path is required by the RigCraft requirement baseline (P1 upgrade and list diagnosis).

## What Changes
Deliver manual existing-part entry, pasted list matching, deterministic diagnosis and diagnostic UI.

## Package Boundary and Split Assessment
Single outcome: a sourced diagnosis of reusable parts, conflicts and an incremental replacement path. Depends on evidence-backed-compatibility. Excludes automatic local hardware detection, FPS prediction, merchant optimization. The interface and behavior share one acceptance journey; adjacent outcomes have separate packages in ROADMAP.md.

## Sibling and Shared Architecture Reconnaissance
| Candidate | Evidence | Compatibility and ownership | Decision |
| --- | --- | --- | --- |
| Forge Rust platform libraries | `../forge/modules/rust-platform-libs/Cargo.toml`, `forge.yaml` | Profile declares a zero-consumption kit; no verified two-consumer PC domain package. Versioned provider would require explicit adoption. | keep local |
| Forge design tokens | `apps/web/src/styles/vendor/tokens.css`, `.project.json` | Pinned artifact is already vendored; UI can consume without runtime sibling import. | adopt |
| Existing PC builder projects | References in `requirement.md` | Licenses, API stability and data rights are unverified; no source-level dependency in this package. | adapt through a generic adapter |

## User Experience and Interface Impact
Actor: owner of a PC or pasted parts list. Entry: /diagnose from primary navigation. The UI covers empty, paste/manual entry, match review, checking, results, ambiguity, no replacement, error. Keep the primary action and status visible, use labelled fields, keyboard/focus order, live status announcements, non-color-only outcomes, desktop/mobile layouts and reduced motion. Verify the complete journey in a real browser and with accessibility checks.

## BFS Impact Map
Rust domain and HTTP API, local persistence, React/TypeScript Web flow, configuration, error handling, privacy, unit/API/browser tests are affected. External pricing, deployment and cross-repository source changes are unaffected. Browser evidence and local Gate are required before completion.

## Capabilities
- `build-diagnosis`: a sourced diagnosis of reusable parts, conflicts and an incremental replacement path.

## Non-goals
automatic local hardware detection, FPS prediction, merchant optimization.

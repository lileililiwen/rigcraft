# Proposal: Parse and revise build requests with BYOK

## Why
a confirmed structured request or local revision that is rechecked by the deterministic pipeline is required by the RigCraft requirement baseline (P1 AI boundary).

## What Changes
Deliver optional OpenAI-compatible adapter, parsing/revision UI, data disclosure and session-only key handling.

## Package Boundary and Split Assessment
Single outcome: a confirmed structured request or local revision that is rechecked by the deterministic pipeline. Depends on inventory-build-recommendation. Excludes autonomous agent loops, model compatibility verdicts, provider-specific SDKs. The interface and behavior share one acceptance journey; adjacent outcomes have separate packages in ROADMAP.md.

## Sibling and Shared Architecture Reconnaissance
| Candidate | Evidence | Compatibility and ownership | Decision |
| --- | --- | --- | --- |
| Forge Rust platform libraries | `../forge/modules/rust-platform-libs/Cargo.toml`, `forge.yaml` | Profile declares a zero-consumption kit; no verified two-consumer PC domain package. Versioned provider would require explicit adoption. | keep local |
| Forge design tokens | `apps/web/src/styles/vendor/tokens.css`, `.project.json` | Pinned artifact is already vendored; UI can consume without runtime sibling import. | adopt |
| Existing PC builder projects | References in `requirement.md` | Licenses, API stability and data rights are unverified; no source-level dependency in this package. | adapt through a generic adapter |

## User Experience and Interface Impact
Actor: user choosing natural-language assistance. Entry: /recommend assistant panel from primary navigation. The UI covers disabled, consent, key entry, parsing, proposed fields, ambiguity, provider error, confirmed. Keep the primary action and status visible, use labelled fields, keyboard/focus order, live status announcements, non-color-only outcomes, desktop/mobile layouts and reduced motion. Verify the complete journey in a real browser and with accessibility checks.

## BFS Impact Map
Rust domain and HTTP API, local persistence, React/TypeScript Web flow, configuration, error handling, privacy, unit/API/browser tests are affected. External pricing, deployment and cross-repository source changes are unaffected. Browser evidence and local Gate are required before completion.

## Capabilities
- `ai-planning`: a confirmed structured request or local revision that is rechecked by the deterministic pipeline.

## Non-goals
autonomous agent loops, model compatibility verdicts, provider-specific SDKs.

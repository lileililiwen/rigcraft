# rigcraft Roadmap

This is delivery order, not active OpenSpec changes. See `requirement.md`. Each future product change needs a usable Web flow and browser/accessibility evidence with its behavior.

## Implementation package map

| Package | Single outcome | Owner/language | Boundary | Depends on | Independent oracle |
| --- | --- | --- | --- | --- | --- |
| `inventory-import-and-provenance` | Versioned private inventory | RigCraft Rust + React | CSV, normalized records, correction UI | None | Mixed-row import browser flow and snapshot fixtures |
| `evidence-backed-compatibility` | Three-state sourced verdict | RigCraft Rust + React | Rule report and check UI | Inventory import | Conflict/unknown fixtures and browser diagnostics |
| `inventory-build-recommendation` | Budget and stock valid builds | RigCraft Rust + React | Search, compare, export UI | Compatibility | Repeatable no-AI browser journey |
| `byok-natural-language-planning` | Optional confirmed AI intent | RigCraft Rust + React | Provider adapter and assistant panel | Recommendation | Mock provider and no-key browser flows |
| `existing-build-diagnosis` | Reuse and replacement advice | RigCraft Rust + React | Match, diagnose and review UI | Compatibility | Ambiguous match and upgrade browser flows |

These packages are authored together at the user's request. Implementation remains serial: select one active package in dependency order, set `current_spec` in `HANDOFF.md`, complete and archive it, then stop.

1. v0.1 (implemented in `inventory-import-and-provenance`): provenance and local storage verified with user-supplied specs; inventory import and correction UI delivered. A licensed catalog source is still unconfirmed.
2. v0.2: normalize parts and snapshots; implement three-state compatibility checks and evidence UI.
3. v0.3: constrained recommendation, no-solution explanation, comparison and export; verify a complete browser journey without AI.
4. Later: optional BYOK input and modification, upgrade planning and external-list diagnosis.
5. Deferred: FPS, merchant margin, live retail pricing, MCP and advanced optimization.

Only a Forge scaffold and governance exist today. No release or deployment readiness is claimed.

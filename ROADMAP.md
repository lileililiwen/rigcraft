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

All five packages are implemented, archived and committed on `main`, in dependency order. A new product change starts a new OpenSpec change with its own package boundary, browser evidence and accessibility check.

1. v0.1 (implemented in `inventory-import-and-provenance`, corrected in `real-catalog-seed-and-import`): provenance and local storage verified with user-supplied specs; inventory import and correction UI delivered. The data-source position is now recorded: the shipped seed is sourced and CC BY 4.0, and BuildCores OpenDB is operator-fetched under its own licence rather than bundled.
2. v0.2 (implemented in `evidence-backed-compatibility`): normalize parts and snapshots; three-state compatibility checks and evidence UI delivered.
3. v0.3 (implemented in `inventory-build-recommendation`): constrained recommendation, no-solution explanation, comparison and export; the complete no-AI browser journey is verified.
4. P1 (implemented in `byok-natural-language-planning` and `existing-build-diagnosis`): optional BYOK input and modification, existing-build upgrade planning and pasted external-list diagnosis.
5. Deferred: FPS, merchant margin, live retail pricing, MCP and advanced optimization.

The five implementation packages above exist and are archived. The operator-supplied catalog mechanism is implemented in `user-catalog-ingestion`: a versioned document, strict validation, per-instance SQLite storage, search, matching and import enrichment, with the data and licence supplied by the operator. No release or deployment readiness is claimed: deployment remains disabled in `.project.json`, and no third-party hardware catalog, live pricing or performance evidence is bundled. The sourced sample, the operator-fetch converters and the ingestion contract are recorded in `docs/data-sources.md` and `docs/catalog-ingestion.md`.

## Design program (reference-led)

The Stitch `OpenPCBuilder` / `Kinetic Silicon` project is a visual reference, not
a source. `docs/design-program.md` records what is taken, what is declined (the
crawler/ETL and AI-by-default scope), the token/contrast guardrails that may not
break, and six delivery packages:

| # | Change | Single outcome | Depends on |
| --- | --- | --- | --- |
| 1 | `dark-brand-token-layer` | The measured dark token set and a defect-free shell | — |
| 2 | `workbench-recommendation` | `/recommend` as the workbench reference | 1 |
| 3 | `verdict-console` | `/builds/check` and `/diagnose` as a rule console | 1 |
| 4 | `dataops-console` | `/inventory` and `/catalog` as an operator console | 1 |
| 5 | `service-correctness` | API and client-route correctness, safe CSV | — |
| 6 | `delivery-hardening` | Compression, cache and security headers | — |

One change is active at a time; packages 5 and 6 are independent and may be taken
first.

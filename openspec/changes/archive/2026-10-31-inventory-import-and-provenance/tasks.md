# Tasks: Import and correct owned inventory

## 1. BFS — Baseline and impact coverage
- [x] 1.1 Map REQ-D01–D06 to actors, route, contracts, persistence, errors, fixtures and browser states; reconcile proposal/design/spec.
- [x] 1.2 Inspect Forge shared artifacts and pinned token receipt; confirm local ownership and the implementation-handoff gate.

## 2. DFS — Requirement-by-requirement implementation
- [x] 2.1 Implement domain types, validation, persistence and contract for inventory-import; add failure fixtures.
- [x] 2.2 Implement /inventory with all stated states, keyboard/focus, mobile layout and accessible feedback.
- [x] 2.3 Connect the UI to the real API and verify each spec scenario end to end.

## 3. BFS — Cross-surface regression and completeness
- [x] 3.1 Recheck callers, snapshots, error codes, privacy, stale data, responsive and accessibility behavior; remove placeholders.
- [x] 3.2 Review adjacent package contracts without implementing their scope.

## 4. Verification
- [x] 4.1 Run formatter, native build/tests, Web checks, browser flows, accessibility checks, change-name check, strict OpenSpec validation and Forge full Gate.
- [x] 4.2 Record revision-bound evidence and capture; archive without skip-specs only after implementation is complete.

# Tasks: Recommend and compare inventory builds

## 1. BFS — Baseline and impact coverage
- [ ] 1.1 Map REQ-I01–I03, REQ-D05, REQ-R01–R05 to actors, route, contracts, persistence, errors, fixtures and browser states; reconcile proposal/design/spec.
- [ ] 1.2 Inspect Forge shared artifacts and pinned token receipt; confirm local ownership and the implementation-handoff gate.

## 2. DFS — Requirement-by-requirement implementation
- [ ] 2.1 Implement domain types, validation, persistence and contract for build-recommendation; add failure fixtures.
- [ ] 2.2 Implement /recommend with all stated states, keyboard/focus, mobile layout and accessible feedback.
- [ ] 2.3 Connect the UI to the real API and verify each spec scenario end to end.

## 3. BFS — Cross-surface regression and completeness
- [ ] 3.1 Recheck callers, snapshots, error codes, privacy, stale data, responsive and accessibility behavior; remove placeholders.
- [ ] 3.2 Review adjacent package contracts without implementing their scope.

## 4. Verification
- [ ] 4.1 Run formatter, native build/tests, Web checks, browser flows, accessibility checks, change-name check, strict OpenSpec validation and Forge full Gate.
- [ ] 4.2 Record revision-bound evidence and capture; archive without skip-specs only after implementation is complete.

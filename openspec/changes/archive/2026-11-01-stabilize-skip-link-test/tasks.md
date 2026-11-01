# Tasks: Stabilise the skip-link focus test

## 1. Reproduce and bound the failure
- [x] 1.1 Re-read the failure, the test and the `/inventory` focus handling; confirm the app does not steal focus on load.
- [x] 1.2 Repeat the targeted test and the whole `landmarks.spec.ts` file to separate a harness race from an application defect.

## 2. Stabilise the setup
- [x] 2.1 Wait for the page `h1` and bring the page to front before the first Tab.
- [x] 2.2 Keep the focus assertions unchanged.

## 3. Prove it
- [x] 3.1 Run the targeted test 50 times with `--repeat-each`.
- [x] 3.2 Run the full browser suite twice; both must pass.
- [x] 3.3 Record the counts and the commands in `docs/evidence/stabilize-skip-link-test.md`.

## 4. Verify and close
- [x] 4.1 `npm run build`, `npm run check`, `npm test`, `npm run test:e2e`, `./verify-local.sh`.
- [x] 4.2 `openspec validate stabilize-skip-link-test --strict`; `openspec validate --all --strict`.
- [x] 4.3 Archive; commit the change, then `HANDOFF.md` only; no push.

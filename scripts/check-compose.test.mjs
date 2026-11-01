#!/usr/bin/env node
// Proves the compose gate in a temporary directory: a fixture with the contract
// shape passes, and a malformed fixture fails naming what is missing. The
// fixtures are normalised by the real `docker compose config`, so this is the
// same path the repository's compose.yaml takes. Nothing is written to the
// repository.
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { after, test } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const script = fileURLToPath(new URL("./check-compose.mjs", import.meta.url));
const scratch = mkdtempSync(join(tmpdir(), "rigcraft-compose-check-"));

after(() => rmSync(scratch, { recursive: true, force: true }));

const service = (overrides = "") => `services:
  rigcraft:
    build:
      context: .
      dockerfile: Dockerfile
    image: rigcraft:local
    ports:
      - "127.0.0.1:8080:8080"
    environment:
      RIGCRAFT_DATA_DIR: /data
    volumes:
      - rigcraft-data:/data
    read_only: true
    healthcheck:
      test: ["CMD", "curl", "--fail", "http://127.0.0.1:8080/api/health"]
      interval: 30s
volumes:
  rigcraft-data:
${overrides}`;

function fixture(name, text) {
  const file = join(scratch, `${name}.yaml`);
  writeFileSync(file, text);
  return file;
}

function run(file) {
  return spawnSync(process.execPath, [script, "--file", file], { encoding: "utf8" });
}

test("a fixture matching the contract passes", () => {
  const result = run(fixture("valid", service()));
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /PASS .*valid\.yaml: one service/);
  assert.match(result.stdout, /rigcraft-data at \/data/);
  assert.match(result.stdout, /127\.0\.0\.1:8080->8080/);
  assert.match(result.stdout, /read_only/);
});

test("a fixture that binds every interface and drops read_only fails", () => {
  const malformed = service()
    .replace('"127.0.0.1:8080:8080"', '"8080:8080"')
    .replace("    read_only: true\n", "");
  const result = run(fixture("malformed", malformed));
  assert.equal(result.status, 1);
  assert.match(result.stderr, /FAIL .*not 127\.0\.0\.1/);
  assert.match(result.stderr, /FAIL .*read_only/);
});

test("a fixture without a build or healthcheck fails", () => {
  const result = run(
    fixture(
      "no-build",
      `services:
  rigcraft:
    image: rigcraft:local
    read_only: true
volumes:
  rigcraft-data:
`,
    ),
  );
  assert.equal(result.status, 1);
  assert.match(result.stderr, /FAIL .*no local build/);
  assert.match(result.stderr, /FAIL .*no \/api\/health healthcheck/);
  assert.match(result.stderr, /FAIL .*named volume at \/data/);
});

// The gate's prerequisite is the container runtime. Proving a missing runtime
// fails would mean hiding docker from PATH, which node:test cannot do portably;
// the message and exit path are exercised by the two fixtures above through the
// same `check()` entry point.

#!/usr/bin/env node
// Proves the bundle budget in a temporary directory: a within-budget build
// passes, an over-budget file fails with its name, size, ceiling and delta, and
// an absent build fails instead of passing. Nothing is written to the repository.
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { after, test } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

import { CEILINGS } from "./check-bundle-size.mjs";

const script = fileURLToPath(new URL("./check-bundle-size.mjs", import.meta.url));
const scratch = mkdtempSync(join(tmpdir(), "rigcraft-bundle-budget-"));

after(() => rmSync(scratch, { recursive: true, force: true }));

function fixture(name, files) {
  const directory = join(scratch, name);
  mkdirSync(directory, { recursive: true });
  for (const [file, size] of Object.entries(files)) {
    writeFileSync(join(directory, file), "x".repeat(size));
  }
  return directory;
}

function run(assets) {
  return spawnSync(process.execPath, [script, "--assets", assets], { encoding: "utf8" });
}

test("a build within the ceiling passes", () => {
  const assets = fixture("within", { "index-abc.js": 1_024, "index-abc.css": 512 });
  const result = run(assets);
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /PASS .*index-abc\.js 1024 bytes \(JS ceiling 330000/);
  assert.match(result.stdout, /PASS .*index-abc\.css 512 bytes \(CSS ceiling 32000/);
  assert.match(result.stdout, /All 2 assets within budget\./);
});

test("an over-budget file fails and names its size, ceiling and delta", () => {
  const assets = fixture("over", {
    "index-abc.js": 1_024,
    "index-over.js": CEILINGS[".js"] + 1,
  });
  const result = run(assets);
  assert.equal(result.status, 1);
  assert.match(result.stderr, /FAIL .*index-over\.js 330001 bytes exceeds the JS ceiling 330000 by 1 byte/);
  assert.match(result.stderr, /PASS .*index-abc\.js/);
  assert.match(result.stderr, /1 asset over budget\./);
});

test("an absent build is a failure, not a pass", () => {
  const result = run(join(scratch, "absent"));
  assert.equal(result.status, 1);
  assert.match(result.stderr, /No built assets at .*absent.*npm run build/);
});

test("a build with no JavaScript or CSS is a failure", () => {
  const assets = fixture("empty", {});
  const result = run(assets);
  assert.equal(result.status, 1);
  assert.match(result.stderr, /No \.js or \.css assets at .*empty.*npm run build/);
});

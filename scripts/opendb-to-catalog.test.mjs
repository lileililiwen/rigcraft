// Behavioural tests for the catalog converter. They build a fake OpenDB
// checkout and run the real script, so the CLI contract itself is covered.
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

const script = new URL("./opendb-to-catalog.mjs", import.meta.url).pathname;
const created = [];

function checkout(files) {
  const root = mkdtempSync(join(tmpdir(), "rigcraft-opendb-catalog-"));
  created.push(root);
  for (const [path, value] of Object.entries(files)) {
    const target = join(root, "open-db", path);
    mkdirSync(join(target, ".."), { recursive: true });
    writeFileSync(target, JSON.stringify(value));
  }
  return root;
}

function run(root, extra = []) {
  return spawnSync(
    process.execPath,
    [
      script,
      root,
      "--name",
      "buildcores-opendb",
      "--license",
      "CC BY-NC-SA 4.0",
      "--commit",
      "abc1234",
      ...extra,
    ],
    { encoding: "utf8" },
  );
}

function cpu(overrides = {}) {
  return {
    opendb_id: "cpu-uuid",
    socket: "AM5",
    specifications: { tdp: 120, integratedGraphics: { model: "AMD Radeon Graphics" } },
    metadata: { name: "AMD Ryzen 7 7800X3D", part_numbers: ["100-100000910WOF"] },
    identifiers: { identifiers: [{ type: "mpn", value: "100-100000910WOF", region: "all" }] },
    general_product_information: {
      manufacturer_url: "https://www.amd.com/en/products/cpu/amd-ryzen-7-7800x3d",
    },
    ...overrides,
  };
}

test.after(() => {
  for (const root of created) rmSync(root, { recursive: true, force: true });
});

test("converts a checkout into the documented catalog document", () => {
  const root = checkout({ "CPU/cpu-1.json": cpu() });
  const result = run(root, ["--only", "CPU"]);
  assert.equal(result.status, 0, result.stderr);
  const document = JSON.parse(result.stdout);
  assert.equal(document.format, "rigcraft-catalog");
  assert.equal(document.version, 1);
  assert.equal(document.dataset.name, "buildcores-opendb");
  assert.equal(document.dataset.license, "CC BY-NC-SA 4.0");
  assert.equal(document.dataset.commit, "abc1234");
  assert.equal(
    document.dataset.sourceUrl,
    "https://github.com/buildcores/buildcores-open-db",
  );
  assert.equal(document.parts.length, 1);
  const [part] = document.parts;
  assert.equal(part.sku, "100-100000910WOF");
  assert.equal(part.externalId, "cpu-uuid");
  assert.equal(part.category, "cpu");
  assert.equal(part.model, "AMD Ryzen 7 7800X3D");
  assert.deepEqual(part.specs, { socket: "AM5", tdp: "120W", igpu: "yes" });
  assert.equal(
    part.sourceUrl,
    "https://www.amd.com/en/products/cpu/amd-ryzen-7-7800x3d",
  );
});

test("requires the operator's dataset name and licence", () => {
  const root = checkout({ "CPU/cpu-1.json": cpu() });
  const noName = spawnSync(
    process.execPath,
    [script, root, "--license", "CC0-1.0", "--commit", "abc1234"],
    { encoding: "utf8" },
  );
  assert.equal(noName.status, 1);
  assert.match(noName.stderr, /--name is required/);
  const noLicence = spawnSync(
    process.execPath,
    [script, root, "--name", "buildcores-opendb", "--commit", "abc1234"],
    { encoding: "utf8" },
  );
  assert.equal(noLicence.status, 1);
  assert.match(noLicence.stderr, /--license is required/);
});

test("refuses a shared SKU instead of writing an invalid document", () => {
  const root = checkout({
    "CPU/cpu-1.json": cpu(),
    "CPU/cpu-2.json": cpu({ opendb_id: "cpu-uuid-2" }),
  });
  const out = join(root, "catalog.json");
  const result = run(root, ["--only", "CPU", "--out", out]);
  assert.equal(result.status, 1);
  assert.match(result.stderr, /shared by .* refuses duplicates/);
  assert.throws(() => readFileSync(out), /ENOENT/);
});

test("refuses a part with no identity and writes nothing", () => {
  const root = checkout({
    "CPU/cpu-1.json": cpu({ metadata: { name: null, part_numbers: ["A"] } }),
  });
  const out = join(root, "catalog.json");
  const result = run(root, ["--only", "CPU", "--out", out]);
  assert.equal(result.status, 1);
  assert.match(result.stderr, /metadata\.name is missing/);
  assert.throws(() => readFileSync(out), /ENOENT/);
});

test("refuses an unmapped category", () => {
  const root = checkout({ "CPU/cpu-1.json": cpu() });
  const result = run(root, ["--only", "Chair"]);
  assert.equal(result.status, 1);
  assert.match(result.stderr, /Chair has no RigCraft mapping/);
});

test("omits an unusable manufacturer URL with a warning", () => {
  const root = checkout({
    "CPU/cpu-1.json": cpu({
      general_product_information: { manufacturer_url: "not a url" },
    }),
  });
  const result = run(root, ["--only", "CPU"]);
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stderr, /sourceUrl is omitted/);
  const document = JSON.parse(result.stdout);
  assert.equal(document.parts[0].sourceUrl, undefined);
});

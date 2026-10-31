// Behavioural tests for the OpenDB converter. They build a fake checkout and
// run the real script, so the assertions cover the CLI contract as documented.
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

const script = new URL("./opendb-to-csv.mjs", import.meta.url).pathname;
const created = [];

function checkout(files) {
  const root = mkdtempSync(join(tmpdir(), "rigcraft-opendb-"));
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
    [script, root, "--price", "42.50", "--quantity", "1", "--commit", "abc1234", ...extra],
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

function cooler(overrides = {}) {
  return {
    opendb_id: "cooler-uuid",
    height: 155,
    cpu_sockets: ["AM4", "AM5"],
    water_cooled: false,
    metadata: { name: "Thermalright Peerless Assassin 120 SE" },
    general_product_information: { manufacturer_url: "https://www.thermalright.com/product/peerless-assassin-120-se/" },
    ...overrides,
  };
}

test.after(() => {
  for (const root of created) rmSync(root, { recursive: true, force: true });
});

test("converts mapped categories with catalog provenance and operator stock values", () => {
  const root = checkout({
    "CPU/cpu-1.json": cpu(),
    "PCCase/case-1.json": {
      opendb_id: "case-uuid",
      supported_motherboard_form_factors: ["ATX", "Micro ATX"],
      supported_power_supply_form_factors: ["ATX"],
      max_video_card_length: 360,
      max_cpu_cooler_height: 170,
      max_psu_length: 220,
      metadata: { name: "Corsair 4000D AIRFLOW" },
      general_product_information: { manufacturer_url: "https://www.corsair.com/case" },
    },
  });
  const result = run(root, ["--only", "CPU", "--only", "PCCase"]);
  assert.equal(result.status, 0, result.stderr);
  const lines = result.stdout.trim().split("\n");
  assert.equal(
    lines[0],
    "sku,category,model,quantity,price,specs,source,sourceUrl",
  );
  assert.equal(lines.length, 3);
  const [sku, category, model, quantity, price, specs, source, url] = lines[1].split(",");
  assert.equal(sku, "100-100000910WOF");
  assert.equal(category, "cpu");
  assert.equal(model, "AMD Ryzen 7 7800X3D");
  assert.equal(quantity, "1");
  assert.equal(price, "42.50");
  assert.equal(specs, "socket=AM5;tdp=120W;igpu=yes");
  assert.equal(source, "catalog:buildcores@abc1234");
  assert.equal(url, "https://www.amd.com/en/products/cpu/amd-ryzen-7-7800x3d");
  assert.match(lines[2], /case,Corsair 4000D AIRFLOW/);
  assert.match(lines[2], /max_gpu_length=360mm/);
  assert.match(lines[2], /supported_form_factors=ATX\+Micro ATX/);
  assert.match(lines[2], /psu_form_factor=ATX/);
});

test("refuses a category it has no mapping for", () => {
  const root = checkout({ "CPU/cpu-1.json": cpu() });
  const result = run(root, ["--only", "Chair"]);
  assert.equal(result.status, 1);
  assert.match(result.stderr, /Chair has no RigCraft mapping/);
  assert.equal(result.stdout, "");
});

test("refuses a part with no name and writes nothing", () => {
  const root = checkout({
    "CPU/cpu-1.json": cpu({ metadata: { name: null, part_numbers: ["A"] } }),
  });
  const out = join(root, "result.csv");
  const result = run(root, ["--only", "CPU", "--out", out]);
  assert.equal(result.status, 1);
  assert.match(result.stderr, /metadata\.name is missing/);
  assert.throws(() => readFileSync(out), /ENOENT/);
});

test("surfaces a duplicate SKU instead of merging it", () => {
  const root = checkout({
    "CPU/cpu-1.json": cpu(),
    "CPU/cpu-2.json": cpu({ opendb_id: "cpu-uuid-2" }),
  });
  const result = run(root, ["--only", "CPU"]);
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stderr, /shared by .* resolve it as a duplicate/);
  assert.equal(result.stdout.trim().split("\n").length, 3);
});

test("omits an unusable manufacturer URL instead of failing the row", () => {
  const root = checkout({
    "CPU/cpu-1.json": cpu({
      general_product_information: { manufacturer_url: "not a url" },
    }),
  });
  const result = run(root, ["--only", "CPU"]);
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stderr, /sourceUrl is left empty/);
  const fields = result.stdout.trim().split("\n")[1].split(",");
  assert.equal(fields[fields.length - 1], "");
});

test("omits a multi-socket cooler socket and strips the M.2 prefix", () => {
  const root = checkout({
    "CPUCooler/cooler-1.json": cooler(),
    "Storage/storage-1.json": {
      opendb_id: "storage-uuid",
      capacity: 1000,
      form_factor: "M.2-2280",
      interface: "M.2 PCIe 4.0 x4",
      metadata: { name: "Corsair MP600 PRO NH 1TB" },
    },
  });
  const result = run(root, ["--only", "CPUCooler", "--only", "Storage"]);
  assert.equal(result.status, 0, result.stderr);
  const coolerLine = result.stdout.split("\n").find((line) => line.includes("cooler"));
  assert.match(coolerLine, /height=155mm/);
  assert.match(coolerLine, /type=air/);
  assert.doesNotMatch(coolerLine, /cooler_socket/);
  const storageLine = result.stdout.split("\n").find((line) => line.includes("storage"));
  assert.match(storageLine, /interface=PCIe 4\.0 x4/);
  assert.doesNotMatch(storageLine, /M\.2 PCIe/);
});

test("falls back to opendb_id when an MPN exceeds the import SKU limit", () => {
  const longMpn = "X".repeat(130);
  const root = checkout({
    "CPU/cpu-1.json": cpu({
      opendb_id: "cpu-fallback-id",
      identifiers: { identifiers: [{ type: "mpn", value: longMpn, region: "all" }] },
      metadata: { name: "Long MPN CPU" },
    }),
  });
  const result = run(root, ["--only", "CPU"]);
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stderr, /using opendb_id cpu-fallback-id/);
  assert.match(result.stdout, /^cpu-fallback-id,cpu,Long MPN CPU/m);
});

test("requires an operator price and quantity", () => {
  const root = checkout({ "CPU/cpu-1.json": cpu() });
  const result = spawnSync(process.execPath, [script, root, "--quantity", "1"], {
    encoding: "utf8",
  });
  assert.equal(result.status, 1);
  assert.match(result.stderr, /--price is required/);
});

#!/usr/bin/env node
// Convert a local BuildCores OpenDB checkout into the CSV RigCraft imports.
//
// OpenDB is not distributed with RigCraft. The operator fetches it under its own
// licence (CC BY-NC-SA 4.0) and runs this script locally. Every emitted row
// records the OpenDB commit in `source` and the manufacturer page in `sourceUrl`;
// a fact OpenDB does not record is omitted so it stays unknown.
import { execFileSync } from "node:child_process";
import { readFileSync, readdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const HEADER = [
  "sku",
  "category",
  "model",
  "quantity",
  "price",
  "specs",
  "source",
  "sourceUrl",
];

const MAPPINGS = [
  { folder: "CPU", category: "cpu", specs: cpuSpecs },
  { folder: "Motherboard", category: "motherboard", specs: motherboardSpecs },
  { folder: "RAM", category: "ram", specs: ramSpecs },
  { folder: "GPU", category: "gpu", specs: gpuSpecs },
  { folder: "PSU", category: "psu", specs: psuSpecs },
  { folder: "PCCase", category: "case", specs: caseSpecs },
  { folder: "CPUCooler", category: "cooler", specs: coolerSpecs },
  { folder: "Storage", category: "storage", specs: storageSpecs },
];

function fail(message) {
  console.error(`error: ${message}`);
  process.exit(1);
}

function text(value) {
  return typeof value === "string" && value.trim() !== "" ? value.trim() : null;
}

function count(value) {
  return typeof value === "number" && Number.isFinite(value) && value > 0
    ? Math.trunc(value)
    : 0;
}

function numericSpec(key, value, unit) {
  if (typeof value !== "number" || !Number.isFinite(value) || value <= 0) {
    return null;
  }
  return `${key}=${value}${unit}`;
}

function stringList(value) {
  return Array.isArray(value) ? value.filter((entry) => text(entry)) : [];
}

function objectList(value) {
  return Array.isArray(value)
    ? value.filter((entry) => entry && typeof entry === "object")
    : [];
}

function cpuSpecs(part) {
  const specs = [];
  const socket = text(part.socket);
  if (socket) specs.push(`socket=${socket}`);
  const tdp = part.specifications?.tdp;
  const tdpSpec = numericSpec("tdp", tdp, "W");
  if (tdpSpec) specs.push(tdpSpec);
  if (text(part.specifications?.integratedGraphics?.model)) {
    specs.push("igpu=yes");
  }
  return specs;
}

function pcieGeneration(slot) {
  const value = text(slot?.gen);
  return value && /^\d+\.\d+$/.test(value) ? value : null;
}

function motherboardSpecs(part) {
  const specs = [];
  const socket = text(part.socket);
  if (socket) specs.push(`socket=${socket}`);
  const form = text(part.form_factor);
  if (form) specs.push(`form_factor=${form}`);
  const memory = part.memory ?? {};
  if (text(memory.ram_type)) specs.push(`memory_type=${memory.ram_type}`);
  const slots = numericSpec("ram_slots", memory.slots, "");
  if (slots) specs.push(slots);
  const maxMemory = numericSpec("max_memory", memory.max, "GB");
  if (maxMemory) specs.push(maxMemory);
  const generations = objectList(part.pcie_slots)
    .filter((slot) => count(slot.quantity) > 0)
    .map(pcieGeneration)
    .filter(Boolean)
    .sort();
  if (generations.length > 0) {
    specs.push(`pcie_version=PCIe ${generations[generations.length - 1]}`);
  }
  // A board has several M.2 slots. The engine models one board storage
  // interface, so it is emitted only when every recorded slot agrees.
  const m2 = objectList(part.m2_slots)
    .map((slot) => text(slot.interface))
    .filter(Boolean);
  if (m2.length > 0 && new Set(m2).size === 1) {
    specs.push(`storage_interface=${m2[0]}`);
  }
  return specs;
}

function ramSpecs(part) {
  const specs = [];
  if (text(part.ram_type)) specs.push(`memory_type=${part.ram_type}`);
  const capacity = numericSpec("capacity", part.capacity, "GB");
  if (capacity) specs.push(capacity);
  const modules = numericSpec("modules", part.modules?.quantity, "");
  if (modules) specs.push(modules);
  return specs;
}

function connectorTokens(connectors) {
  if (!connectors || typeof connectors !== "object") return [];
  const pins = new Map();
  const add = (pin, value) => {
    const current = count(value);
    if (current > 0) pins.set(pin, (pins.get(pin) ?? 0) + current);
  };
  add(8, connectors.pcie_8_pin);
  add(6, connectors.pcie_6_pin);
  add(16, connectors.pcie_12VHPWR);
  add(16, connectors.pcie_12V_2x6);
  return [...pins.entries()]
    .sort((left, right) => left[0] - right[0])
    .map(([pin, quantity]) => `${quantity}x${pin}pin`);
}

function gpuSpecs(part) {
  const specs = [];
  if (text(part.interface)) specs.push(`interface=${part.interface}`);
  const length = numericSpec("length", part.length, "mm");
  if (length) specs.push(length);
  const tdp = numericSpec("tdp", part.tdp, "W");
  if (tdp) specs.push(tdp);
  const memory = numericSpec("vram", part.memory, "GB");
  if (memory) specs.push(memory);
  const connectors = connectorTokens(part.power_connectors);
  if (connectors.length > 0) specs.push(`connector=${connectors.join("+")}`);
  return specs;
}

function psuSpecs(part) {
  const specs = [];
  const wattage = numericSpec("wattage", part.wattage, "W");
  if (wattage) specs.push(wattage);
  if (text(part.form_factor)) specs.push(`form_factor=${part.form_factor}`);
  const length = numericSpec("length", part.length, "mm");
  if (length) specs.push(length);
  const connectors = part.connectors ?? {};
  const tokens = [];
  const eight = count(connectors.pcie_6_plus_2_pin);
  if (eight > 0) tokens.push(`${eight}x8pin`);
  const sixteen = count(connectors.pcie_12vhpwr);
  if (sixteen > 0) tokens.push(`${sixteen}x16pin`);
  if (tokens.length > 0) specs.push(`psu_connectors=${tokens.join("+")}`);
  return specs;
}

function caseSpecs(part) {
  const specs = [];
  const supported = stringList(part.supported_motherboard_form_factors);
  if (supported.length > 0) {
    specs.push(`supported_form_factors=${supported.join("+")}`);
  }
  const gpu = numericSpec("max_gpu_length", part.max_video_card_length, "mm");
  if (gpu) specs.push(gpu);
  const cooler = numericSpec("max_cooler_height", part.max_cpu_cooler_height, "mm");
  if (cooler) specs.push(cooler);
  const psu = numericSpec("max_psu_length", part.max_psu_length, "mm");
  if (psu) specs.push(psu);
  const psuForms = stringList(part.supported_power_supply_form_factors);
  if (psuForms.includes("ATX")) {
    specs.push("psu_form_factor=ATX");
  } else if (psuForms.length === 1) {
    specs.push(`psu_form_factor=${psuForms[0]}`);
  }
  return specs;
}

function coolerSpecs(part) {
  const specs = [];
  const height = numericSpec("height", part.height, "mm");
  if (height) specs.push(height);
  // A cooler lists every socket it mounts. The engine compares one socket, so a
  // multi-socket cooler stays unknown rather than being reduced to a guess.
  const sockets = stringList(part.cpu_sockets);
  if (sockets.length === 1) specs.push(`cooler_socket=${sockets[0]}`);
  if (part.water_cooled === true) specs.push("type=liquid");
  else if (part.water_cooled === false) specs.push("type=air");
  return specs;
}

function storageSpecs(part) {
  const specs = [];
  const interfaceValue = text(part.interface);
  if (interfaceValue) {
    specs.push(`interface=${interfaceValue.replace(/^M\.2\s+/, "")}`);
  }
  const capacity = numericSpec("capacity", part.capacity, "GB");
  if (capacity) specs.push(capacity);
  if (text(part.form_factor)) specs.push(`form_factor=${part.form_factor}`);
  return specs;
}

function skuOf(part) {
  const identifiers = objectList(part.identifiers?.identifiers);
  const mpn = identifiers.find(
    (entry) => text(entry.type)?.toLowerCase() === "mpn" && text(entry.value),
  );
  if (mpn) return text(mpn.value);
  const partNumbers = stringList(part.metadata?.part_numbers);
  if (partNumbers.length > 0) return text(partNumbers[0]);
  return text(part.opendb_id);
}

function sourceUrlOf(part) {
  const url = text(part.general_product_information?.manufacturer_url);
  if (!url) return null;
  if (!/^https?:\/\/\S+$/.test(url)) {
    return { invalid: url };
  }
  return { url };
}

function csvField(value) {
  const string = String(value ?? "");
  return /[",\n\r]/.test(string)
    ? `"${string.replaceAll('"', '""')}"`
    : string;
}

function parseArgs(argv) {
  const options = {
    checkout: null,
    out: null,
    price: null,
    quantity: null,
    commit: null,
    folders: [],
  };
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    const next = () => {
      const value = argv[index + 1];
      if (value === undefined) fail(`${argument} needs a value`);
      index += 1;
      return value;
    };
    if (argument === "--out") options.out = next();
    else if (argument === "--price") options.price = next();
    else if (argument === "--quantity") options.quantity = next();
    else if (argument === "--commit") options.commit = next();
    else if (argument === "--only") options.folders.push(next());
    else if (argument === "--help" || argument === "-h") {
      console.log(
        "usage: node scripts/opendb-to-csv.mjs <opendb-checkout> --price <decimal> --quantity <integer> [--out file] [--commit sha] [--only Folder]",
      );
      process.exit(0);
    } else if (argument.startsWith("-")) fail(`unknown option ${argument}`);
    else if (options.checkout === null) options.checkout = argument;
    else fail(`unexpected argument ${argument}`);
  }
  if (!options.checkout) fail("pass the path to a local OpenDB checkout");
  if (options.price === null || !/^\d+(\.\d{1,2})?$/.test(options.price)) {
    fail("--price is required and must be a non-negative number with at most two decimals");
  }
  if (options.quantity === null || !/^\d+$/.test(options.quantity)) {
    fail("--quantity is required and must be a non-negative whole number");
  }
  return options;
}

function resolveCommit(checkout, explicit) {
  if (explicit) {
    if (!/^[0-9a-f]{7,40}$/i.test(explicit)) {
      fail("--commit must be a hexadecimal git revision");
    }
    return explicit;
  }
  try {
    const value = execFileSync("git", ["-C", checkout, "rev-parse", "HEAD"], {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    }).trim();
    if (!/^[0-9a-f]{40}$/i.test(value)) {
      fail("the checkout commit could not be resolved; pass --commit <sha>");
    }
    return value;
  } catch {
    fail("the checkout is not a git repository; pass --commit <sha>");
  }
}

function main() {
  const options = parseArgs(process.argv.slice(2));
  const commit = resolveCommit(options.checkout, options.commit);
  const source = `catalog:buildcores@${commit}`;
  const requested =
    options.folders.length > 0
      ? options.folders
      : MAPPINGS.map((mapping) => mapping.folder);
  const mappings = requested.map((folder) => {
    const mapping = MAPPINGS.find(
      (entry) => entry.folder.toLowerCase() === folder.toLowerCase(),
    );
    if (!mapping) {
      fail(`category ${folder} has no RigCraft mapping; supported: ${MAPPINGS.map((entry) => entry.folder).join(", ")}`);
    }
    return mapping;
  });

  const problems = [];
  const warnings = [];
  const rows = [];
  const seenSkus = new Map();
  for (const mapping of mappings) {
    const directory = join(options.checkout, "open-db", mapping.folder);
    let entries;
    try {
      entries = readdirSync(directory, { withFileTypes: true });
    } catch {
      problems.push(`${directory}: category folder is missing`);
      continue;
    }
    for (const entry of entries) {
      if (!entry.isFile() || !entry.name.endsWith(".json")) continue;
      const file = join(directory, entry.name);
      let part;
      try {
        part = JSON.parse(readFileSync(file, "utf8"));
      } catch (error) {
        problems.push(`${file}: cannot parse JSON (${error.message})`);
        continue;
      }
      const model = text(part.metadata?.name);
      if (!model) {
        problems.push(`${file}: metadata.name is missing`);
        continue;
      }
      let sku = skuOf(part);
      if (!sku) {
        problems.push(`${file}: no MPN, part number or opendb_id`);
        continue;
      }
      if (sku.length > 120) {
        const identifier = text(part.opendb_id);
        if (!identifier) {
          problems.push(`${file}: SKU ${sku.slice(0, 40)}… exceeds 120 characters and opendb_id is missing`);
          continue;
        }
        warnings.push(
          `${file}: SKU ${sku.slice(0, 40)}… exceeds 120 characters; using opendb_id ${identifier}`,
        );
        sku = identifier;
      }
      const key = sku.toUpperCase();
      if (seenSkus.has(key)) {
        warnings.push(
          `SKU ${sku} is shared by ${seenSkus.get(key)} and ${file}; resolve it as a duplicate in the import preview`,
        );
      } else {
        seenSkus.set(key, file);
      }
      if (model.length > 200) {
        warnings.push(
          `${file}: model is ${model.length} characters; the import preview will flag it as too long`,
        );
      }
      const url = sourceUrlOf(part);
      if (url && url.invalid) {
        warnings.push(
          `${file}: manufacturer_url is not an http(s) URL; sourceUrl is left empty`,
        );
      }
      rows.push([
        sku,
        mapping.category,
        model,
        options.quantity,
        options.price,
        mapping.specs(part).join(";"),
        source,
        url?.url ?? "",
      ]);
    }
  }
  if (problems.length > 0) {
    for (const problem of problems) console.error(`error: ${problem}`);
    console.error(`error: ${problems.length} problem(s); no CSV was written`);
    process.exit(1);
  }
  for (const warning of warnings) console.error(`warning: ${warning}`);
  if (rows.length === 0) fail("no parts were mapped");
  const csv = [HEADER, ...rows]
    .map((row) => row.map(csvField).join(","))
    .join("\n")
    .concat("\n");
  if (options.out) writeFileSync(options.out, csv);
  else process.stdout.write(csv);
  console.error(
    `converted ${rows.length} parts from ${source}${warnings.length > 0 ? ` with ${warnings.length} warning(s)` : ""}`,
  );
}

main();

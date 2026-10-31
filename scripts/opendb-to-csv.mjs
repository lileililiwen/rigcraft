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

import { MAPPINGS, skuOf, sourceUrlOf, text } from "./opendb-fields.mjs";

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


function fail(message) {
  console.error(`error: ${message}`);
  process.exit(1);
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

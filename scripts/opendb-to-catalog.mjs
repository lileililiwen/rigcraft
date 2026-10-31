#!/usr/bin/env node
// Convert a local BuildCores OpenDB checkout into the rigcraft-catalog document
// that POST /api/catalog/datasets accepts.
//
// OpenDB is not distributed with RigCraft. The operator fetches it under its own
// licence and runs this script locally, naming the dataset and its licence. The
// conversion is an external step; RigCraft never fetches or redistributes the
// result.
import { execFileSync } from "node:child_process";
import { readFileSync, readdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { MAPPINGS, skuOf, sourceUrlOf, text } from "./opendb-fields.mjs";

const NAME_PATTERN = /^[A-Za-z0-9][A-Za-z0-9._@+/-]{0,119}$/;

function fail(message) {
  console.error(`error: ${message}`);
  process.exit(1);
}

function parseArgs(argv) {
  const options = {
    checkout: null,
    out: null,
    commit: null,
    name: null,
    license: null,
    sourceUrl: "https://github.com/buildcores/buildcores-open-db",
    retrievedAt: null,
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
    else if (argument === "--commit") options.commit = next();
    else if (argument === "--name") options.name = next();
    else if (argument === "--license") options.license = next();
    else if (argument === "--source-url") options.sourceUrl = next();
    else if (argument === "--retrieved-at") options.retrievedAt = next();
    else if (argument === "--only") options.folders.push(next());
    else if (argument === "--help" || argument === "-h") {
      console.log(
        "usage: node scripts/opendb-to-catalog.mjs <opendb-checkout> --name <dataset-token> --license <text> [--out file] [--commit sha] [--only Folder] [--source-url url] [--retrieved-at ISO]",
      );
      process.exit(0);
    } else if (argument.startsWith("-")) fail(`unknown option ${argument}`);
    else if (options.checkout === null) options.checkout = argument;
    else fail(`unexpected argument ${argument}`);
  }
  if (!options.checkout) fail("pass the path to a local OpenDB checkout");
  if (!options.name || !NAME_PATTERN.test(options.name)) {
    fail(
      "--name is required and must be a dataset token: letters, digits and . _ @ / + - only, starting with a letter or digit",
    );
  }
  if (!options.license || options.license.trim() === "") {
    fail("--license is required; state the licence the operator is using");
  }
  if (options.sourceUrl && !/^https?:\/\/\S+$/.test(options.sourceUrl)) {
    fail("--source-url must be an absolute http(s) URL");
  }
  if (options.retrievedAt && Number.isNaN(Date.parse(options.retrievedAt))) {
    fail("--retrieved-at must be an ISO 8601 timestamp");
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

function specsObject(specs) {
  const values = {};
  for (const spec of specs) {
    const index = spec.indexOf("=");
    if (index <= 0) continue;
    values[spec.slice(0, index)] = spec.slice(index + 1);
  }
  return values;
}

function main() {
  const options = parseArgs(process.argv.slice(2));
  const commit = resolveCommit(options.checkout, options.commit);
  const requested =
    options.folders.length > 0
      ? options.folders
      : MAPPINGS.map((mapping) => mapping.folder);
  const mappings = requested.map((folder) => {
    const mapping = MAPPINGS.find(
      (entry) => entry.folder.toLowerCase() === folder.toLowerCase(),
    );
    if (!mapping) {
      fail(
        `category ${folder} has no RigCraft mapping; supported: ${MAPPINGS.map((entry) => entry.folder).join(", ")}`,
      );
    }
    return mapping;
  });

  const problems = [];
  const warnings = [];
  const parts = [];
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
          problems.push(
            `${file}: SKU ${sku.slice(0, 40)}… exceeds 120 characters and opendb_id is missing`,
          );
          continue;
        }
        warnings.push(
          `${file}: SKU ${sku.slice(0, 40)}… exceeds 120 characters; using opendb_id ${identifier}`,
        );
        sku = identifier;
      }
      if (model.length > 200) {
        problems.push(
          `${file}: model is ${model.length} characters; the catalog document refuses names over 200. Curate or exclude this file.`,
        );
        continue;
      }
      const key = sku.toUpperCase();
      if (seenSkus.has(key)) {
        // The catalog refuses duplicates rather than merging them, so the
        // converter must not write a document that would be rejected.
        problems.push(
          `SKU ${sku} is shared by ${seenSkus.get(key)} and ${file}; the catalog refuses duplicates, so curate one of them`,
        );
        continue;
      }
      seenSkus.set(key, file);
      const url = sourceUrlOf(part);
      if (url && url.invalid) {
        warnings.push(
          `${file}: manufacturer_url is not an http(s) URL; sourceUrl is omitted`,
        );
      }
      const partDocument = {
        externalId: text(part.opendb_id) ?? undefined,
        sku,
        category: mapping.category,
        model,
        specs: specsObject(mapping.specs(part)),
      };
      if (url?.url) partDocument.sourceUrl = url.url;
      for (const field of ["externalId", "sourceUrl"]) {
        if (partDocument[field] === undefined) delete partDocument[field];
      }
      parts.push(partDocument);
    }
  }
  if (problems.length > 0) {
    for (const problem of problems) console.error(`error: ${problem}`);
    console.error(`error: ${problems.length} problem(s); no catalog was written`);
    process.exit(1);
  }
  for (const warning of warnings) console.error(`warning: ${warning}`);
  if (parts.length === 0) fail("no parts were mapped");

  const dataset = {
    name: options.name,
    license: options.license.trim(),
    commit,
  };
  if (options.sourceUrl) dataset.sourceUrl = options.sourceUrl;
  if (options.retrievedAt) dataset.retrievedAt = options.retrievedAt;
  const document = {
    format: "rigcraft-catalog",
    version: 1,
    dataset,
    parts,
  };
  const json = `${JSON.stringify(document, null, 2)}\n`;
  if (options.out) writeFileSync(options.out, json);
  else process.stdout.write(json);
  console.error(
    `converted ${parts.length} parts into catalog ${options.name}@${commit}${warnings.length > 0 ? ` with ${warnings.length} warning(s)` : ""}`,
  );
}

main();

#!/usr/bin/env node
// Fails when a built client asset exceeds its type's ceiling.
//
// The check reads the fresh build, never a cached copy: `npm run build` runs it
// after Vite writes `apps/web/dist/assets`, and `npm run check` runs it too. A
// missing or empty build is a failure with the command to run, so a fresh
// checkout cannot pass by having nothing to measure.
//
// The ceilings are declared here, per file type, rather than read from the
// environment or a build tool: a budget that can be switched off is not a
// budget. Raising one is a code change under review.
import { readdirSync, statSync } from "node:fs";
import { extname, join, relative, resolve } from "node:path";
import { pathToFileURL } from "node:url";

/** Ceilings in bytes for one built asset, by type. */
export const CEILINGS = { ".js": 330_000, ".css": 32_000 };

const DEFAULT_ASSETS = "apps/web/dist/assets";

function assetFiles(directory) {
  let entries;
  try {
    entries = readdirSync(directory, { withFileTypes: true });
  } catch {
    throw new Error(
      `No built assets at ${directory}. Run \`npm run build\` first; this check reads the fresh build, never an absent dist.`,
    );
  }
  const files = entries
    .filter((entry) => entry.isFile() && CEILINGS[extname(entry.name)])
    .map((entry) => join(directory, entry.name))
    .sort();
  if (files.length === 0) {
    throw new Error(
      `No .js or .css assets at ${directory}. Run \`npm run build\` first; an empty build is not a passing budget.`,
    );
  }
  return files;
}

function plural(count, singular, pluralForm) {
  return `${count} ${count === 1 ? singular : pluralForm}`;
}

function bytes(count) {
  return plural(count, "byte", "bytes");
}

/**
 * Measure every JavaScript and CSS asset in `directory`.
 * Returns the rows and the failures; throws only when there is nothing to measure.
 */
export function measure(directory) {
  const rows = assetFiles(directory).map((file) => {
    const type = extname(file);
    const size = statSync(file).size;
    const ceiling = CEILINGS[type];
    const displayed = relative(process.cwd(), file);
    return {
      file: displayed.startsWith("..") ? file : displayed,
      type: type === ".js" ? "JS" : "CSS",
      size,
      ceiling,
      delta: size - ceiling,
      pass: size <= ceiling,
    };
  });
  return { rows, failures: rows.filter((row) => !row.pass) };
}

export function format(rows, failures) {
  const lines = rows.map((row) =>
    row.pass
      ? `PASS ${row.file} ${row.size} bytes (${row.type} ceiling ${row.ceiling}, ${bytes(-row.delta)} headroom)`
      : `FAIL ${row.file} ${row.size} bytes exceeds the ${row.type} ceiling ${row.ceiling} by ${bytes(row.delta)}`,
  );
  if (failures.length > 0) {
    lines.push(`${plural(failures.length, "asset", "assets")} over budget.`);
  } else {
    lines.push(`All ${plural(rows.length, "asset", "assets")} within budget.`);
  }
  return lines.join("\n");
}

function main(argv) {
  const assetsIndex = argv.indexOf("--assets");
  const directory =
    assetsIndex === -1 ? resolve(process.cwd(), DEFAULT_ASSETS) : resolve(argv[assetsIndex + 1]);

  let result;
  try {
    result = measure(directory);
  } catch (error) {
    console.error(`FAIL ${error.message}`);
    return 1;
  }

  const output = format(result.rows, result.failures);
  if (result.failures.length > 0) {
    console.error(output);
    return 1;
  }
  console.log(output);
  return 0;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  process.exitCode = main(process.argv.slice(2));
}

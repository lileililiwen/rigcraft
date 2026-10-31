#!/usr/bin/env node
// Measures the vendored token pairs the inventory UI uses and writes
// docs/evidence/token-contrast.json. Fails when any pair misses its WCAG 2.2
// minimum, so the check can be bound to `npm run check` instead of being a
// document that no gate reads.
import { createHash } from "node:crypto";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve as resolvePath } from "node:path";

const root = resolvePath(process.cwd());
const tokenFile = resolvePath(root, "apps/web/src/styles/vendor/tokens.css");
const outputFile = resolvePath(root, "docs/evidence/token-contrast.json");
const css = readFileSync(tokenFile, "utf8");

const variables = new Map();
for (const match of css.matchAll(/--([a-z0-9-]+)\s*:\s*([^;]+);/gi)) {
  variables.set(match[1], match[2].trim());
}

function resolveVariable(name, trail = []) {
  if (trail.includes(name)) {
    throw new Error(`Token alias cycle: ${[...trail, name].join(" -> ")}`);
  }
  const value = variables.get(name);
  if (!value) {
    throw new Error(`Unknown token --${name}`);
  }
  const alias = value.match(/^var\(--([a-z0-9-]+)\)$/i);
  if (alias) {
    return resolveVariable(alias[1], [...trail, name]);
  }
  return value;
}

function channel(value) {
  const normalised = value / 255;
  return normalised <= 0.04045
    ? normalised / 12.92
    : Math.pow((normalised + 0.055) / 1.055, 2.4);
}

function luminance(hex) {
  const value = hex.replace("#", "");
  if (!/^[0-9a-f]{6}$/i.test(value)) {
    throw new Error(`Expected a 6-digit hex color, received ${hex}`);
  }
  const [r, g, b] = [0, 2, 4].map((index) =>
    channel(Number.parseInt(value.slice(index, index + 2), 16)),
  );
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

function contrast(foreground, background) {
  const values = [luminance(foreground), luminance(background)].sort((a, b) => b - a);
  return (values[0] + 0.05) / (values[1] + 0.05);
}

const pairs = [
  ["Body text on page", "color-foreground", "color-background", 4.5],
  ["Body text on surface", "color-foreground", "color-surface", 4.5],
  ["Secondary text on page", "color-muted-foreground", "color-background", 4.5],
  ["Secondary text on muted", "color-muted-foreground", "color-muted", 4.5],
  ["Primary button label", "color-on-primary", "color-primary", 4.5],
  ["Primary button hover label", "color-on-primary", "color-primary-hover", 4.5],
  ["Link on surface", "color-primary", "color-surface", 4.5],
  ["Error text on surface", "color-danger", "color-surface", 4.5],
  ["Success text on muted", "color-success", "color-muted", 4.5],
  ["Warning text on muted", "color-warning", "color-muted", 4.5],
  ["Control border on surface", "color-border", "color-surface", 3],
  ["Focus ring on page", "focus-ring", "color-background", 3],
  ["Focus ring on surface", "focus-ring", "color-surface", 3],
];

const results = pairs.map(([name, foregroundToken, backgroundToken, required]) => {
  const foreground = resolveVariable(foregroundToken);
  const background = resolveVariable(backgroundToken);
  const ratio = contrast(foreground, background);
  return {
    name,
    foreground: `--${foregroundToken}`,
    background: `--${backgroundToken}`,
    resolvedForeground: foreground,
    resolvedBackground: background,
    ratio: Number(ratio.toFixed(2)),
    required,
    pass: ratio >= required,
  };
});

const failures = results.filter((entry) => !entry.pass);
const report = {
  schemaVersion: 1,
  standard: "WCAG 2.2 AA",
  source: {
    file: "apps/web/src/styles/vendor/tokens.css",
    sha256: createHash("sha256").update(css).digest("hex"),
  },
  pairs: results,
  status: failures.length === 0 ? "pass" : "fail",
};

mkdirSync(dirname(outputFile), { recursive: true });
writeFileSync(outputFile, `${JSON.stringify(report, null, 2)}\n`);

for (const entry of results) {
  const verdict = entry.pass ? "PASS" : "FAIL";
  console.log(
    `${verdict} ${entry.ratio.toFixed(2)}:1 (needs ${entry.required}:1) ${entry.name}`,
  );
}
if (failures.length > 0) {
  console.error(`${failures.length} token pair(s) below the WCAG 2.2 AA minimum.`);
  process.exit(1);
}
console.log(`Wrote ${outputFile}`);

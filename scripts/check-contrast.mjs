#!/usr/bin/env node
// Measures the effective theme's token pairs and writes
// docs/evidence/token-contrast.json. Fails when any pair misses its WCAG 2.2
// minimum, so the check can be bound to `npm run check` instead of being a
// document that no gate reads.
//
// The theme is resolved from an ordered source list, exactly as the cascade
// does it: a later declaration of a variable wins. The vendored kit is
// generated and hash-pinned; the brand layer is the app-owned override, and it
// must declare custom properties only. Every measured pair must resolve to two
// flat 6-digit hex colours: a `color-mix()` or alpha value is decoration, and
// this checker lists it as decoration instead of inventing a text ratio for it.
import { createHash } from "node:crypto";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve as resolvePath } from "node:path";

const root = resolvePath(process.cwd());
const outputFile = resolvePath(root, "docs/evidence/token-contrast.json");

const sources = [
  {
    file: "apps/web/src/styles/vendor/tokens.css",
    role: "vendored shared kit (generated, hash-pinned, not edited)",
  },
  {
    file: "apps/web/src/styles/brand.css",
    role: "app-owned brand layer (declared after the kit)",
  },
];

/** The brand layer may not smuggle in a selector or a non-custom property. */
function assertCustomPropertiesOnly(file, css) {
  const stripped = css.replace(/\/\*[\s\S]*?\*\//g, "");
  for (const block of stripped.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
    const selector = block[1].trim();
    if (selector !== ":root") {
      throw new Error(`${file}: only a :root block is allowed, found "${selector}"`);
    }
    for (const raw of block[2].split(";")) {
      const declaration = raw.trim();
      if (!declaration) {
        continue;
      }
      const [property] = declaration.split(":");
      if (!property.trim().startsWith("--")) {
        throw new Error(
          `${file}: "${property.trim()}" is a regular property; the brand layer declares custom properties only`,
        );
      }
      if (!declaration.includes(":")) {
        throw new Error(`${file}: malformed declaration "${declaration}"`);
      }
    }
  }
}

const variables = new Map();
const resolvedSources = [];
for (const source of sources) {
  const absolute = resolvePath(root, source.file);
  const css = readFileSync(absolute, "utf8");
  if (source.file.endsWith("brand.css")) {
    assertCustomPropertiesOnly(source.file, css);
  }
  for (const match of css.matchAll(/--([a-z0-9-]+)\s*:\s*([^;]+);/gi)) {
    variables.set(match[1], match[2].trim());
  }
  resolvedSources.push({
    file: source.file,
    role: source.role,
    sha256: createHash("sha256").update(css).digest("hex"),
  });
}

const FLAT_HEX = /^#[0-9a-f]{6}$/i;

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

/** Text pairs stay flat, so no pair is claimed for a value this cannot resolve. */
function flatPair(foregroundToken, backgroundToken) {
  const foreground = resolveVariable(foregroundToken);
  const background = resolveVariable(backgroundToken);
  for (const [token, value] of [
    [foregroundToken, foreground],
    [backgroundToken, background],
  ]) {
    if (!FLAT_HEX.test(value)) {
      throw new Error(
        `--${token} resolves to "${value}"; a measured pair needs a flat hex colour (decoration is never a text pair)`,
      );
    }
  }
  return { foreground, background };
}

// The 16 pairs the interface already used, re-measured on the effective dark
// theme, followed by the dark-only pairs. Every text pair is 4.5:1; a border or
// focus ring is a non-text UI pair at 3:1.
const pairSpecs = [
  ["Body text on page", "color-foreground", "color-background", 4.5],
  ["Body text on surface", "color-foreground", "color-surface", 4.5],
  ["Body text on raised", "color-foreground", "color-surface-raised", 4.5],
  ["Secondary text on page", "color-muted-foreground", "color-background", 4.5],
  ["Secondary text on surface", "color-muted-foreground", "color-surface", 4.5],
  ["Secondary text on raised", "color-muted-foreground", "color-surface-raised", 4.5],
  ["Secondary text on muted", "color-muted-foreground", "color-muted", 4.5],
  ["Primary button label", "color-on-primary", "color-primary", 4.5],
  ["Primary button hover label", "color-on-primary", "color-primary-hover", 4.5],
  ["Link on surface", "color-primary", "color-surface", 4.5],
  ["Link on raised", "color-primary", "color-surface-raised", 4.5],
  ["Error text on surface", "color-danger", "color-surface", 4.5],
  ["Error text on raised", "color-danger", "color-surface-raised", 4.5],
  ["Error text on muted", "color-danger", "color-muted", 4.5],
  ["Success text on surface", "color-success", "color-surface", 4.5],
  ["Success text on raised", "color-success", "color-surface-raised", 4.5],
  ["Success text on muted", "color-success", "color-muted", 4.5],
  ["Warning text on surface", "color-warning", "color-surface", 4.5],
  ["Warning text on raised", "color-warning", "color-surface-raised", 4.5],
  ["Warning text on muted", "color-warning", "color-muted", 4.5],
  ["Warning text on page", "color-warning", "color-background", 4.5],
  ["Catalog badge on muted", "color-primary", "color-muted", 4.5],
  ["Chip text on verified tint", "color-success", "color-chip-verified", 4.5],
  ["Chip text on warning tint", "color-warning", "color-chip-warning", 4.5],
  ["Chip text on unknown tint", "color-warning", "color-chip-unknown", 4.5],
  ["Chip text on neutral tint", "color-muted-foreground", "color-chip-neutral", 4.5],
  ["Chip text on danger tint", "color-danger", "color-chip-danger", 4.5],
  ["Chip text on primary tint", "color-primary", "color-chip-primary", 4.5],
  ["Control border on page", "color-border", "color-background", 3],
  ["Control border on surface", "color-border", "color-surface", 3],
  ["Control border on raised", "color-border", "color-surface-raised", 3],
  ["Focus ring on page", "focus-ring", "color-background", 3],
  ["Focus ring on surface", "focus-ring", "color-surface", 3],
  ["Focus ring on raised", "focus-ring", "color-surface-raised", 3],
  ["Focus ring on verified tint", "focus-ring", "color-chip-verified", 3],
  ["Focus ring on warning tint", "focus-ring", "color-chip-warning", 3],
  ["Focus ring on unknown tint", "focus-ring", "color-chip-unknown", 3],
  ["Focus ring on neutral tint", "focus-ring", "color-chip-neutral", 3],
  ["Focus ring on danger tint", "focus-ring", "color-chip-danger", 3],
  ["Focus ring on primary tint", "focus-ring", "color-chip-primary", 3],
];

const results = pairSpecs.map(([name, foregroundToken, backgroundToken, required]) => {
  const { foreground, background } = flatPair(foregroundToken, backgroundToken);
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

// Decoration is listed so its absence from the measured pairs is a decision,
// not an omission. It carries no text and has no computable pair.
const decoration = [
  {
    token: "color-border-subtle",
    value: resolveVariable("color-border-subtle"),
    reason:
      "1 px separators inside raised surfaces; a color-mix() value with no text on it, so no pair is claimed for it.",
  },
].filter((entry) => Boolean(entry.value));

const failures = results.filter((entry) => !entry.pass);
const lowest = results.reduce((worst, entry) =>
  entry.ratio < worst.ratio ? entry : worst,
);
const report = {
  schemaVersion: 2,
  standard: "WCAG 2.2 AA",
  mode: "dark",
  sources: resolvedSources,
  decoration,
  pairs: results,
  summary: {
    pairs: results.length,
    passed: results.length - failures.length,
    failed: failures.length,
    lowest: { name: lowest.name, ratio: lowest.ratio, required: lowest.required },
  },
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
for (const entry of decoration) {
  console.log(`DECORATION (not measured) --${entry.token}: ${entry.value}`);
}
console.log(
  `${resolvedSources.length} token source(s): ${resolvedSources
    .map((source) => `${source.file} ${source.sha256.slice(0, 12)}…`)
    .join(", ")}`,
);
if (failures.length > 0) {
  console.error(`${failures.length} token pair(s) below the WCAG 2.2 AA minimum.`);
  process.exit(1);
}
console.log(`Wrote ${outputFile}`);

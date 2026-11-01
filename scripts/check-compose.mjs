#!/usr/bin/env node
// Validates the Compose topology that Forge deploys.
//
// A container artifact is checked by a container runtime: `docker compose config`
// parses the file and resolves interpolation, and this script then asserts the
// shape a reviewer would otherwise have to eyeball — one service, a local build,
// a healthcheck, the stateful `/data` volume, a loopback-only published port and
// a read-only root filesystem. Docker is a declared prerequisite, not an opt-out:
// without it the topology cannot be validated, so the check fails and names it.
import { spawnSync } from "node:child_process";
import { dirname, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const ROOT = dirname(dirname(fileURLToPath(import.meta.url)));
const DEFAULT_FILE = resolve(ROOT, "compose.yaml");
const PREREQUISITE =
  "Docker Engine with the `docker compose` v2 plugin is required to validate a container artifact";

function run(args) {
  return spawnSync("docker", args, { encoding: "utf8" });
}

/**
 * Assert the normalised `docker compose config --format json` shape.
 * Returns the failure messages; an empty array means the topology is acceptable.
 */
export function validate(config) {
  const failures = [];
  const services = config?.services ?? {};
  const names = Object.keys(services);
  if (names.length !== 1 || names[0] !== "rigcraft") {
    failures.push(`expected exactly one service named rigcraft, found: ${names.join(", ") || "none"}`);
  }
  const service = services.rigcraft;
  if (!service) return failures;

  if (!service.build || service.build.dockerfile !== "Dockerfile" || !service.build.context) {
    failures.push("the rigcraft service has no local build (context and Dockerfile)");
  }

  const healthTest = JSON.stringify(service.healthcheck?.test ?? "");
  if (!healthTest.includes("/api/health")) {
    failures.push("the rigcraft service has no /api/health healthcheck");
  }

  const dataVolume = (service.volumes ?? []).find(
    (volume) => volume.type === "volume" && volume.target === "/data",
  );
  if (!dataVolume) {
    failures.push("the rigcraft service does not mount a named volume at /data");
  }

  const ports = service.ports ?? [];
  if (ports.length === 0) {
    failures.push("the rigcraft service publishes no port");
  }
  for (const port of ports) {
    if (port.host_ip !== "127.0.0.1" || Number(port.target) !== 8080) {
      failures.push(
        `published port ${JSON.stringify(port)} is not 127.0.0.1:<host>:8080; the service has no authentication and must not bind a public interface by default`,
      );
    }
  }

  if (service.read_only !== true) {
    failures.push("the rigcraft service does not set read_only: true");
  }

  return failures;
}

/** Run compose against `file`, then validate the resolved configuration. */
export function check(file = DEFAULT_FILE) {
  const probe = run(["compose", "version"]);
  if (probe.error || probe.status !== 0) {
    return { ok: false, failures: [`${PREREQUISITE} (${probe.error?.message ?? probe.stderr.trim()})`] };
  }

  const quiet = run(["compose", "-f", file, "config", "--quiet"]);
  if (quiet.error || quiet.status !== 0) {
    return {
      ok: false,
      failures: [`docker compose config --quiet rejected ${file}: ${quiet.error?.message ?? quiet.stderr.trim()}`],
    };
  }

  const resolved = run(["compose", "-f", file, "config", "--format", "json"]);
  if (resolved.error || resolved.status !== 0) {
    return {
      ok: false,
      failures: [`docker compose config --format json failed for ${file}: ${resolved.error?.message ?? resolved.stderr.trim()}`],
    };
  }
  let config;
  try {
    config = JSON.parse(resolved.stdout);
  } catch (error) {
    return { ok: false, failures: [`docker compose emitted unparseable JSON for ${file}: ${error.message}`] };
  }

  const failures = validate(config);
  return { ok: failures.length === 0, failures, config };
}

function main(argv) {
  const index = argv.indexOf("--file");
  const file = index === -1 ? DEFAULT_FILE : resolve(argv[index + 1]);
  const result = check(file);
  if (!result.ok) {
    for (const failure of result.failures) console.error(`FAIL ${failure}`);
    return 1;
  }
  const service = result.config.services.rigcraft;
  const port = service.ports[0];
  console.log(
    `PASS ${file}: one service, local build, /api/health healthcheck, ${service.volumes[0].source} at /data, ${port.host_ip}:${port.published}->${port.target}, read_only`,
  );
  return 0;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  process.exitCode = main(process.argv.slice(2));
}

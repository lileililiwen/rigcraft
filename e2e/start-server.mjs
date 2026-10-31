#!/usr/bin/env node
// Playwright `webServer` entry point.
//
// Starts the built service on the suite's own port with a fresh scratch data
// directory under the OS temporary directory, so a run never reads or writes a
// developer's inventory. The directory is removed when the service exits. The
// binary is not built here: `cargo build` is the documented prerequisite and a
// missing binary is a named failure rather than a silent rebuild.
import { spawn } from "node:child_process";
import { existsSync, mkdirSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const port = process.env.RIGCRAFT_E2E_PORT;
if (!port) {
  console.error("RIGCRAFT_E2E_PORT is required; run `npm run test:e2e`.");
  process.exit(1);
}

const binary = join(process.cwd(), "target", "debug", "rigcraft");
if (!existsSync(binary)) {
  console.error(
    `Cannot start the browser suite: ${binary} is missing. Build the service first: cargo build.`,
  );
  process.exit(1);
}

const bundle = join(process.cwd(), "apps", "web", "dist", "index.html");
if (!existsSync(bundle)) {
  console.error(
    `Cannot start the browser suite: ${bundle} is missing. Build the Web app first: npm run build.`,
  );
  process.exit(1);
}

// One directory per port: a run that was killed without cleanup leaves at most
// one directory, and the next run on that port removes it before starting.
const dataDir = join(tmpdir(), `rigcraft-e2e-${port}`);
rmSync(dataDir, { recursive: true, force: true });
mkdirSync(dataDir, { recursive: true });
let cleaned = false;
function cleanup() {
  if (cleaned) {
    return;
  }
  cleaned = true;
  rmSync(dataDir, { recursive: true, force: true });
}

const child = spawn(binary, [], {
  stdio: "inherit",
  env: {
    ...process.env,
    RIGCRAFT_ADDR: `127.0.0.1:${port}`,
    RIGCRAFT_DATA_DIR: dataDir,
  },
});

for (const signal of ["SIGINT", "SIGTERM"]) {
  process.on(signal, () => child.kill(signal));
}
process.on("exit", cleanup);

child.on("error", (error) => {
  console.error(`Cannot start ${binary}: ${error.message}`);
  cleanup();
  process.exit(1);
});
child.on("exit", (code, signal) => {
  cleanup();
  process.exit(code ?? (signal ? 0 : 1));
});

#!/usr/bin/env node
// `npm run test:e2e` entry point.
//
// Selects a free loopback port, hands it to Playwright (the config and the
// `webServer` command both read RIGCRAFT_E2E_PORT), and forwards extra arguments
// to the Playwright CLI. Nothing here builds the app: `cargo build`, `npm run
// build` and `npm ci` are documented prerequisites.
import { spawn } from "node:child_process";
import { rmSync } from "node:fs";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

function freePort() {
  return new Promise((resolve, reject) => {
    const server = createServer();
    server.unref();
    server.on("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      const port = typeof address === "object" && address ? address.port : 0;
      server.close(() => (port ? resolve(port) : reject(new Error("no free port"))));
    });
  });
}

const port = process.env.RIGCRAFT_E2E_PORT ?? String(await freePort());
const dataDir = join(tmpdir(), `rigcraft-e2e-${port}`);
const cli = fileURLToPath(new URL("../node_modules/@playwright/test/cli.js", import.meta.url));

function cleanup() {
  rmSync(dataDir, { recursive: true, force: true });
}

console.log(`Browser suite on 127.0.0.1:${port} (scratch data directory, removed after the run)`);

const child = spawn(process.execPath, [cli, "test", ...process.argv.slice(2)], {
  stdio: "inherit",
  env: { ...process.env, RIGCRAFT_E2E_PORT: port },
});

for (const signal of ["SIGINT", "SIGTERM"]) {
  process.on(signal, () => {
    child.kill(signal);
    cleanup();
  });
}
process.on("exit", cleanup);

child.on("exit", (code, signal) => {
  cleanup();
  process.exit(code ?? (signal ? 1 : 0));
});

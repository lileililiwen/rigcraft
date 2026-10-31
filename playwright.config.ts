import { defineConfig, devices } from "@playwright/test";

/**
 * The browser suite drives the bundle the Rust service serves. `npm run test:e2e`
 * (see `e2e/run.mjs`) picks a free loopback port and exports it here; running
 * `npx playwright test` directly falls back to 8137. Port 8080 belongs to an
 * unrelated tool on this host and is never used by the suite.
 */
const port = Number(process.env.RIGCRAFT_E2E_PORT ?? "8137");
const baseURL = `http://127.0.0.1:${port}`;

export default defineConfig({
  testDir: "./e2e",
  outputDir: "./test-results",
  fullyParallel: false,
  // The tests share one service and one SQLite file. Each owns its snapshot, but
  // calling the run parallel would claim a data isolation that does not exist.
  workers: 1,
  retries: 0,
  // A stray `.only` must fail everywhere, not only on CI.
  forbidOnly: true,
  timeout: 30_000,
  expect: { timeout: 10_000 },
  reporter: [
    ["list"],
    ["json", { outputFile: "test-results/e2e-results.json" }],
  ],
  use: {
    baseURL,
    channel: "chrome",
    headless: true,
    locale: "en-US",
    timezoneId: "UTC",
    acceptDownloads: true,
    trace: "on-first-retry",
    screenshot: "only-on-failure",
  },
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"], channel: "chrome" },
    },
  ],
  webServer: {
    command: "node e2e/start-server.mjs",
    url: `${baseURL}/api/health`,
    env: { RIGCRAFT_E2E_PORT: String(port) },
    reuseExistingServer: false,
    timeout: 60_000,
  },
});

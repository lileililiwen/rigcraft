import { expect, test } from "@playwright/test";
import { chooseSnapshot, expectNoHorizontalOverflow, seedSample } from "./fixtures";

/**
 * The cross-browser smoke subset. It proves the six client routes, the skip
 * link, phone-width layout, the real 404 and two seeded journeys outside Chrome,
 * through the same fixtures and the same service as the full suite.
 *
 * It runs in the chromium project with every other journey, and as the only spec
 * in the firefox and webkit projects. It deliberately carries no pixel baseline:
 * there is one committed baseline set and it belongs to Chrome.
 */

const ROUTES = [
  { path: "/", title: "RigCraft inventory", heading: "Inventory" },
  { path: "/inventory", title: "RigCraft inventory", heading: "Inventory" },
  { path: "/catalog", title: "RigCraft catalog", heading: "Catalog" },
  {
    path: "/builds/check",
    title: "RigCraft compatibility check",
    heading: "Compatibility check",
  },
  {
    path: "/recommend",
    title: "RigCraft build recommendation",
    heading: "Build recommendation",
  },
  {
    path: "/diagnose",
    title: "RigCraft build diagnosis",
    heading: "Diagnose an existing build",
  },
];

test("every client route serves the shell with its own title", async ({ page }) => {
  for (const route of ROUTES) {
    const response = await page.goto(route.path);
    expect(response?.status(), `GET ${route.path}`).toBe(200);
    await expect(page.locator("#root"), `${route.path} serves the shell`).toHaveCount(1);
    await expect(page, `${route.path} sets its title`).toHaveTitle(route.title);
    await expect(
      page.getByRole("heading", { name: route.heading, level: 1 }),
      `${route.path} renders its page heading`,
    ).toBeVisible();
    await expect(page.locator("main#main")).toBeVisible();
  }
});

test("the skip link is the first Tab stop and moves focus to main", async ({ page }) => {
  await page.goto("/inventory");

  await page.keyboard.press("Tab");
  const skip = page.getByRole("link", { name: /^Skip to inventory$/ });
  await expect(skip, "the first Tab stop is the skip link").toBeFocused();

  await page.keyboard.press("Enter");
  await expect(page.locator("main#main"), "the skip link focuses the main region").toBeFocused();
});

test("no client route overflows horizontally at 390 px", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });

  for (const route of ROUTES) {
    await page.goto(route.path);
    await expect(page.getByRole("heading", { name: route.heading, level: 1 })).toBeVisible();
    await expectNoHorizontalOverflow(page);
  }
});

test("a typo path is the real 404 document, not the shell", async ({ page }) => {
  const response = await page.goto("/recomend");
  expect(response?.status(), "GET /recomend").toBe(404);
  expect(response?.headers()["content-type"]).toContain("text/plain");
  await expect(page.locator("body")).toContainText("404 Not found.");
  await expect(page.locator("#root"), "the 404 is not the SPA shell").toHaveCount(0);
});

test("a recommendation generates from the seeded sample", async ({ page, request }) => {
  const snapshot = await seedSample(request);

  await page.goto("/recommend");
  await chooseSnapshot(page, "#recommend-snapshot", snapshot);
  await page.locator("#recommend-budget").fill("1800");
  await page.getByRole("button", { name: "Generate recommendations" }).click();

  await expect(page.getByRole("heading", { name: "Recommended builds" })).toBeVisible();
  await expect(page.locator("[role='status']").first()).toContainText(
    "Recommendation complete",
  );
  await expect(page.locator("article.candidate").first()).toContainText("Best overall");
  await expect(
    page.getByRole("heading", { name: "Gaming requires a discrete graphics card" }),
  ).toBeVisible();
});

test("the inventory export downloads with its revision-bearing filename", async ({
  page,
  request,
}) => {
  await seedSample(request);

  await page.goto("/inventory");
  const csvButton = page.getByRole("button", { name: "Download CSV" });
  await expect(csvButton).toBeEnabled();

  const download = page.waitForEvent("download");
  await csvButton.click();
  expect((await download).suggestedFilename()).toMatch(
    /^rigcraft-inventory-snapshot-r1-.*\.csv$/,
  );
});

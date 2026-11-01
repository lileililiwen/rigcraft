import { expect, test } from "@playwright/test";
import { chooseSnapshot, seedSample } from "./fixtures";

/**
 * The accessible names this change added, proved on the real pages rather than
 * in a component test. A `<form>` is exposed with role `form` only when it has an
 * accessible name, so comparing the named form landmarks with the raw `form`
 * elements also proves that no form is left unnamed.
 */

const ROUTES = [
  { path: "/", name: "Inventory" },
  { path: "/inventory", name: "Inventory" },
  { path: "/catalog", name: "Catalog" },
  { path: "/builds/check", name: "Compatibility check" },
  { path: "/recommend", name: "Build recommendation" },
  { path: "/diagnose", name: "Diagnose an existing build" },
];

test("every route names its single main region and no form is unnamed", async ({
  page,
}) => {
  for (const route of ROUTES) {
    await page.goto(route.path);
    // The page fetches its snapshot state after paint; wait for that request to
    // finish so a form is counted in the state the operator actually reaches.
    await page.waitForLoadState("networkidle");
    await expect(page.locator("main"), `${route.path} renders one main`).toHaveCount(1);
    await expect(
      page.getByRole("main", { name: route.name }),
      `${route.path} names its main region`,
    ).toHaveCount(1);

    // A `<form>` is exposed with role `form` only when it has an accessible
    // name, so the difference is the unnamed forms. Content inside the collapsed
    // assistant disclosure is not rendered and not yet in the tree; that form is
    // asserted after the disclosure is opened in the test below.
    const unnamed =
      (await page.locator("form:visible").count()) - (await page.getByRole("form").count());
    expect(unnamed, `${route.path} has no unnamed form`).toBe(0);
  }
});

test("the import, catalog and restore forms are named landmarks", async ({ page }) => {
  await page.goto("/inventory");
  await expect(page.getByRole("form", { name: "Import a CSV" })).toBeVisible();
  await expect(page.getByRole("form", { name: "Restore from a backup" })).toBeVisible();

  await page.goto("/catalog");
  await expect(page.getByRole("form", { name: "Upload a catalog" })).toBeVisible();
  await expect(page.getByRole("form", { name: "Search" })).toBeVisible();
});

test("the budget, assistant and parts forms are named once a snapshot loads", async ({
  page,
  request,
}) => {
  const snapshot = await seedSample(request);

  await page.goto("/recommend");
  await chooseSnapshot(page, "#recommend-snapshot", snapshot);
  await expect(page.getByRole("form", { name: "3. Budget and constraints" })).toBeVisible();

  // The transport form is inside the collapsed assistant, so it enters the
  // accessibility tree only once the disclosure is opened.
  await page.getByText("2. Ask in your own words (optional)").click();
  await expect(
    page.getByRole("form", { name: "Assistant provider settings" }),
  ).toBeVisible();

  await page.goto("/builds/check");
  await chooseSnapshot(page, "#check-snapshot", snapshot);
  await expect(page.getByRole("form", { name: "2. Select parts" })).toBeVisible();
});

test("the skip link still lands in the named main region", async ({ page }) => {
  await page.goto("/inventory");
  // Wait for the SPA to render and make this the frontmost target, so the first
  // key event cannot be delivered to the page that is still closing. The
  // assertion itself is unchanged: no retry, no looser condition.
  await expect(page.getByRole("heading", { name: "Inventory", level: 1 })).toBeVisible();
  await page.bringToFront();

  await page.keyboard.press("Tab");
  const skip = page.getByRole("link", { name: "Skip to inventory" });
  await expect(skip, "the first Tab stop is the skip link").toBeFocused();

  await page.keyboard.press("Enter");
  await expect(
    page.getByRole("main", { name: "Inventory" }),
    "the skip link focuses the named main region",
  ).toBeFocused();
});

import { expect, test } from "@playwright/test";
import { expectNoHorizontalOverflow, seedSample } from "./fixtures";

test.use({ viewport: { width: 390, height: 844 } });

/**
 * One journey completed from the keyboard alone. Unit tests cannot see focus
 * order or a form that only submits on Enter, and a pointer-driven test would
 * hide a broken tab order.
 */
test("a recommendation is generated from the keyboard alone", async ({ page, request }) => {
  const snapshot = await seedSample(request);

  await page.goto("/recommend");
  // The page selects the newest snapshot; prove it is this test's own.
  await expect(page.locator("#recommend-snapshot")).toHaveValue(snapshot.id);

  await page.keyboard.press("Tab");
  const firstStop = await page.evaluate(() => ({
    className: document.activeElement?.className ?? "",
    text: document.activeElement?.textContent?.trim() ?? "",
  }));
  expect(firstStop.className).toContain("skip-link");
  expect(firstStop.text).toBe("Skip to recommendation");

  let budgetReached = false;
  for (let presses = 0; presses < 80; presses += 1) {
    const activeId = await page.evaluate(() => document.activeElement?.id ?? "");
    if (activeId === "recommend-budget") {
      budgetReached = true;
      break;
    }
    await page.keyboard.press("Tab");
  }
  expect(budgetReached, "Tab reaches the budget field").toBe(true);

  await page.keyboard.type("1800");
  await page.keyboard.press("Enter");

  await expect(page.getByRole("heading", { name: "Recommended builds" })).toBeVisible();
  await expect(page.locator("article.candidate").first()).toContainText("Best overall");
  await expectNoHorizontalOverflow(page);
});

test("no page overflows horizontally at 390 px", async ({ page, request }) => {
  await seedSample(request);

  const pages: Array<{ path: string; heading: string; ready: string }> = [
    { path: "/inventory", heading: "Inventory", ready: "Import a CSV" },
    { path: "/builds/check", heading: "Compatibility check", ready: "Snapshot" },
    { path: "/recommend", heading: "Build recommendation", ready: "Snapshot" },
    { path: "/diagnose", heading: "Diagnose an existing build", ready: "Snapshot" },
  ];

  for (const entry of pages) {
    await page.goto(entry.path);
    await expect(page.getByRole("heading", { name: entry.heading, level: 1 })).toBeVisible();
    await expect(page.getByText(entry.ready).first()).toBeVisible();

    const width = await page.evaluate(() => ({
      scrollWidth: document.documentElement.scrollWidth,
      clientWidth: document.documentElement.clientWidth,
    }));
    expect(width.clientWidth, `${entry.path} renders at 390 px`).toBe(390);
    expect(width.scrollWidth, `${entry.path} must not overflow`).toBe(width.clientWidth);
  }
});

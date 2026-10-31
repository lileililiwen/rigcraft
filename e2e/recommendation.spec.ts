import { expect, test } from "@playwright/test";
import { chooseSnapshot, expectNoHorizontalOverflow, moneyToMinor, seedSample } from "./fixtures";

test("a budget that fits returns a ranked option below budget with an honest status", async ({
  page,
  request,
}) => {
  const snapshot = await seedSample(request);

  await page.goto("/recommend");
  await chooseSnapshot(page, "#recommend-snapshot", snapshot);
  await page.locator("#recommend-budget").fill("1800");
  await page.getByRole("button", { name: "Generate recommendations" }).click();

  await expect(page.getByRole("heading", { name: "Recommended builds" })).toBeVisible();
  await expect(page.locator(".status-line")).toContainText("Recommendation complete");

  const summary = page.locator("dl.result-summary").first();
  await expect(summary.locator("dd").nth(0)).toHaveText("3");
  await expect(summary.locator("dd").nth(2)).toHaveText("5");
  await expect(summary.locator("dd").nth(3)).toHaveText("Solved");

  const best = page.locator("article.candidate").first();
  await expect(best.locator("h3")).toContainText("1.");
  await expect(best.locator("h3")).toContainText("Best overall");
  const totalMinor = moneyToMinor(await best.locator("dl.summary-grid dd").first().textContent());
  expect(totalMinor).toBeGreaterThan(0);
  expect(totalMinor, "the ranked total is under the requested budget").toBeLessThan(180000);

  // The two unknown rules keep the option labelled Unknown, not Compatible.
  const badge = best.locator(".badge--verdict");
  await expect(badge).toContainText("Unknown");
  await expect(badge).not.toContainText("Compatible");
  await expect(best.locator(".diagnostic-block--unknown")).toContainText(
    "Confirm before building",
  );
  await expect(best).toContainText("installed BIOS version supports AMD Ryzen 7 7800X3D");

  // Every option is ranked and compared side by side.
  await expect(page.locator("article.candidate")).toHaveCount(3);
  await expect(page.locator("table.compare-table thead th")).toHaveCount(4);

  await expectNoHorizontalOverflow(page);
});

test("a budget below the cheapest complete build names the budget as the constraint", async ({
  page,
  request,
}) => {
  const snapshot = await seedSample(request);

  await page.goto("/recommend");
  await chooseSnapshot(page, "#recommend-snapshot", snapshot);
  await page.locator("#recommend-budget").fill("900");
  await page.getByRole("button", { name: "Generate recommendations" }).click();

  await expect(page.getByRole("heading", { name: "Recommended builds" })).toBeVisible();
  await expect(page.locator(".status-line")).toContainText(
    "No solution. The budget is below the cheapest complete build.",
  );

  const summary = page.locator("dl.result-summary").first();
  await expect(summary.locator("dd").nth(0)).toHaveText("0");
  await expect(summary.locator("dd").nth(3)).toHaveText("No solution");

  // The binding constraint is budget, and the minimum is stated rather than implied.
  const noSolution = page.locator(".diagnostic-block--conflict").filter({
    has: page.getByRole("heading", { name: "Budget constraint" }),
  });
  await expect(noSolution).toContainText("The budget is below the cheapest complete build.");
  await expect(noSolution).toContainText("1,123.90 USD");
  await expect(noSolution).toContainText("900.00 USD");
  await expect(noSolution).toContainText("Raise the budget to at least 1,123.90 USD.");
  await expect(page.locator("article.candidate")).toHaveCount(0);

  await expectNoHorizontalOverflow(page);
});

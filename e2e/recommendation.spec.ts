import { expect, test } from "@playwright/test";
import { chooseSnapshot, expectNoHorizontalOverflow, moneyToMinor, seedSample } from "./fixtures";

test("a gaming request includes a discrete graphics card and says why", async ({
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
  await expect(summary.locator("dd").nth(0)).toHaveText("1");
  await expect(summary.locator("dd").nth(2)).toHaveText("2");
  await expect(summary.locator("dd").nth(3)).toHaveText("Solved");

  // A gaming build now carries the card, not just the CPU's igpu=yes fact.
  await expect(
    page.getByRole("heading", { name: "Gaming requires a discrete graphics card" }),
  ).toBeVisible();
  await expect(page.locator('[data-use-case-rule="gaming"]')).toContainText(
    "Gaming requires a discrete graphics card in every option.",
  );

  const best = page.locator("article.candidate").first();
  await expect(best.locator("h3")).toContainText("1.");
  await expect(best.locator("h3")).toContainText("Best overall");
  await expect(
    best.getByRole("row", { name: /NVIDIA GeForce RTX 4070 Founders Edition/ }),
  ).toBeVisible();
  await expect(best).toContainText("Gaming requires a discrete graphics card");
  const totalMinor = moneyToMinor(await best.locator("dl.summary-grid dd").first().textContent());
  expect(totalMinor).toBeGreaterThan(0);
  expect(totalMinor, "the GPU-inclusive total is under the requested budget").toBeLessThan(
    180000,
  );

  // The two unknown rules keep the option labelled Unknown, not Compatible.
  const badge = best.locator(".badge--verdict");
  await expect(badge).toContainText("Unknown");
  await expect(badge).not.toContainText("Compatible");
  await expect(best.locator(".diagnostic-block--unknown")).toContainText(
    "Confirm before building",
  );
  await expect(best).toContainText("installed BIOS version supports AMD Ryzen 7 7800X3D");

  await expectNoHorizontalOverflow(page);
});

test("a budget below the cheapest GPU build names the graphics card", async ({
  page,
  request,
}) => {
  const snapshot = await seedSample(request);

  await page.goto("/recommend");
  await chooseSnapshot(page, "#recommend-snapshot", snapshot);
  await page.locator("#recommend-budget").fill("1500");
  await page.getByRole("button", { name: "Generate recommendations" }).click();

  await expect(page.getByRole("heading", { name: "Recommended builds" })).toBeVisible();
  await expect(page.locator(".status-line")).toContainText(
    "No solution. The budget is below the cheapest gaming build with a discrete graphics card.",
  );

  const summary = page.locator("dl.result-summary").first();
  await expect(summary.locator("dd").nth(0)).toHaveText("0");
  await expect(summary.locator("dd").nth(3)).toHaveText("No solution");

  // 1,500.00 is enough for the iGPU-only machine, but gaming needs the card.
  const noSolution = page.locator(".diagnostic-block--conflict").filter({
    has: page.getByRole("heading", { name: "Budget constraint" }),
  });
  await expect(noSolution).toContainText(
    "The budget is below the cheapest gaming build with a discrete graphics card.",
  );
  await expect(noSolution).toContainText("requires a discrete graphics card");
  await expect(noSolution).toContainText("1,672.90 USD");
  await expect(noSolution).toContainText("172.90 USD");
  await expect(noSolution).toContainText("Raise the budget to at least 1,672.90 USD.");
  await expect(noSolution).toContainText("Cheapest gpu");
  await expect(page.locator("article.candidate")).toHaveCount(0);

  await expectNoHorizontalOverflow(page);
});

test("one materially different build is not padded to three", async ({ page, request }) => {
  const snapshot = await seedSample(request);

  await page.goto("/recommend");
  await chooseSnapshot(page, "#recommend-snapshot", snapshot);
  await page.locator("#recommend-budget").fill("1900");
  await page.getByRole("button", { name: "Generate recommendations" }).click();

  await expect(page.getByRole("heading", { name: "Recommended builds" })).toBeVisible();
  await expect(page.locator(".status-line")).toContainText(
    "Recommendation complete: 1 materially different option",
  );

  // All three drives fit at 1,900.00, but a larger drive is the same machine.
  await expect(page.locator("article.candidate")).toHaveCount(1);
  await expect(page.getByText(/1 materially different build found/)).toBeVisible();
  await expect(page.getByText(/no option was padded to reach three/)).toBeVisible();
  await expect(page.locator(".compare-table thead th")).toHaveCount(2);

  await expectNoHorizontalOverflow(page);
});

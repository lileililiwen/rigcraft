import { expect, test, type Page } from "@playwright/test";
import { chooseSnapshot, expectNoHorizontalOverflow, seedSample } from "./fixtures";

type Box = { x: number; y: number; width: number };

async function regionBoxes(page: Page) {
  return page.evaluate<{
    constraints: Box;
    options: Box;
    verdict: Box;
    main: Box;
  }>(() => {
    const box = (selector: string): Box => {
      const element = document.querySelector(selector);
      if (!element) {
        throw new Error(`${selector} is missing`);
      }
      const rect = element.getBoundingClientRect();
      return { x: rect.x, y: rect.y, width: rect.width };
    };
    return {
      constraints: box(".workbench__constraints"),
      options: box(".workbench__options"),
      verdict: box(".workbench__verdict"),
      main: box("main"),
    };
  });
}

/**
 * The workbench layout: constraints, options and verdict as three columns at
 * 1100 px and wider, one column below, and no horizontal overflow at any width
 * the audit visits. The same generated report is measured at every width, so a
 * failure names the width rather than an unrelated state change.
 */
test("the recommendation is a three-region workbench with no overflow at any audited width", async ({
  page,
  request,
}) => {
  const snapshot = await seedSample(request);

  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto("/recommend");
  await chooseSnapshot(page, "#recommend-snapshot", snapshot);

  // A grouped budget is accepted without a submit: the text is what an operator
  // writes, and the parser is what turns it into minor units.
  await page.locator("#recommend-budget").fill("1,800");
  const generate = page.getByRole("button", { name: "Generate recommendations" });
  await expect(generate).toBeEnabled();
  await generate.click();

  await expect(page.getByRole("heading", { name: "Recommended builds" })).toBeVisible();

  const wide = await regionBoxes(page);
  expect(wide.constraints.x, "constraints is the first column").toBeLessThan(
    wide.options.x,
  );
  expect(wide.options.x, "options is the middle column").toBeLessThan(wide.verdict.x);
  expect(Math.abs(wide.constraints.y - wide.verdict.y)).toBeLessThanOrEqual(1);
  expect(wide.main.width, "the workbench is capped at 88 rem").toBeLessThanOrEqual(1408);
  await expectNoHorizontalOverflow(page);

  // Exactly at the breakpoint the three columns are already in place.
  await page.setViewportSize({ width: 1100, height: 900 });
  const boundary = await regionBoxes(page);
  expect(boundary.constraints.x).toBeLessThan(boundary.options.x);
  expect(boundary.options.x).toBeLessThan(boundary.verdict.x);
  await expectNoHorizontalOverflow(page);

  // One pixel below it, the same three regions stack in source order.
  await page.setViewportSize({ width: 390, height: 844 });
  const narrow = await regionBoxes(page);
  expect(narrow.constraints.y, "constraints stacks first").toBeLessThan(narrow.options.y);
  expect(narrow.options.y, "options follows constraints").toBeLessThan(narrow.verdict.y);
  expect(narrow.constraints.width).toBeCloseTo(narrow.options.width, 0);
  expect(narrow.constraints.width).toBeCloseTo(narrow.verdict.width, 0);
  await expectNoHorizontalOverflow(page);

  for (const width of [320, 768]) {
    await page.setViewportSize({ width, height: 844 });
    const stacked = await regionBoxes(page);
    expect(stacked.constraints.y, `${width}px stacks`).toBeLessThan(stacked.options.y);
    await expectNoHorizontalOverflow(page);
  }
});

test("the verdict rail and spec modules carry the decision without dropping a fact", async ({
  page,
  request,
}) => {
  const snapshot = await seedSample(request);

  await page.goto("/recommend");
  await chooseSnapshot(page, "#recommend-snapshot", snapshot);
  await page.locator("#recommend-budget").fill("1,800");
  await page.getByRole("button", { name: "Generate recommendations" }).click();
  await expect(page.getByRole("heading", { name: "Recommended builds" })).toBeVisible();

  const rail = page.locator(".workbench__verdict");
  await expect(rail.getByRole("heading", { name: "Verdict and score" })).toBeVisible();
  await expect(rail.locator("dl.result-summary dd").nth(0)).toHaveText("1");
  await expect(rail.locator("dl.result-summary dd").nth(2)).toHaveText("2");
  await expect(rail.locator("dl.result-summary dd").nth(3)).toHaveText("Solved");
  await expect(rail).toContainText("2 valid candidates");
  await expect(rail).toContainText("1 distinct build");
  await expect(
    rail.getByRole("heading", { name: "Gaming requires a discrete graphics card" }),
  ).toBeVisible();
  await expect(rail).toContainText("1 materially different build found");
  await expect(rail).toContainText("no option was padded to reach three");

  const module = page.locator("article.spec-module").first();
  await expect(module.locator(".caps-label")).toHaveAttribute("aria-hidden", "true");
  await expect(module.locator("h3")).toContainText("Best overall");
  await expect(module.locator("dl.spec-rows dd").first()).toContainText("US$");
  await expect(module.locator(".badge--verdict")).toContainText("Unknown");
  await expect(
    module.getByRole("row", { name: /NVIDIA GeForce RTX 4070 Founders Edition/ }),
  ).toBeVisible();

  // The optional assistant is a closed native disclosure until it is used.
  const assistant = page.locator("details.assistant");
  await expect(assistant).not.toHaveAttribute("open", /.*/);
  await assistant.locator("summary").click();
  await expect(assistant).toHaveAttribute("open", /.*/);
  await expect(assistant.getByLabel("Provider key")).toHaveAttribute("type", "password");

  await expectNoHorizontalOverflow(page);
});

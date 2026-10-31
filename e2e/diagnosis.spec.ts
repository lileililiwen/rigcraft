import { expect, test } from "@playwright/test";
import { chooseSnapshot, expectNoHorizontalOverflow, seedSample } from "./fixtures";

const PASTED = [
  "cpu: AMD Ryzen 7 7800X3D",
  "storage: Corsair MP600 PRO NH",
  "gpu: Voodoo 3dfx 5500",
].join("\n");

test("an exact, an ambiguous and an unmatched line are never coerced, and unverified parts are not praised", async ({
  page,
  request,
}) => {
  const snapshot = await seedSample(request);

  await page.goto("/diagnose");
  await chooseSnapshot(page, "#diagnose-snapshot", snapshot);

  await page.locator("#diagnose-text").fill(PASTED);
  await page.getByRole("button", { name: "Match this list" }).click();

  const lines = page.locator(".review-line");
  await expect(lines).toHaveCount(3);
  await expect(lines.nth(0)).toContainText("Matched");
  await expect(lines.nth(1)).toContainText("Ambiguous");
  await expect(lines.nth(2)).toContainText("No match");

  // The exact line preselects its single candidate...
  await expect(lines.nth(0).getByRole("radio", { checked: true })).toHaveCount(1);
  // ...the ambiguous line starts undecided, with every candidate offered...
  await expect(lines.nth(1).getByRole("radio", { checked: true })).toHaveCount(0);
  await expect(lines.nth(1).getByRole("radio")).toHaveCount(4);
  await expect(lines.nth(1)).toContainText("adds 1tb");
  await expect(lines.nth(1)).toContainText("Choose a candidate");
  // ...and the unmatched line offers only an explicit decision.
  await expect(lines.nth(2).getByRole("radio")).toHaveCount(1);

  // Diagnosing before the review is resolved is blocked, not silently guessed.
  await page.getByRole("button", { name: "Diagnose this build" }).click();
  const blocked = page.getByRole("alert");
  await expect(blocked.locator("h2")).toHaveText("Resolve the match review first");
  await expect(blocked).toContainText("Line 2 is ambiguous");
  await expect(blocked).toContainText("Line 3 has no match");
  await expect(blocked).toContainText("review_incomplete");
  await expect(page.getByRole("heading", { name: "Build diagnosis" })).toHaveCount(0);

  // Resolve both: choose one drive, leave the unmatched line out.
  await page.locator("#line-2-SSD-MP600-1TB").check();
  await page.locator("#line-3-skip").check();
  await page.getByRole("button", { name: "Diagnose this build" }).click();

  await expect(page.getByRole("heading", { name: "Build diagnosis" })).toBeVisible();
  const summary = page.locator("dl.result-summary");
  await expect(summary.locator("dd").nth(0)).toHaveText("0");
  await expect(summary.locator("dd").nth(1)).toHaveText("0");
  await expect(summary.locator("dd").nth(2)).toHaveText("0");
  await expect(summary.locator("dd").nth(3)).toHaveText("2");
  await expect(page.locator(".result-panel > p.lede")).toContainText(
    "2 parts not evaluated by any rule",
  );

  // No rule ran, so the verdict is never a green pass.
  await expect(page.locator(".verdict-line .badge--verdict")).not.toContainText("Compatible");

  // The unverified group is explicit and holds the only badges.
  const unverified = page.locator("section.advice-group").filter({
    has: page.getByRole("heading", { name: "Not verified — no rule evaluated (2)" }),
  });
  await expect(unverified.locator(".advice-card")).toHaveCount(2);
  await expect(unverified.locator(".advice-card .badge")).toHaveCount(2);
  await expect(unverified.locator(".advice-card").nth(0)).toContainText("Not verified");
  await expect(unverified.locator(".advice-card").nth(1)).toContainText("Not verified");
  // Never a check mark for a part no rule evaluated.
  await expect(unverified.locator(".badge--compatible")).toHaveCount(0);
  await expect(unverified).not.toContainText("Fine as is");
  await expect(page.locator(".advice-card--keep")).toHaveCount(0);

  // "Fine as is" stays empty rather than collecting the unverified parts.
  const keep = page.locator("section.advice-group").filter({
    has: page.getByRole("heading", { name: "Fine as is (0)" }),
  });
  await expect(keep.locator(".advice-card")).toHaveCount(0);
  await expect(keep).toContainText("No part is kept by this diagnosis.");

  await expectNoHorizontalOverflow(page);
});

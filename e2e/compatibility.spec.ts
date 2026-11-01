import { expect, test } from "@playwright/test";
import { chooseSnapshot, expectNoHorizontalOverflow, seedSample } from "./fixtures";

/**
 * The seed is one physically buildable AM5 machine whose BIOS support and peak
 * power are not stated by the manufacturer pages. The check must therefore end
 * as 12 compatible, 0 conflicts and 2 unknown — never as a green pass.
 */
test("the seeded AM5 build reports 12 compatible, 0 conflicts and 2 named unknowns", async ({
  page,
  request,
}) => {
  const snapshot = await seedSample(request);

  await page.goto("/builds/check");
  await chooseSnapshot(page, "#check-snapshot", snapshot);

  // The full build: one of each single part and one storage drive.
  for (const part of [
    /AMD Ryzen 7 7800X3D/,
    /ASUS ROG STRIX B650E-F/,
    /NVIDIA GeForce RTX 4070 Founders Edition/,
    /Corsair RM750e/,
    /Corsair 4000D AIRFLOW/,
    /Thermalright Peerless Assassin 120 SE/,
  ]) {
    await page.getByRole("radio", { name: part }).check();
  }
  await page.getByRole("checkbox", { name: /Kingston FURY Beast 32GB/ }).check();
  await page.getByRole("checkbox", { name: /Corsair MP600 PRO NH 1TB/ }).check();

  await page.getByRole("button", { name: "Check compatibility" }).click();

  await expect(page.getByRole("heading", { name: "Compatibility result" })).toBeVisible();
  await expect(page.locator(".status-line")).toContainText(
    "12 compatible · 0 conflicts · 2 unknown",
  );

  const summary = page.locator("dl.result-summary");
  await expect(summary.locator("dd").nth(0)).toHaveText("12");
  await expect(summary.locator("dd").nth(1)).toHaveText("0");
  await expect(summary.locator("dd").nth(2)).toHaveText("2");

  // The overall verdict is Unknown, not Compatible: two rules could not run.
  const verdict = page.locator(".verdict-line .badge--verdict");
  await expect(verdict).toContainText("Unknown");
  await expect(verdict).not.toContainText("Compatible");
  await expect(page.locator(".verdict-line")).toContainText("Complete build");

  // Both unknowns are named, and each states the fact an operator must confirm.
  const factsToConfirm = page.locator("section.result-panel .diagnostic-block").filter({
    has: page.getByRole("heading", { name: "Facts to confirm" }),
  });
  await expect(factsToConfirm.locator("li")).toHaveCount(2);
  await expect(factsToConfirm).toContainText("cpu-bios");
  await expect(factsToConfirm).toContainText("psu-capacity");
  await expect(factsToConfirm.locator("li").filter({ hasText: "cpu-bios" })).toContainText(
    "Confirm the motherboard's installed BIOS version supports AMD Ryzen 7 7800X3D.",
  );
  await expect(
    factsToConfirm.locator("li").filter({ hasText: "psu-capacity" }),
  ).toContainText("Confirm sourced peak or transient power draw");

  // No conflict is claimed merely because two rules are unknown.
  await expect(page.locator(".diagnostic-block--conflict")).toContainText(
    "No definite conflict was found among the rules that could run.",
  );

  // The rule matrix keeps every rule and shows the compared values with their
  // provenance reference and the follow-up, not just a verdict.
  await expect(page.getByRole("heading", { name: "Rule matrix" })).toBeVisible();
  await expect(page.locator("table.rule-matrix tbody tr")).toHaveCount(14);
  const socketRow = page
    .locator("table.rule-matrix tbody tr")
    .filter({ hasText: "cpu-socket" });
  await expect(socketRow).toContainText("Compatible");
  await expect(socketRow).toContainText("AM5");
  await expect(socketRow).toContainText("Catalog · manufacturer@2026-10-31");
  await expect(socketRow).toContainText("https://www.amd.com");
  await expect(socketRow).toContainText("No follow-up");
  const biosRow = page
    .locator("table.rule-matrix tbody tr")
    .filter({ hasText: "cpu-bios" });
  await expect(biosRow).toContainText(
    "Confirm the motherboard's installed BIOS version supports AMD Ryzen 7 7800X3D.",
  );

  await expectNoHorizontalOverflow(page);
});

/**
 * An incomplete selection names the gap once: the reason is a sentence and the
 * follow-up is an instruction, never a repeat of "is required".
 */
test("a CPU-only check states the motherboard gap without duplicating it", async ({
  page,
  request,
}) => {
  const snapshot = await seedSample(request);

  await page.goto("/builds/check");
  await chooseSnapshot(page, "#check-snapshot", snapshot);
  await page.getByRole("radio", { name: /AMD Ryzen 7 7800X3D/ }).check();
  await page.getByRole("button", { name: "Check compatibility" }).click();

  await expect(page.getByRole("heading", { name: "Compatibility result" })).toBeVisible();
  const gaps = page
    .locator(".diagnostic-block")
    .filter({ has: page.getByRole("heading", { name: "Incomplete build" }) });
  await expect(gaps).toContainText("A motherboard is required.");
  await expect(gaps).toContainText("Select a motherboard from this snapshot.");
  await expect(gaps).not.toContainText("is required from this snapshot");
  await expect(gaps).toContainText("A power supply is required.");

  await expect(page.getByRole("heading", { name: "Rule matrix" })).toBeVisible();
  await expect(page.locator("dl.result-summary dd").nth(3)).toHaveText("1");

  await expectNoHorizontalOverflow(page);
});

import { readFileSync } from "node:fs";
import { expect, test } from "@playwright/test";
import {
  CATALOG_SOURCE,
  CPU_URL,
  SAMPLE_FILENAME,
  expectNoHorizontalOverflow,
} from "./fixtures";

/**
 * The operator's whole import path in a real browser. The sample is downloaded
 * from the service, loaded through the file input, previewed and committed, so a
 * regression in the download, the mapping, the review table or the commit state
 * fails here and not only in Vitest.
 */
test("the sample imports, commits revision 1 and keeps catalog facts apart from operator facts", async ({
  page,
}) => {
  await page.goto("/inventory");
  await expect(page.getByRole("heading", { name: "Inventory", level: 1 })).toBeVisible();

  // Download the sample the way an operator does, and read what was served.
  const downloading = page.waitForEvent("download");
  await page.getByRole("link", { name: "Download sample CSV" }).click();
  const download = await downloading;
  expect(download.suggestedFilename()).toBe(SAMPLE_FILENAME);
  const csvPath = test.info().outputPath(SAMPLE_FILENAME);
  await download.saveAs(csvPath);
  const csv = readFileSync(csvPath, "utf8");
  const lines = csv.trim().split("\n");
  expect(lines[0].split(",")).toEqual([
    "sku",
    "category",
    "model",
    "quantity",
    "price",
    "currency",
    "specs",
    "source",
    "sourceUrl",
  ]);
  expect(lines, "one header plus ten seeded rows").toHaveLength(11);
  expect(csv).toContain(`catalog:${CATALOG_SOURCE}`);
  expect(csv).toContain(CPU_URL);

  await expect(page.locator("#sample-licence")).toContainText("CC BY 4.0");
  await expect(page.locator("#sample-licence")).toContainText("operator-entered");

  // Load it, preview it, commit it.
  await page.locator("#inventory-file").setInputFiles(csvPath);
  await expect(page.locator("#file-hint")).toContainText(`Selected ${SAMPLE_FILENAME}`);
  await page.getByRole("button", { name: "Preview CSV" }).click();

  await expect(page.getByRole("heading", { name: "Review import" })).toBeVisible();
  await expect(
    page.locator("p.lede").filter({ hasText: SAMPLE_FILENAME }),
  ).toContainText("10 rows · 0 errors · 0 duplicates");
  const previewSummary = page.locator("dl.summary-grid dd");
  await expect(previewSummary.nth(0)).toHaveText("10");
  await expect(previewSummary.nth(1)).toHaveText("10");
  await expect(previewSummary.nth(2)).toHaveText("0");
  await expect(previewSummary.nth(3)).toHaveText("0");
  await expect(previewSummary.nth(4)).toHaveText("10");
  const previewTable = page.locator("table.responsive-table");
  await expect(previewTable.locator("tbody tr")).toHaveCount(10);
  const previewCpu = previewTable.locator("tbody tr").filter({ hasText: "CPU-7800X3D" });
  await expect(previewCpu).toContainText(`Catalog facts · ${CATALOG_SOURCE}`);

  await page.getByRole("button", { name: "Commit snapshot" }).click();

  // Revision 1 is the snapshot this import created, with all ten records.
  await expect(
    page.getByRole("heading", { name: "Committed snapshot · revision 1", level: 2 }),
  ).toBeVisible();
  const snapshotTable = page.locator("table.responsive-table");
  await expect(snapshotTable.locator("tbody tr")).toHaveCount(10);
  await expect(page.locator("dl.summary-grid dd").nth(0)).toHaveText("10");
  await expect(page.locator("dl.summary-grid dd").nth(1)).toHaveText("10");
  for (const sku of [
    "CPU-7800X3D",
    "MB-B650E-F",
    "RAM-FURY-32-6000",
    "GPU-4070-FE",
    "PSU-RM750E",
    "CASE-4000D",
    "COOL-PA120SE",
    "SSD-MP600-1TB",
    "SSD-MP600-2TB",
    "SSD-MP600-4TB",
  ]) {
    await expect(snapshotTable).toContainText(sku);
  }

  // The committed timestamp is ISO 8601 in UTC, with no locale formatting.
  const committed = page.locator("dl.record-meta time");
  await expect(committed).toHaveText(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z$/);
  const committedText = (await committed.textContent()) ?? "";
  const committedAttribute = (await committed.getAttribute("datetime")) ?? "";
  expect(committedText).not.toContain(",");
  expect(Date.parse(committedText)).toBe(Date.parse(committedAttribute));

  // The provenance split: catalog facts for the part, operator facts for stock.
  const cpuRow = snapshotTable.locator("tbody tr").filter({ hasText: "CPU-7800X3D" });
  await expect(cpuRow).toContainText(`Catalog facts · ${CATALOG_SOURCE}`);
  await cpuRow.locator("summary").click();
  const provenance = cpuRow.locator("ul.provenance-list");
  const entry = (field: string) => provenance.locator("li").filter({ hasText: field });

  await expect(entry("model:")).toContainText(`Catalog · ${CATALOG_SOURCE}`);
  await expect(entry("model:")).toContainText(CPU_URL);
  await expect(entry("specs:")).toContainText(`Catalog · ${CATALOG_SOURCE}`);
  await expect(entry("specs:")).toContainText(CPU_URL);

  await expect(entry("quantity:")).toContainText("User-provided");
  await expect(entry("quantity:")).toContainText(`csv:${SAMPLE_FILENAME}`);
  await expect(entry("price:")).toContainText("User-provided");
  await expect(entry("price:")).toContainText(`csv:${SAMPLE_FILENAME}`);
  await expect(entry("sku:")).toContainText("User-provided");
  await expect(entry("sku:")).toContainText(`csv:${SAMPLE_FILENAME}`);

  // Stock values are never claimed as catalog facts.
  await expect(entry("quantity:")).not.toContainText("Catalog");
  await expect(entry("price:")).not.toContainText("Catalog");

  await expectNoHorizontalOverflow(page);
});

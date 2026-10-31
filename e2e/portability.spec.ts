import { readFileSync } from "node:fs";
import { expect, test } from "@playwright/test";
import { CATALOG_SOURCE, seedSample } from "./fixtures";

/**
 * The portability journeys a shop actually needs: get the catalog out in both
 * formats, put the CSV back, download a whole-database backup and restore it
 * only after an explicit inspection and typed confirmation.
 */
test("a snapshot exports to CSV and JSON, and the CSV re-imports with provenance intact", async ({
  page,
}) => {
  const seeded = await seedSample(page.request);
  await page.goto("/inventory");
  await page.getByRole("button", { name: /^View / }).first().click();
  await expect(
    page.getByRole("heading", { name: /Committed snapshot · revision/ }),
  ).toBeVisible();

  const csvDownload = page.waitForEvent("download");
  await page.getByRole("button", { name: "Download CSV" }).click();
  const csv = await csvDownload;
  expect(csv.suggestedFilename()).toMatch(
    new RegExp(`^rigcraft-inventory-snapshot-r${seeded.revision}-.*\\.csv$`),
  );
  const csvPath = test.info().outputPath("exported-inventory.csv");
  await csv.saveAs(csvPath);
  const csvText = readFileSync(csvPath, "utf8");
  const csvLines = csvText.trim().split("\n");
  expect(csvLines[0].split(",")).toEqual([
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
  expect(csvLines, "one header plus ten lots").toHaveLength(11);
  expect(csvText).toContain(`catalog:${CATALOG_SOURCE}`);

  const jsonDownload = page.waitForEvent("download");
  await page.getByRole("button", { name: "Download JSON" }).click();
  const jsonFile = await jsonDownload;
  expect(jsonFile.suggestedFilename()).toMatch(
    new RegExp(`^rigcraft-inventory-snapshot-r${seeded.revision}-.*\\.json$`),
  );
  const jsonPath = test.info().outputPath("exported-inventory.json");
  await jsonFile.saveAs(jsonPath);
  const exportBody = JSON.parse(readFileSync(jsonPath, "utf8"));
  expect(exportBody.format).toBe("rigcraft.inventory.snapshot");
  expect(exportBody.authoritative).toBe(true);
  expect(exportBody.lossy).toBe(false);
  expect(exportBody.snapshot.id).toBe(seeded.id);
  expect(exportBody.snapshot.revision).toBe(seeded.revision);
  expect(exportBody.snapshot.records).toHaveLength(10);
  const cpu = exportBody.snapshot.records.find(
    (record: { sku: string }) => record.sku === "CPU-7800X3D",
  );
  expect(cpu.provenance.model.source).toBe(`catalog:${CATALOG_SOURCE}`);
  expect(cpu.provenance.quantity.source).toBe("user");
  expect(cpu.specs[0].provenance.source).toBe(`catalog:${CATALOG_SOURCE}`);

  // Feed the exported CSV straight back through the real import path.
  await page.getByRole("button", { name: "Start a new import" }).click();
  await page.locator("#inventory-file").setInputFiles(csvPath);
  await expect(page.locator("#file-hint")).toContainText("exported-inventory.csv");
  await page.getByRole("button", { name: "Preview CSV" }).click();
  await expect(page.getByRole("heading", { name: "Review import" })).toBeVisible();
  await expect(
    page.locator("p.lede").filter({ hasText: "exported-inventory.csv" }),
  ).toContainText("10 rows · 0 errors · 0 duplicates");
  await page.getByRole("button", { name: "Commit snapshot" }).click();
  await expect(
    page.getByRole("heading", { name: "Committed snapshot · revision 1" }),
  ).toBeVisible();
  const table = page.locator("table.responsive-table");
  await expect(table.locator("tbody tr")).toHaveCount(10);
  const cpuRow = table.locator("tbody tr").filter({ hasText: "CPU-7800X3D" });
  await expect(cpuRow).toContainText(`Catalog facts · ${CATALOG_SOURCE}`);
  await cpuRow.locator("summary").click();
  const provenance = cpuRow.locator("ul.provenance-list");
  await expect(provenance.locator("li").filter({ hasText: "model:" })).toContainText(
    `Catalog · ${CATALOG_SOURCE}`,
  );
  await expect(
    provenance.locator("li").filter({ hasText: "quantity:" }),
  ).toContainText("User-provided");
  await expect(
    provenance.locator("li").filter({ hasText: "quantity:" }),
  ).toContainText("csv:exported-inventory.csv");
});

test("a backup restores only after inspection and a typed confirmation, and a rejected upload changes nothing", async ({
  page,
}) => {
  await seedSample(page.request);
  await page.goto("/inventory");
  await page.getByRole("button", { name: /^View / }).first().click();
  await expect(
    page.getByRole("heading", { name: /Committed snapshot · revision/ }),
  ).toBeVisible();

  const backupDownload = page.waitForEvent("download");
  await page.getByRole("button", { name: "Download backup" }).click();
  const backup = await backupDownload;
  expect(backup.suggestedFilename()).toMatch(
    /^rigcraft-backup-\d{8}T\d{12}Z\.sqlite3$/,
  );
  const backupPath = test.info().outputPath("downloaded-backup.sqlite3");
  await backup.saveAs(backupPath);
  expect(readFileSync(backupPath).subarray(0, 16).toString()).toBe(
    "SQLite format 3\u0000",
  );

  // A non-SQLite upload is rejected with its documented code and replaced nothing.
  await page.locator("#backup-file").setInputFiles({
    name: "notes.sqlite3",
    mimeType: "application/vnd.sqlite3",
    buffer: Buffer.from("this is not a database"),
  });
  await page.getByRole("button", { name: "Inspect backup" }).click();
  const alert = page.getByRole("alert");
  await expect(alert).toContainText("This backup cannot be restored");
  await expect(alert).toContainText("backup_not_sqlite");
  await expect(
    page.getByRole("heading", { name: /Committed snapshot · revision/ }),
  ).toBeVisible();
  await alert.getByRole("button", { name: "Dismiss" }).click();

  // Inspect the real backup: details are shown, replacement is not yet possible.
  await page.locator("#backup-file").setInputFiles(backupPath);
  await page.getByRole("button", { name: "Inspect backup" }).click();
  await expect(
    page.getByRole("heading", { name: "Backup inspected — nothing replaced yet" }),
  ).toBeVisible();
  const replace = page.getByRole("button", { name: "Replace this inventory" });
  await expect(replace).toBeDisabled();
  const liveSummaries = await (
    await page.request.get("/api/inventory/snapshots")
  ).json();
  const expectedSnapshots = liveSummaries.length as number;
  const expectedRecords = liveSummaries.reduce(
    (total: number, entry: { recordCount: number }) => total + entry.recordCount,
    0,
  );
  const preview = page.locator("#restore-preview");
  const field = (name: string) =>
    preview.locator(".summary-grid div").filter({ hasText: name }).locator("dd");
  await expect(field("Integrity")).toHaveText("ok");
  await expect(field("Schema version")).toHaveText("5");
  await expect(field("Snapshots")).toHaveText(String(expectedSnapshots));
  await expect(field("Latest revision")).toHaveText(
    String(liveSummaries[0].revision),
  );
  await expect(field("Records")).toHaveText(String(expectedRecords));
  await expect(field("Lots in newest snapshot")).toHaveText("10");

  // A near miss stays disabled; the exact word enables one explicit action.
  await page.locator("#restore-confirmation").fill("RESTOR");
  await expect(replace).toBeDisabled();
  await page.locator("#restore-confirmation").fill("RESTORE");
  await expect(replace).toBeEnabled();
  await replace.click();

  await expect(
    page.getByRole("heading", { name: "Inventory restored" }),
  ).toBeVisible();
  await expect(page.locator("#pre-restore-path")).toContainText(
    "inventory.sqlite3.pre-restore-",
  );
  await expect(
    page.getByRole("heading", { name: /Committed snapshot · revision/ }),
  ).toBeVisible();
  await expect(page.locator("table.responsive-table tbody tr")).toHaveCount(10);
});

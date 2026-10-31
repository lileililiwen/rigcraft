import { writeFileSync } from "node:fs";
import { expect, test } from "@playwright/test";
import { expectNoHorizontalOverflow } from "./fixtures";

/**
 * Operator-supplied catalog journeys. The document is a small synthetic fixture;
 * no third-party dataset is read, uploaded or committed.
 */

const CATALOG_NAME = "synthetic-fixture";
const CLEAN_URL = "https://example.invalid/cpu-one";

function syntheticCatalog() {
  return {
    format: "rigcraft-catalog",
    version: 1,
    dataset: {
      name: CATALOG_NAME,
      sourceUrl: "https://example.invalid/catalog",
      commit: "fixture-1",
      license: "CC0-1.0",
      retrievedAt: "2026-11-01T00:00:00Z",
    },
    parts: [
      {
        externalId: "MPN-CPU-1",
        sku: "SYN-CPU-1",
        category: "cpu",
        model: "Synthetic CPU One",
        manufacturer: "Synthetic Labs",
        specs: { socket: "AM5", tdp: "120W", igpu: "yes" },
        sourceUrl: CLEAN_URL,
        verifiedAt: "2026-11-01T00:00:00Z",
      },
      {
        externalId: "MPN-CPU-2",
        sku: "SYN-CPU-2",
        category: "cpu",
        model: "Synthetic CPU One",
        specs: { socket: "AM5", tdp: "105W" },
      },
      {
        externalId: "MPN-GPU-1",
        sku: "SYN-GPU-1",
        category: "gpu",
        model: "Synthetic GPU One",
        specs: { length: "250mm", tdp: "200W", vram: "12GB" },
      },
    ],
  };
}

function writeCatalog(name: string): string {
  return test.info().outputPath(name);
}

test("an operator uploads a catalog, sees the registry and searches it", async ({ page }) => {
  const path = writeCatalog("synthetic-catalog.json");
  writeFileSync(path, JSON.stringify(syntheticCatalog()));

  await page.goto("/catalog");
  await expect(page.getByRole("heading", { name: "Catalog", level: 1 })).toBeVisible();
  await expect(page.getByText(/never bundled, fetched from the internet/)).toBeVisible();

  await page.locator("#catalog-file").setInputFiles(path);
  await expect(page.locator("#catalog-file-hint")).toContainText("Selected synthetic-catalog.json");
  await expect(page.getByRole("button", { name: "Import catalog" })).toBeDisabled();
  await page.getByLabel(/I have the right to use this data/).check();
  await page.getByRole("button", { name: "Import catalog" }).click();

  const accepted = page.getByRole("heading", { name: "Catalog accepted" });
  await expect(accepted).toBeVisible();
  await expect(accepted).toBeFocused();
  await expect(page.getByText(/3 parts/).first()).toBeVisible();
  await expect(page.getByText(`catalog:${CATALOG_NAME}@fixture-1`)).toBeVisible();

  const registry = page
    .locator("table.responsive-table")
    .filter({ has: page.locator("caption", { hasText: "Stored catalogs" }) });
  const row = registry.locator("tbody tr").filter({ hasText: CATALOG_NAME });
  await expect(row).toContainText("CC0-1.0");
  await expect(row).toContainText("fixture-1");
  await expect(row).toContainText("3");
  await expect(row).toContainText("Active");

  await page.locator("#catalog-query").fill("SYN-CPU-1");
  await page.locator("#catalog-category").selectOption("cpu");
  await page.getByRole("button", { name: "Search catalog" }).click();
  const results = page
    .locator("table.responsive-table")
    .filter({ has: page.locator("caption", { hasText: "Ranked catalog search results" }) });
  await expect(results.locator("tbody tr").first()).toContainText("SYN-CPU-1");
  await expect(results.locator("tbody tr").first()).toContainText("Synthetic CPU One");
  await expect(results.locator("tbody tr").first()).toContainText("CC0-1.0");

  await results.getByRole("button", { name: "View part" }).first().click();
  const partHeading = page.getByRole("heading", { name: "Synthetic CPU One" });
  await expect(partHeading).toBeVisible();
  await expect(partHeading).toBeFocused();
  await expect(page.getByText("https://example.invalid/cpu-one").first()).toBeVisible();
  await expect(page.locator(".badge--catalog").first()).toBeVisible();

  await expectNoHorizontalOverflow(page);

  await page.setViewportSize({ width: 390, height: 844 });
  await expectNoHorizontalOverflow(page);
});

test("a refused document names its row and adds no dataset", async ({ page }) => {
  const path = writeCatalog("refused-catalog.json");
  const refused = syntheticCatalog();
  refused.dataset.name = "refused-fixture";
  refused.parts[1].model = "";
  writeFileSync(path, JSON.stringify(refused));

  await page.goto("/catalog");
  await page.locator("#catalog-file").setInputFiles(path);
  await page.getByLabel(/I have the right to use this data/).check();
  await page.getByRole("button", { name: "Import catalog" }).click();

  const alert = page.getByRole("alert");
  await expect(alert).toBeVisible();
  await expect(alert).toBeFocused();
  await expect(alert).toContainText("Catalog refused");
  await expect(alert).toContainText("Row 1");
  await expect(alert).toContainText("model");
  await expect(page.getByRole("heading", { name: "Catalog accepted" })).toBeHidden();
  await expect(page.getByText("refused-fixture")).toHaveCount(0);
});

test("an import row is enriched from the catalog and the committed field carries its provenance", async ({
  page,
  request,
}) => {
  // Seed the catalog through the documented API; the upload UI is covered above.
  const seeded = await request.post("/api/catalog/datasets", {
    data: { acknowledged: true, catalog: syntheticCatalog() },
  });
  expect(seeded.status(), "POST /api/catalog/datasets").toBe(200);
  const seededBody = await seeded.json();
  expect(seededBody.partCount).toBe(3);

  const csvPath = writeCatalog("enrich-row.csv");
  writeFileSync(
    csvPath,
    "sku,category,model,quantity,price,currency,specs\nSYN-CPU-1,cpu,Synthetic CPU One,1,10.00,USD,\n",
  );

  await page.goto("/inventory");
  await page.locator("#inventory-file").setInputFiles(csvPath);
  await page.getByRole("button", { name: "Preview CSV" }).click();
  await expect(page.getByRole("heading", { name: "Review import" })).toBeVisible();

  const previewRow = page
    .locator("table.responsive-table")
    .filter({ has: page.locator("caption", { hasText: "Parsed CSV rows" }) })
    .locator("tbody tr")
    .filter({ hasText: "SYN-CPU-1" });
  await expect(previewRow).toContainText("Matched");
  await expect(previewRow).toContainText(`by sku to ${CATALOG_NAME}`);
  await expect(previewRow).toContainText("Will add from the catalog");
  await expect(previewRow).toContainText("specs.igpu, specs.socket, specs.tdp");

  await previewRow.getByLabel("Apply catalog proposals").check();
  await page.getByRole("button", { name: "Commit snapshot" }).click();

  await expect(
    page.getByRole("heading", { name: "Committed snapshot · revision 1", level: 2 }),
  ).toBeVisible();
  const snapshotRow = page
    .locator("table.responsive-table")
    .filter({
      has: page.locator("caption", { hasText: "Committed stock records with provenance" }),
    })
    .locator("tbody tr")
    .filter({ hasText: "SYN-CPU-1" });
  await expect(snapshotRow).toContainText("match catalog-declared");
  await expect(snapshotRow.locator(".badge--catalog")).toHaveCount(3);
  await snapshotRow.getByText("Show provenance").click();
  const provenance = snapshotRow.locator("ul.provenance-list");
  const specs = provenance.locator("li").filter({ hasText: "specs:" });
  await expect(specs).toContainText(`Catalog · ${CATALOG_NAME}@fixture-1`);
  await expect(specs).toContainText(CLEAN_URL);
  await expect(provenance.locator("li").filter({ hasText: "sku:" })).toContainText(
    "User-provided",
  );
  await expect(provenance.locator("li").filter({ hasText: "quantity:" })).toContainText(
    "User-provided",
  );
  await expect(provenance.locator("li").filter({ hasText: "price:" })).toContainText(
    "User-provided",
  );

  await expectNoHorizontalOverflow(page);
});

import { expect, type APIRequestContext, type Page } from "@playwright/test";

/** The filename the sample endpoint advertises, and the provenance reference it produces. */
export const SAMPLE_FILENAME = "rigcraft-inventory-sample.csv";
export const CATALOG_SOURCE = "manufacturer@2026-10-31";
export const CPU_URL =
  "https://www.amd.com/en/products/processors/desktops/ryzen/7000-series/amd-ryzen-7-7800x3d.html";

/** The small synthetic catalog the journeys upload; no third-party data is used. */
export const SYNTHETIC_CATALOG_NAME = "synthetic-fixture";

export function syntheticCatalog() {
  return {
    format: "rigcraft-catalog",
    version: 1,
    dataset: {
      name: SYNTHETIC_CATALOG_NAME,
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
        sourceUrl: "https://example.invalid/cpu-one",
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

/**
 * Put the catalog registry into one known state: remove every stored dataset
 * through the documented API, then upload the synthetic fixture once. Removing a
 * dataset cascades only to its own parts and never touches inventory, so this is
 * safe to call on any database and yields the same registry on a repeat run.
 */
export async function resetCatalog(request: APIRequestContext): Promise<void> {
  const listed = await request.get("/api/catalog/datasets");
  expect(listed.status(), "GET /api/catalog/datasets").toBe(200);
  const datasets: Array<{ id: string }> = await listed.json();
  for (const dataset of datasets) {
    const removed = await request.delete(
      `/api/catalog/datasets/${dataset.id}?confirm=remove`,
    );
    expect(removed.status(), `DELETE /api/catalog/datasets/${dataset.id}`).toBe(200);
  }
  const uploaded = await request.post("/api/catalog/datasets", {
    data: { acknowledged: true, catalog: syntheticCatalog() },
  });
  expect(uploaded.status(), "POST /api/catalog/datasets").toBe(200);
  const report = await uploaded.json();
  expect(report.partCount, "the synthetic fixture has three parts").toBe(3);
}

export interface SeededSnapshot {
  id: string;
  revision: number;
}

/**
 * Seed the shipped sample through the documented API. Used by the journeys whose
 * subject is not the import itself; the preview and commit are still verified so
 * a broken sample cannot masquerade as a passing compatibility test.
 */
export async function seedSample(request: APIRequestContext): Promise<SeededSnapshot> {
  const sample = await request.get("/api/imports/sample");
  expect(sample.status(), "GET /api/imports/sample").toBe(200);
  const csv = await sample.body();

  const previewResponse = await request.post("/api/imports", {
    multipart: {
      currency: "USD",
      file: { name: SAMPLE_FILENAME, mimeType: "text/csv", buffer: csv },
    },
  });
  expect(previewResponse.status(), "POST /api/imports").toBe(200);
  const preview = await previewResponse.json();
  expect(preview.summary, "the shipped sample previews cleanly").toEqual({
    total: 10,
    valid: 10,
    errors: 0,
    duplicateRows: 0,
  });

  const commitResponse = await request.post(`/api/imports/${preview.previewId}/commit`, {
    data: {
      decisions: preview.rows.map((row: { row: number }) => ({
        row: row.row,
        action: "accept",
      })),
      duplicateResolutions: [],
    },
  });
  expect(commitResponse.status(), "POST /api/imports/{id}/commit").toBe(200);
  const { snapshot } = await commitResponse.json();
  expect(snapshot.revision, "a fresh import is revision 1").toBe(1);
  expect(snapshot.totals.recordCount, "the sample commits ten records").toBe(10);
  return { id: snapshot.id, revision: snapshot.revision };
}

/**
 * Select the test's own snapshot on a page and wait for its records to load, so a
 * test never reads whichever snapshot happens to be newest.
 */
export async function chooseSnapshot(
  page: Page,
  select: string,
  snapshot: SeededSnapshot,
): Promise<void> {
  const loaded = page.waitForResponse(
    (response) =>
      response.request().method() === "GET" &&
      new URL(response.url()).pathname === `/api/inventory/snapshots/${snapshot.id}`,
  );
  await page.selectOption(select, snapshot.id);
  const response = await loaded;
  expect(response.status(), "the selected snapshot loads").toBe(200);
}

/** The assertion the evidence documents repeat at 390 px. */
export async function expectNoHorizontalOverflow(page: Page): Promise<void> {
  const width = await page.evaluate(() => ({
    scrollWidth: document.documentElement.scrollWidth,
    clientWidth: document.documentElement.clientWidth,
  }));
  expect(
    width.scrollWidth,
    `document scrollWidth must equal clientWidth at ${width.clientWidth}px`,
  ).toBe(width.clientWidth);
}

/** Parse a rendered money amount such as "$1,123.90" into minor units. */
export function moneyToMinor(text: string | null): number {
  expect(text, "a rendered money amount").not.toBeNull();
  const amount = Number((text ?? "").replace(/[^0-9.]/g, ""));
  expect(Number.isFinite(amount), `parses ${text}`).toBe(true);
  return Math.round(amount * 100);
}

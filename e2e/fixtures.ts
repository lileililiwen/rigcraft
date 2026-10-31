import { expect, type APIRequestContext, type Page } from "@playwright/test";

/** The filename the sample endpoint advertises, and the provenance reference it produces. */
export const SAMPLE_FILENAME = "rigcraft-inventory-sample.csv";
export const CATALOG_SOURCE = "manufacturer@2026-10-31";
export const CPU_URL =
  "https://www.amd.com/en/products/processors/desktops/ryzen/7000-series/amd-ryzen-7-7800x3d.html";

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

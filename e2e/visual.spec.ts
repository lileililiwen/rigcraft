import { expect, test, type Locator, type Page } from "@playwright/test";
import { chooseSnapshot, resetCatalog, seedSample } from "./fixtures";

/**
 * Committed pixel baselines for the six client routes at desktop and phone
 * widths. Every test seeds the state it needs immediately before the capture, so
 * no baseline depends on which spec file ran before it, and a second run against
 * the same database produces the same image.
 *
 * The captures are viewport screenshots: a long records table or history list
 * must not make a baseline grow. Each route is scrolled to a fixed anchor so the
 * interesting region is in frame.
 */

const VIEWPORTS = [
  { label: "1440x900", viewport: { width: 1440, height: 900 } },
  { label: "390x844", viewport: { width: 390, height: 844 } },
] as const;

/**
 * The bounded tolerance covers antialiasing at a glyph edge. It is not a
 * substitute for the locator masks: an unmasked colour or spacing change moves
 * far more than one percent of the viewport.
 */
const CAPTURE = {
  animations: "disabled" as const,
  caret: "hide" as const,
  maxDiffPixelRatio: 0.01,
  threshold: 0.05,
  scale: "css" as const,
};

async function settle(page: Page): Promise<void> {
  await page.evaluate(async () => {
    await document.fonts.ready;
  });
}

/** Scroll the route's evidence anchor to a fixed 8 px from the viewport top. */
async function anchor(page: Page, selector: string): Promise<void> {
  await page.evaluate((target) => {
    const element = document.querySelector(target);
    if (!element) {
      throw new Error(`missing visual anchor ${target}`);
    }
    const top = window.scrollY + element.getBoundingClientRect().top;
    window.scrollTo(0, Math.max(0, Math.round(top - 8)));
  }, selector);
  await settle(page);
}

/** A block-level grid cell in the committed-snapshot meta list. */
function recordMeta(page: Page, label: string): Locator {
  return page.locator("dl.record-meta > div").filter({ hasText: label });
}

for (const size of VIEWPORTS) {
  test.describe(`visual baseline at ${size.label}`, () => {
    test.use({ viewport: size.viewport, deviceScaleFactor: 1 });

    test(`inventory at ${size.label}`, async ({ page, request }) => {
      // A fresh import is always revision 1, so the newest snapshot the page
      // opens automatically is the ten-record shipped sample.
      await seedSample(request);

      await page.goto("/inventory");
      await expect(page.locator("#console-bar-title")).toBeVisible();
      await expect(page.locator("table.responsive-table tbody tr")).toHaveCount(10);
      await anchor(page, ".console-bar");

      await expect(page).toHaveScreenshot(`inventory-${size.label}.png`, {
        ...CAPTURE,
        mask: [
          page.locator("section.console-bar .console-stats > div").first(),
          page.locator("section.console-bar .console-bar__meta"),
          recordMeta(page, "Snapshot ID"),
          recordMeta(page, "Committed"),
          recordMeta(page, "Source file"),
        ],
      });
    });

    test(`catalog at ${size.label}`, async ({ page, request }) => {
      // Reset through the documented API, then upload the synthetic fixture:
      // the registry is exactly one three-part dataset on the first and on a
      // repeat run. The import time is masked.
      await resetCatalog(request);

      await page.goto("/catalog");
      await expect(page.getByRole("heading", { name: "Datasets", level: 2 })).toBeVisible();
      await expect(page.locator("table.catalog-table tbody tr")).toHaveCount(1);
      await anchor(page, "section.panel:has(#registry-title)");

      await expect(page).toHaveScreenshot(`catalog-${size.label}.png`, {
        ...CAPTURE,
        mask: [
          page.locator("section.panel:has(#registry-title) td[data-label='Dataset']"),
          // The native file control's label is supplied by the browser, not the app.
          page.locator("#catalog-file"),
        ],
      });
    });

    test(`build check at ${size.label}`, async ({ page, request }) => {
      const snapshot = await seedSample(request);

      await page.goto("/builds/check");
      await chooseSnapshot(page, "#check-snapshot", snapshot);
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
      await anchor(page, "#result-title");

      await expect(page).toHaveScreenshot(`builds-check-${size.label}.png`, {
        ...CAPTURE,
        mask: [
          page.locator(".console-summary .verdict-line > span.small.muted"),
          // Provenance observation times are recorded at import time.
          page.locator("time"),
          // The option label embeds a timestamp and a short snapshot id.
          page.locator("#check-snapshot"),
        ],
      });
    });

    test(`recommend at ${size.label}`, async ({ page, request }) => {
      const snapshot = await seedSample(request);

      await page.goto("/recommend");
      await chooseSnapshot(page, "#recommend-snapshot", snapshot);
      await page.locator("#recommend-budget").fill("1800");
      await page.getByRole("button", { name: "Generate recommendations" }).click();

      await expect(page.getByRole("heading", { name: "Recommended builds" })).toBeVisible();
      await expect(page.locator(".status-line")).toContainText("Recommendation complete");
      await anchor(page, ".workbench");

      await expect(page).toHaveScreenshot(`recommend-${size.label}.png`, {
        ...CAPTURE,
        mask: [
          page.locator("article.candidate > p.small.muted"),
          page.locator("table.compare-table tbody tr").filter({ hasText: "Check report" }),
          page.locator("#recommend-snapshot"),
        ],
      });
    });

    test(`diagnose at ${size.label}`, async ({ page, request }) => {
      const snapshot = await seedSample(request);

      await page.goto("/diagnose");
      await chooseSnapshot(page, "#diagnose-snapshot", snapshot);
      await page
        .locator("#diagnose-text")
        .fill(
          [
            "cpu: AMD Ryzen 7 7800X3D",
            "motherboard: ASUS ROG STRIX B650E-F GAMING WIFI",
          ].join("\n"),
        );
      await page.getByRole("button", { name: "Match this list" }).click();
      await expect(page.locator(".review-line")).toHaveCount(2);
      await page.getByRole("button", { name: "Diagnose this build" }).click();

      await expect(page.getByRole("heading", { name: "Build diagnosis" })).toBeVisible();
      await anchor(page, "#diagnosis-result-title");

      await expect(page).toHaveScreenshot(`diagnose-${size.label}.png`, {
        ...CAPTURE,
        mask: [
          page.locator(".console-summary .verdict-line > span.small.muted"),
          page.locator("time"),
          page.locator("#diagnose-snapshot"),
        ],
      });
    });

    test(`unknown route 404 at ${size.label}`, async ({ page }) => {
      const response = await page.goto("/nope");
      expect(response?.status(), "GET /nope").toBe(404);
      expect(await page.evaluate(() => document.contentType)).toBe("text/plain");
      await expect(page.locator("body")).toContainText("404 Not found.");

      await expect(page).toHaveScreenshot(`not-found-${size.label}.png`, CAPTURE);
    });
  });
}

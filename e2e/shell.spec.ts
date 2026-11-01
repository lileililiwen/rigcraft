import { expect, test } from "@playwright/test";
import { seedSample } from "./fixtures";

/**
 * Shell chrome rather than a journey. Both rules this guards were invisible to
 * unit tests: a collapsed table caption and a skip link that painted a fragment
 * of itself above the header on every route.
 */
test.use({ viewport: { width: 390, height: 844 } });

function shortId(id: string): string {
  return id.length > 12 ? `${id.slice(0, 8)}…${id.slice(-4)}` : id;
}

test("a table caption keeps the full card width at 390 px", async ({ page, request }) => {
  const snapshot = await seedSample(request);

  await page.goto("/inventory");
  await page.getByRole("button", { name: `View ${shortId(snapshot.id)}` }).click();

  const caption = page.locator("table.responsive-table caption").first();
  await expect(caption).toBeVisible();
  await expect(caption).toHaveText(
    "Committed stock records with provenance and unknown specs",
  );

  // A block-level responsive table used to shrink-wrap its caption to one word
  // per line (93 px at this viewport). The card is roughly 330 px wide.
  const box = await caption.boundingBox();
  expect(box, "the caption renders").not.toBeNull();
  expect(box!.width, "caption width at 390 px").toBeGreaterThan(150);
});

test("the skip link is off-screen at rest and fully visible on the first Tab", async ({
  page,
}) => {
  await page.goto("/inventory");
  const skip = page.getByRole("link", { name: /^Skip to/ });

  const atRest = await skip.boundingBox();
  expect(atRest, "the skip link is laid out while unfocused").not.toBeNull();
  expect(
    atRest!.y + atRest!.height,
    "no fragment of the skip link may paint above the viewport",
  ).toBeLessThanOrEqual(0);

  await page.keyboard.press("Tab");
  await expect(skip).toBeFocused();

  const focused = await skip.boundingBox();
  expect(focused, "the focused skip link is laid out").not.toBeNull();
  expect(focused!.y, "the focused skip link starts inside the viewport").toBeGreaterThanOrEqual(0);
  expect(
    focused!.y + focused!.height,
    "the focused skip link is fully visible",
  ).toBeGreaterThan(0);
});

test("the built app serves its icon without a failed request", async ({ page, request }) => {
  const ico = await request.get("/favicon.ico");
  expect(ico.status(), "GET /favicon.ico").toBe(200);
  expect(ico.headers()["content-type"]).toContain("image");

  const svg = await request.get("/favicon.svg");
  expect(svg.status(), "GET /favicon.svg").toBe(200);
  expect(svg.headers()["content-type"]).toContain("image/svg+xml");

  const failed: string[] = [];
  page.on("response", (response) => {
    if (response.url().includes("favicon") && response.status() >= 400) {
      failed.push(`${response.status()} ${response.url()}`);
    }
  });
  await page.goto("/inventory");
  await page.waitForLoadState("load");

  await expect(page.locator('link[rel="icon"]').first()).toHaveAttribute(
    "href",
    /favicon/,
  );
  expect(failed, "no icon request fails").toEqual([]);
});

import { expect, test } from "@playwright/test";
import { chooseSnapshot, seedSample } from "./fixtures";

/**
 * The audited fixes were only visible in a browser. These guards fail if any of
 * them regresses, even when the unit and component tests still pass.
 */
test.use({ locale: "de-DE" });

test("static and metadata paths are not the SPA shell, while a client route is", async ({
  request,
}) => {
  for (const path of ["/assets/nope.js", "/robots.txt", "/llms.txt"]) {
    const response = await request.get(path);
    expect(response.status(), `${path} must be a real 404`).toBe(404);
    const body = await response.text();
    expect(body.toLowerCase(), `${path} must not serve the shell`).not.toContain(
      "<!doctype html",
    );
    expect(body, `${path} must not serve the shell`).not.toContain('id="root"');
  }

  // Positive control: the same fallback still serves a real client route.
  const shell = await request.get("/diagnose");
  expect(shell.status()).toBe(200);
  expect(shell.headers()["content-type"]).toContain("text/html");
  expect(await shell.text()).toContain('id="root"');
});

test("a typo path is a real 404, not the Inventory page", async ({ page, request }) => {
  // Visiting the typo directly must not render the app. The SPA shell is only
  // served for the six listed routes.
  const response = await page.goto("/recomend");
  expect(response?.status(), "GET /recomend").toBe(404);
  expect(response?.headers()["content-type"]).toContain("text/plain");
  expect(await page.content()).not.toContain('id="root"');
  expect(await page.locator("h1").count()).toBe(0);
  await expect(page.locator("body")).toContainText("404 Not found");

  // The client route it was meant to be still works.
  const ok = await page.goto("/recommend");
  expect(ok?.status()).toBe(200);
  expect(await page.locator('div[id="root"]').count()).toBe(1);

  // The API root is JSON, never HTML.
  for (const path of ["/api", "/api/"]) {
    const api = await request.get(path);
    expect(api.status(), `GET ${path}`).toBe(404);
    expect(api.headers()["content-type"], `GET ${path}`).toContain("application/json");
    expect((await api.json()).error.code, `GET ${path}`).toBe("route_not_found");
  }
});

test("a malformed JSON body still gets the JSON error envelope", async ({ request }) => {
  const response = await request.post("/api/builds/check", {
    headers: { "content-type": "application/json" },
    data: '{"snapshotId":',
  });

  expect(response.status()).toBe(400);
  expect(response.headers()["content-type"]).toContain("application/json");
  const body = await response.json();
  expect(body.error.code).toBe("invalid_json");
  expect(body.error.message).toContain("JSON");
  expect(JSON.stringify(body)).not.toContain('id="root"');
});

test("/diagnose renders sections 1 to 5 before any list is matched", async ({
  page,
  request,
}) => {
  const snapshot = await seedSample(request);

  await page.goto("/diagnose");
  await chooseSnapshot(page, "#diagnose-snapshot", snapshot);

  // Exact order and exact count: a skipped section number must fail here.
  await expect(page.locator("main h2")).toHaveText([
    "1. Choose an inventory snapshot",
    "2. Paste the parts list",
    "3. Review the matches",
    "4. Add a part by hand",
    "5. Read the diagnosis",
  ]);
  await expect(page.getByText("Match a pasted list in step 2 to review its lines here.")).toBeVisible();
});

test("a timestamp is ISO 8601 UTC even when the browser locale is not", async ({
  page,
  request,
}) => {
  const snapshot = await seedSample(request);

  await page.goto("/inventory");
  const shortId =
    snapshot.id.length > 12
      ? `${snapshot.id.slice(0, 8)}…${snapshot.id.slice(-4)}`
      : snapshot.id;
  await page.getByRole("button", { name: `View ${shortId}` }).click();

  const committed = page.locator("dl.record-meta time");
  await expect(committed).toHaveText(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z$/);
  const rendered = (await committed.textContent()) ?? "";
  // A de-DE locale would render "31.10.2026, 15:55:26" if the UI used toLocaleString.
  expect(rendered).not.toContain(",");
  expect(rendered).not.toContain("/");
  expect(new Date(rendered).toISOString().slice(0, 10)).toBe(rendered.slice(0, 10));
});

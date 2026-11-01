import { createServer, type Server } from "node:http";
import { expect, test } from "@playwright/test";
import { chooseSnapshot, seedSample } from "./fixtures";

/**
 * Delivery hardening is a header change, so the browser suite proves the policy
 * does not break what it protects: every route renders with no CSP violation,
 * a typo still returns a real 404, and the BYOK assistant still reaches a
 * loopback provider through the service.
 */

const ROUTES = ["/", "/inventory", "/catalog", "/builds/check", "/recommend", "/diagnose"];

function collectCspViolations(page: import("@playwright/test").Page): string[] {
  const violations: string[] = [];
  page.on("console", (message) => {
    if (
      message.type() === "error" &&
      /content security policy/i.test(message.text())
    ) {
      violations.push(message.text());
    }
  });
  page.on("pageerror", (error) => {
    violations.push(error.message);
  });
  return violations;
}

test("every route renders under the CSP and a typo is still a real 404", async ({ page }) => {
  const violations = collectCspViolations(page);

  for (const route of ROUTES) {
    await page.goto(route);
    await expect(page.locator("main")).toBeVisible();
    await expect(page).toHaveTitle(/RigCraft/);
  }

  const notFound = await page.goto("/recomend");
  expect(notFound, "the typo path resolves a response").not.toBeNull();
  expect(notFound!.status(), "a typo is a 404").toBe(404);
  await expect(page.locator("body")).toContainText("404 Not found.");
  await expect(page.locator("#root")).toHaveCount(0);

  expect(violations, "no CSP violation or page error on any route").toEqual([]);
});

test("the assistant still reaches a loopback provider", async ({ page, request }) => {
  const snapshot = await seedSample(request);
  const proposal = {
    budgetMinor: { value: 200000, excerpt: "$2000" },
    useCase: { value: "gaming", excerpt: "gaming" },
    inventoryOnly: { value: true, excerpt: "from my stock" },
    lockedSkus: { value: [], excerpt: "" },
    excludedSkus: { value: [], excerpt: "" },
  };
  const calls: string[] = [];
  const provider: Server = createServer((request, response) => {
    calls.push(`${request.method} ${request.url}`);
    request.resume();
    request.on("end", () => {
      const body = JSON.stringify({
        choices: [{ message: { role: "assistant", content: JSON.stringify(proposal) } }],
      });
      response.writeHead(200, {
        "content-type": "application/json",
        "content-length": Buffer.byteLength(body),
      });
      response.end(body);
    });
  });
  await new Promise<void>((resolve) => provider.listen(0, "127.0.0.1", resolve));
  const address = provider.address();
  expect(address && typeof address === "object", "the stub has a port").toBe(true);
  const baseUrl = `http://127.0.0.1:${(address as { port: number }).port}/v1`;

  const violations = collectCspViolations(page);
  try {
    await page.goto("/recommend");
    await chooseSnapshot(page, "#recommend-snapshot", snapshot);
    const assistant = page.locator("details.assistant");
    await assistant.locator("summary").click();
    await page.locator("#assistant-key").fill("sk-loopback-not-real");
    await page.locator("#assistant-base-url").fill(baseUrl);
    await page.locator("#assistant-model").fill("loopback-model");
    await page
      .locator("#assistant-sentence")
      .fill("a $2000 gaming build from my stock");
    await page.locator("#assistant-consent").check();
    await page.getByRole("button", { name: "Interpret sentence" }).click();

    await expect(
      page.getByRole("heading", { name: "Proposed request — review and confirm" }),
    ).toBeVisible();
    await page.getByRole("button", { name: "Confirm and apply to the form" }).click();
    await expect(page.locator("#recommend-budget")).toHaveValue("2000.00");

    expect(calls.length, "the loopback stub received the provider request").toBeGreaterThan(0);
    expect(violations, "the assistant request does not violate the CSP").toEqual([]);
  } finally {
    await new Promise<void>((resolve, reject) => {
      provider.close((error) => (error ? reject(error) : resolve()));
    });
  }
});

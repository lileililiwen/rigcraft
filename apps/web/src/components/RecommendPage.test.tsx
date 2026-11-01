import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { RecommendPage } from "./RecommendPage";
import type {
  Candidate,
  CheckReport,
  RecordItem,
  RecommendationReport,
  Snapshot,
  SnapshotSummary,
} from "../types";

const provenance = {
  source: "user",
  reference: "csv:recommendation.csv",
  observedAt: "2026-10-31T12:00:00Z",
};

function record(
  id: string,
  category: string,
  sku: string,
  model: string,
  priceMinor: number,
): RecordItem {
  return {
    id,
    lot: 1,
    sku,
    category,
    modelRaw: model,
    modelNormalized: model,
    quantity: 2,
    priceMinor,
    currency: "USD",
    specs: [],
    provenance: { model: provenance },
    match: { basis: "lexical", confidence: "unverified" },
  };
}

const snapshot: Snapshot = {
  id: "snap-r1",
  createdAt: "2026-10-31T12:00:00Z",
  revision: 1,
  parentSnapshotId: null,
  currency: "USD",
  source: { filename: "recommendation.csv", sha256: "a".repeat(64) },
  note: null,
  records: [
    record("r-cpu", "cpu", "FIX-CPU", "Fixture CPU", 19900),
    record("r-board", "motherboard", "FIX-MB", "Fixture Board", 15900),
    record("r-ram", "ram", "FIX-RAM", "Fixture Memory", 10900),
    record("r-gpu", "gpu", "FIX-GPU", "Fixture GPU", 29900),
    record("r-psu", "psu", "FIX-PSU", "Fixture PSU", 7900),
    record("r-case", "case", "FIX-CASE", "Fixture Case", 8900),
    record("r-cooler", "cooler", "FIX-COOLER", "Fixture Cooler", 4900),
    record("r-ssd", "storage", "FIX-SSD", "Fixture SSD", 7900),
  ],
  totals: { recordCount: 8, unitCount: 16, valueMinor: 102800 },
};

const summary: SnapshotSummary = {
  id: snapshot.id,
  createdAt: snapshot.createdAt,
  revision: 1,
  parentSnapshotId: null,
  currency: "USD",
  sourceFilename: "recommendation.csv",
  recordCount: 8,
  unitCount: 16,
  valueMinor: 102800,
  note: null,
};

function checkReport(rank: number): CheckReport {
  return {
    id: `check-${rank}`,
    snapshotId: snapshot.id,
    snapshotRevision: 1,
    ruleSetVersion: "compat-rules-v1",
    checkedAt: "2026-10-31T12:01:00Z",
    selection: [],
    completeness: { status: "complete", missing: [] },
    overallStatus: "compatible",
    rules: [
      {
        ruleId: "cpu-socket",
        ruleVersion: "compat-rules-v1",
        title: "CPU and motherboard Socket",
        category: "cpu",
        status: "compatible",
        parts: [],
        evidence: [
          {
            part: { recordId: "r-cpu", sku: "FIX-CPU", category: "cpu", model: "Fixture CPU" },
            field: "socket",
            value: "AM5",
            provenance,
          },
        ],
        explanation: "CPU and motherboard both declare Socket AM5.",
        followUp: null,
        method: "Exact Socket comparison.",
      },
    ],
    summary: { compatible: 14, incompatible: 0, unknown: 0 },
    stale: false,
    currentSnapshotRevision: 1,
  };
}

function candidate(rank: number, label: string, totalMinor: number): Candidate {
  return {
    rank,
    objective: rank === 1 ? "best_overall" : "lowest_cost",
    label,
    materialSignature: `cpu:FIX-CPU-${rank}:1`,
    parts: [
      {
        recordId: "r-cpu",
        sku: "FIX-CPU",
        category: "cpu",
        model: "Fixture CPU",
        quantity: 1,
        owned: false,
        unitPriceMinor: 19900,
        lineTotalMinor: 19900,
        available: 2,
      },
      {
        recordId: "r-ssd",
        sku: "FIX-SSD",
        category: "storage",
        model: "Fixture SSD",
        quantity: 1,
        owned: false,
        unitPriceMinor: 7900,
        lineTotalMinor: 7900,
        available: 2,
      },
    ],
    totalMinor,
    budgetDeltaMinor: 200000 - totalMinor,
    stock: [
      {
        sku: "FIX-CPU",
        category: "cpu",
        used: 1,
        available: 2,
        remaining: 1,
        owned: false,
      },
      {
        sku: "FIX-SSD",
        category: "storage",
        used: 1,
        available: 2,
        remaining: 1,
        owned: false,
      },
    ],
    unitsFromStock: 2,
    ownedUnits: 0,
    overallStatus: "compatible",
    completeness: "complete",
    checkReportId: `check-${rank}`,
    checkSummary: { compatible: 14, incompatible: 0, unknown: 0 },
    unknownFollowUps: [],
    scores: {
      total: rank === 1 ? 71.5 : 65.25,
      confidence: "full",
      dimensions: [
        {
          key: "cost",
          label: "Budget headroom",
          available: true,
          value: 0.3,
          weight: 0.3,
          basis: "Derived from the total price and the budget.",
        },
        {
          key: "performance",
          label: "Performance proxy",
          available: true,
          value: 80,
          weight: 0.55,
          basis: "Mean of confirmed perf_score.",
        },
        {
          key: "upgrade",
          label: "Upgrade room",
          available: true,
          value: 0.5,
          weight: 0.15,
          basis: "Two free memory slots.",
        },
      ],
    },
    reasons: ["Highest weighted score for gaming: 71.50.", "Total 2,000.00 USD leaves 0.00 USD."],
    alternatives: [
      {
        category: "cpu",
        sku: "FIX-CPU-ALT",
        model: "Fixture Alt CPU",
        lineTotalMinor: 17900,
        reason: "different_build",
        detail: "Same or lower cost; not part of this build.",
      },
    ],
    check: checkReport(rank),
  };
}

function report(): RecommendationReport {
  return {
    id: "rec-1",
    createdAt: "2026-10-31T12:02:00Z",
    snapshotId: snapshot.id,
    snapshotRevision: 1,
    ruleSetVersion: "compat-rules-v1",
    scoringVersion: "recommend-score-v1",
    status: "ok",
    request: {
      budgetMinor: 200000,
      currency: "USD",
      useCase: "gaming",
      inventoryOnly: true,
      lockedSkus: [],
      excludedSkus: [],
    },
    useCaseRule: {
      useCase: "gaming",
      requiresDiscreteGpu: true,
      acceptsIgpu: false,
      explanation:
        "Gaming requires a discrete graphics card in every option. A CPU with sourced `igpu=yes` does not complete a gaming build.",
    },
    currency: "USD",
    budgetMinor: 200000,
    candidates: [candidate(1, "Best overall", 27800), candidate(2, "Lowest cost", 25800)],
    noSolution: null,
    search: {
      limit: 100_000,
      nodesExamined: 320,
      leavesEvaluated: 24,
      validBuilds: 6,
      limited: false,
      method: "depth first",
    },
    exclusions: [],
    stale: false,
    currentSnapshotRevision: 1,
  };
}

function noSolutionReport(): RecommendationReport {
  return {
    ...report(),
    status: "no_solution",
    candidates: [],
    noSolution: {
      binding: "budget",
      headline: "The budget is below the cheapest complete build.",
      explanation: "The cheapest complete build costs 2,450.00 USD.",
      blockers: [
        {
          code: "cheapest_part",
          category: "cpu",
          sku: "FIX-CPU",
          detail: "Cheapest cpu: Fixture CPU at 900.00 USD.",
        },
      ],
      adjustable: ["Raise the budget to at least 2,450.00 USD."],
      minimumBuildMinor: 245000,
      budgetShortfallMinor: 45000,
    },
  };
}

function jsonResponse(data: unknown, status = 200): Promise<Response> {
  return Promise.resolve({
    ok: status < 400,
    status,
    headers: { get: () => "application/json" },
    text: async () => JSON.stringify(data),
  } as unknown as Response);
}

function installFetch(
  handler: (url: string, init?: RequestInit) => Promise<Response>,
) {
  const fetchMock = vi.fn((input: RequestInfo | URL, init?: RequestInit) =>
    handler(String(input), init),
  );
  vi.stubGlobal("fetch", fetchMock);
  return fetchMock;
}

function handlerWith(
  results: () => Response | Promise<Response>,
  options: { snapshots?: SnapshotSummary[] } = {},
) {
  const list = options.snapshots ?? [summary];
  return async (url: string, init?: RequestInit) => {
    if (url === "/api/inventory/snapshots") {
      return jsonResponse(list);
    }
    if (url === "/api/inventory/snapshots/snap-r1") {
      return jsonResponse(snapshot);
    }
    if (url === "/api/recommendations" && init?.method === "POST") {
      return results();
    }
    return jsonResponse({ error: { code: "unexpected", message: url } }, 500);
  };
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("recommendation page", () => {
  it("generates ranked options, announces completion and focuses the result", async () => {
    installFetch(handlerWith(() => jsonResponse(report())));
    const user = userEvent.setup();
    render(<RecommendPage />);

    expect(
      screen.getByRole("heading", { level: 1, name: "Build recommendation" }),
    ).toBeInTheDocument();
    const budget = await screen.findByLabelText("Total budget (USD)");
    await waitFor(() => expect(screen.getByLabelText("Snapshot")).toHaveValue("snap-r1"));

    await user.type(budget, "2000");
    await user.click(screen.getByRole("button", { name: "Generate recommendations" }));

    const heading = await screen.findByRole("heading", { name: "Recommended builds" });
    await waitFor(() => expect(heading).toHaveFocus());
    expect(
      screen.getAllByRole("status").some((node) =>
        /Recommendation complete: 2 materially different options/.test(
          node.textContent ?? "",
        ),
      ),
    ).toBe(true);
    expect(screen.getByRole("heading", { name: /1\. Best overall/ })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: /2\. Lowest cost/ })).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: "Gaming requires a discrete graphics card" }),
    ).toBeInTheDocument();
    expect(screen.getByText(/2 materially different builds found/)).toBeInTheDocument();
    expect(screen.getByText("Compare options")).toBeInTheDocument();
    expect(screen.getAllByText(/Highest weighted score for gaming/).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/check-1/).length).toBeGreaterThan(0);
    expect(screen.getByRole("button", { name: "Download JSON export" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Download Markdown plan" })).toBeInTheDocument();
  });

  it("states the real option count when only one material build exists", async () => {
    installFetch(
      handlerWith(() =>
        jsonResponse({ ...report(), candidates: [candidate(1, "Best overall", 27800)] }),
      ),
    );
    const user = userEvent.setup();
    render(<RecommendPage />);

    await screen.findByLabelText("Total budget (USD)");
    await user.type(screen.getByLabelText("Total budget (USD)"), "2000");
    await user.click(screen.getByRole("button", { name: "Generate recommendations" }));

    await screen.findByRole("heading", { name: "Recommended builds" });
    expect(screen.getByText(/1 materially different build found/)).toBeInTheDocument();
    expect(screen.getByText(/no option was padded to reach three/)).toBeInTheDocument();
    expect(document.querySelectorAll("article.candidate")).toHaveLength(1);
    expect(
      screen.getAllByRole("status").some((node) =>
        /Recommendation complete: 1 materially different option/.test(node.textContent ?? ""),
      ),
    ).toBe(true);
  });

  it("explains a no-solution budget constraint instead of showing a build", async () => {
    installFetch(handlerWith(() => jsonResponse(noSolutionReport())));
    const user = userEvent.setup();
    render(<RecommendPage />);

    await screen.findByLabelText("Total budget (USD)");
    await user.type(screen.getByLabelText("Total budget (USD)"), "1200");
    await user.click(screen.getByRole("button", { name: "Generate recommendations" }));

    expect(await screen.findByRole("heading", { name: "Budget constraint" })).toBeInTheDocument();
    expect(
      screen.getByText("The budget is below the cheapest complete build."),
    ).toBeInTheDocument();
    expect(screen.getByText(/Cheapest cpu: Fixture CPU/)).toBeInTheDocument();
    expect(screen.getByText("Raise the budget to at least 2,450.00 USD.")).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Download JSON export" }),
    ).not.toBeInTheDocument();
  });

  it("marks results stale when the budget changes", async () => {
    installFetch(handlerWith(() => jsonResponse(report())));
    const user = userEvent.setup();
    render(<RecommendPage />);

    await screen.findByLabelText("Total budget (USD)");
    await user.type(screen.getByLabelText("Total budget (USD)"), "2000");
    await user.click(screen.getByRole("button", { name: "Generate recommendations" }));
    await screen.findByRole("heading", { name: "Recommended builds" });
    expect(screen.queryByText("These results are stale.")).not.toBeInTheDocument();

    await user.type(screen.getByLabelText("Total budget (USD)"), "0");
    expect(screen.getByText("These results are stale.")).toBeInTheDocument();
  });

  it("downloads an export and announces the file name", async () => {
    const fetchMock = installFetch(async (url, init) => {
      if (url === "/api/recommendations/rec-1/export?format=json") {
        return jsonResponse({ exportVersion: "recommend-export-v1" });
      }
      return handlerWith(() => jsonResponse(report()))(url, init);
    });
    const createObjectURL = vi.fn(() => "blob:recommendation");
    const revokeObjectURL = vi.fn();
    Object.defineProperty(URL, "createObjectURL", { value: createObjectURL, configurable: true });
    Object.defineProperty(URL, "revokeObjectURL", { value: revokeObjectURL, configurable: true });
    const click = vi
      .spyOn(HTMLAnchorElement.prototype, "click")
      .mockImplementation(() => undefined);
    const user = userEvent.setup();
    render(<RecommendPage />);

    await screen.findByLabelText("Total budget (USD)");
    await user.type(screen.getByLabelText("Total budget (USD)"), "2000");
    await user.click(screen.getByRole("button", { name: "Generate recommendations" }));
    await screen.findByRole("heading", { name: "Recommended builds" });

    await user.click(screen.getByRole("button", { name: "Download JSON export" }));
    const notices = await screen.findAllByText(
      /Export downloaded: rigcraft-recommendation\.json/,
    );
    expect(notices.length).toBeGreaterThan(0);
    expect(click).toHaveBeenCalled();
    expect(createObjectURL).toHaveBeenCalled();
    expect(
      fetchMock.mock.calls.some(([url]) =>
        String(url).includes("/api/recommendations/rec-1/export?format=json"),
      ),
    ).toBe(true);
  });

  it("keeps the request and offers retry when the service fails", async () => {
    installFetch(
      handlerWith(() =>
        jsonResponse({ error: { code: "network_error", message: "Service unavailable." } }, 500),
      ),
    );
    const user = userEvent.setup();
    render(<RecommendPage />);

    await screen.findByLabelText("Total budget (USD)");
    await user.type(screen.getByLabelText("Total budget (USD)"), "2000");
    await user.click(screen.getByRole("button", { name: "Generate recommendations" }));

    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent("Service unavailable.");
    await waitFor(() => expect(alert).toHaveFocus());
    expect(screen.getByLabelText("Total budget (USD)")).toHaveValue("2000");
    expect(screen.getByRole("button", { name: "Retry recommendation" })).toBeInTheDocument();
  });

  it("explains the empty inventory state", async () => {
    installFetch(handlerWith(() => jsonResponse(report()), { snapshots: [] }));
    render(<RecommendPage />);
    expect(
      await screen.findByText(/No inventory snapshot exists yet/),
    ).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Import a CSV in Inventory" })).toHaveAttribute(
      "href",
      "/inventory",
    );
  });

  it("disables Generate while the budget is empty or unparsable and keeps the guidance", async () => {
    const fetchMock = installFetch(handlerWith(() => jsonResponse(report())));
    const user = userEvent.setup();
    render(<RecommendPage />);

    const budget = await screen.findByLabelText("Total budget (USD)");
    const generate = screen.getByRole("button", { name: "Generate recommendations" });
    expect(generate).toBeDisabled();
    expect(generate).toHaveAccessibleDescription(
      /Generate stays disabled until the budget is a positive amount/,
    );

    await user.type(budget, "1.234");
    expect(generate).toBeDisabled();
    expect(budget).toHaveAttribute("aria-invalid", "true");
    expect(screen.getByText("Enter a budget")).toBeInTheDocument();
    expect(screen.getAllByText(/at most two decimals/).length).toBeGreaterThan(0);
    expect(screen.getByText("budget_invalid")).toBeInTheDocument();
    expect(
      fetchMock.mock.calls.some(([url]) => String(url) === "/api/recommendations"),
    ).toBe(false);

    await user.clear(budget);
    await user.type(budget, "1,800");
    expect(generate).toBeEnabled();
    expect(budget).not.toHaveAttribute("aria-invalid");
  });

  it("sends grouped comma and space budgets as integer minor units", async () => {
    const bodies: Array<Record<string, unknown>> = [];
    installFetch(async (url, init) => {
      if (url === "/api/recommendations" && init?.method === "POST") {
        bodies.push(JSON.parse(String(init.body)));
        return jsonResponse(report());
      }
      return handlerWith(() => jsonResponse(report()))(url, init);
    });
    const user = userEvent.setup();
    render(<RecommendPage />);

    const budget = await screen.findByLabelText("Total budget (USD)");
    await user.type(budget, "1,800");
    await user.click(screen.getByRole("button", { name: "Generate recommendations" }));
    await screen.findByRole("heading", { name: "Recommended builds" });

    await user.clear(budget);
    await user.type(budget, "1 800.50");
    await user.click(screen.getByRole("button", { name: "Generate recommendations" }));
    await waitFor(() => expect(bodies).toHaveLength(2));

    expect(bodies[0]).toMatchObject({ budgetMinor: 180000 });
    expect(bodies[1]).toMatchObject({ budgetMinor: 180050 });
  });
  it("applies a confirmed assistant proposal and then runs the deterministic search", async () => {
    const bodies: unknown[] = [];
    installFetch(async (url, init) => {
      if (url === "/api/inventory/snapshots") {
        return jsonResponse([summary]);
      }
      if (url === "/api/inventory/snapshots/snap-r1") {
        return jsonResponse(snapshot);
      }
      if (url === "/api/ai/parse") {
        return jsonResponse({
          mode: "provider",
          promptVersion: "ai-parse-v1",
          provider: { kind: "mock", baseUrl: "http://127.0.0.1:9999/v1", model: "mock" },
          proposal: {
            budgetMinor: { value: 200000, source: "sentence", excerpt: "2000" },
            currency: { value: "USD", source: "snapshot" },
            useCase: { value: "office", source: "sentence", excerpt: "office" },
            inventoryOnly: { value: true, source: "default" },
            lockedSkus: { value: ["FIX-CPU"], source: "sentence", excerpt: "FIX-CPU" },
            excludedSkus: { value: [], source: "default" },
          },
          ambiguities: [],
          ignoredFields: [],
          sent: {
            sentence: true,
            currency: true,
            useCases: ["gaming", "office", "creation"],
            inventoryFields: [],
            inventoryRecords: 0,
          },
          confirmRequired: true,
        });
      }
      if (url === "/api/recommendations" && init?.method === "POST") {
        bodies.push(JSON.parse(String(init.body)));
        return jsonResponse(report());
      }
      return jsonResponse({ error: { code: "unexpected", message: url } }, 500);
    });
    const user = userEvent.setup();
    render(<RecommendPage />);

    await screen.findByLabelText("Total budget (USD)");
    await user.type(screen.getByLabelText("Provider key"), "sk-page-test-only");
    await user.type(
      screen.getByLabelText("Provider base URL"),
      "http://127.0.0.1:9999/v1",
    );
    await user.type(screen.getByLabelText("Model name"), "mock");
    await user.type(
      screen.getByLabelText("Your sentence"),
      "a 2000 office build, keep FIX-CPU",
    );
    await user.click(screen.getByLabelText(/I understand this sends my sentence/));
    await user.click(screen.getByRole("button", { name: "Interpret sentence" }));
    await screen.findByRole("heading", { name: "Proposed request — review and confirm" });
    await user.click(screen.getByRole("button", { name: "Confirm and apply to the form" }));

    await waitFor(() =>
      expect(screen.getByLabelText("Total budget (USD)")).toHaveValue("2000.00"),
    );
    expect(screen.getByLabelText("Primary use")).toHaveValue("office");

    await user.click(screen.getByRole("button", { name: "Generate recommendations" }));
    expect(await screen.findByRole("heading", { name: "Recommended builds" })).toBeInTheDocument();
    if (bodies.length !== 1) {
      throw new Error(`expected one recommendation request, received ${bodies.length}`);
    }
    expect(bodies[0]).toMatchObject({
      snapshotId: "snap-r1",
      budgetMinor: 200000,
      currency: "USD",
      useCase: "office",
      lockedSkus: ["FIX-CPU"],
      excludedSkus: [],
      inventoryOnly: true,
    });
  });

  it("lays out constraints, options and verdict as three workbench regions", async () => {
    installFetch(handlerWith(() => jsonResponse(report())));
    const user = userEvent.setup();
    const { container } = render(<RecommendPage />);

    await screen.findByLabelText("Total budget (USD)");
    await user.type(screen.getByLabelText("Total budget (USD)"), "2000");
    await user.click(screen.getByRole("button", { name: "Generate recommendations" }));
    await screen.findByRole("heading", { name: "Recommended builds" });

    const constraints = container.querySelector(".workbench__constraints");
    const options = container.querySelector(".workbench__options");
    const verdict = container.querySelector(".workbench__verdict");
    expect(constraints).not.toBeNull();
    expect(options).not.toBeNull();
    expect(verdict).not.toBeNull();

    // The request controls live in constraints, the modules in options, and the
    // one report summary plus the rule matrix in the verdict rail.
    expect(constraints).toContainElement(screen.getByLabelText("Total budget (USD)"));
    expect(options).toContainElement(
      screen.getByRole("heading", { name: "Recommended builds" }),
    );
    expect(options?.querySelectorAll("article.spec-module")).toHaveLength(2);
    expect(verdict?.querySelector("dl.result-summary")).not.toBeNull();
    expect(options?.querySelector("dl.result-summary")).toBeNull();
    expect(
      screen.getByRole("heading", { name: "Verdict and score" }),
    ).toBeInTheDocument();
    expect(screen.getByText("6 valid candidates")).toBeInTheDocument();
    expect(screen.getByText("2 distinct builds")).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: "Gaming requires a discrete graphics card" }),
    ).toBeInTheDocument();
    expect(screen.getByText(/2 materially different builds found/)).toBeInTheDocument();
  });

  it("renders each option as a labelled spec module with mono numerals and a status chip", async () => {
    installFetch(handlerWith(() => jsonResponse(report())));
    const user = userEvent.setup();
    const { container } = render(<RecommendPage />);

    await screen.findByLabelText("Total budget (USD)");
    await user.type(screen.getByLabelText("Total budget (USD)"), "2000");
    await user.click(screen.getByRole("button", { name: "Generate recommendations" }));
    await screen.findByRole("heading", { name: "Recommended builds" });

    const module = container.querySelector("article.spec-module") as HTMLElement;
    expect(module).not.toBeNull();
    const caps = module.querySelector(".caps-label") as HTMLElement;
    expect(caps).toHaveAttribute("aria-hidden", "true");
    expect(caps.textContent).toBe("Build spec · option 1");

    const heading = module.querySelector("h3") as HTMLElement;
    expect(heading.textContent).toBe("1. Best overall");

    const rows = module.querySelector("dl.spec-rows") as HTMLElement;
    expect(rows).not.toBeNull();
    expect(rows.querySelectorAll("dd")).toHaveLength(4);
    expect(rows.querySelector("dd")?.textContent).toBe("US$278.00");

    const chip = module.querySelector(".badge--verdict") as HTMLElement;
    expect(chip.textContent).toContain("Compatible");
    expect(chip.querySelector(".badge__symbol")?.getAttribute("aria-hidden")).toBe(
      "true",
    );

    // The engine's own sentences and the report's version strings stay verbatim.
    expect(
      screen.getAllByText(/Total 2,000.00 USD leaves 0.00 USD\./).length,
    ).toBeGreaterThan(0);
    expect(screen.getAllByText(/check-1/).length).toBeGreaterThan(0);
    expect(screen.getByText(/24 combinations checked/)).toBeInTheDocument();
  });

  it("collapses the optional assistant by default and still reaches the budget", async () => {
    installFetch(handlerWith(() => jsonResponse(report())));
    const user = userEvent.setup();
    const { container } = render(<RecommendPage />);

    await screen.findByLabelText("Total budget (USD)");
    const details = container.querySelector("details.assistant") as HTMLDetailsElement;
    expect(details).not.toBeNull();
    expect(details.open).toBe(false);

    const summary = details.querySelector("summary") as HTMLElement;
    await user.click(summary);
    expect(details.open).toBe(true);
    expect(
      screen.getByRole("heading", { name: "No key configured" }),
    ).toBeInTheDocument();
    // The key field has a form owner, so the browser stops warning that a
    // password field is outside a form; the form never submits.
    const key = screen.getByLabelText("Provider key") as HTMLInputElement;
    expect(key.form).toBe(details.querySelector("form.assistant-transport"));

    await user.click(screen.getAllByRole("button", { name: "Continue without AI" })[0]);
    await waitFor(() =>
      expect(screen.getByLabelText("Total budget (USD)")).toHaveFocus(),
    );
  });
});

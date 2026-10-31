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
        /Recommendation complete: 2 options/.test(node.textContent ?? ""),
      ),
    ).toBe(true);
    expect(screen.getByRole("heading", { name: /1\. Best overall/ })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: /2\. Lowest cost/ })).toBeInTheDocument();
    expect(screen.getByText("Compare options")).toBeInTheDocument();
    expect(screen.getAllByText(/Highest weighted score for gaming/).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/check-1/).length).toBeGreaterThan(0);
    expect(screen.getByRole("button", { name: "Download JSON export" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Download Markdown plan" })).toBeInTheDocument();
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

  it("rejects an invalid budget without calling the service", async () => {
    const fetchMock = installFetch(handlerWith(() => jsonResponse(report())));
    const user = userEvent.setup();
    render(<RecommendPage />);

    await screen.findByLabelText("Total budget (USD)");
    await user.type(screen.getByLabelText("Total budget (USD)"), "1.234");
    await user.click(screen.getByRole("button", { name: "Generate recommendations" }));

    expect(await screen.findByText("Enter a budget")).toBeInTheDocument();
    expect(screen.getByText(/at most two decimals/)).toBeInTheDocument();
    expect(
      fetchMock.mock.calls.some(([url]) => String(url) === "/api/recommendations"),
    ).toBe(false);
  });
});

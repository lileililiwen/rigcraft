import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { DiagnosePage } from "./DiagnosePage";
import type {
  DiagnosisReport,
  ParsedList,
  Snapshot,
  SnapshotSummary,
} from "../types";

const provenance = {
  source: "user",
  reference: "csv:upgrade.csv",
  observedAt: "2026-10-31T12:00:00Z",
};

const summary: SnapshotSummary = {
  id: "snap-1",
  createdAt: "2026-10-31T12:00:00Z",
  revision: 1,
  parentSnapshotId: null,
  currency: "USD",
  sourceFilename: "upgrade.csv",
  recordCount: 9,
  unitCount: 9,
  valueMinor: 100000,
  note: null,
};

const snapshot: Snapshot = {
  id: "snap-1",
  createdAt: "2026-10-31T12:00:00Z",
  revision: 1,
  parentSnapshotId: null,
  currency: "USD",
  source: { filename: "upgrade.csv", sha256: "a".repeat(64) },
  note: null,
  records: [
    {
      id: "r-cpu",
      lot: 1,
      sku: "FIX-AM5-CPU-7600",
      category: "cpu",
      modelRaw: "Fixture AM5 CPU 7600",
      modelNormalized: "Fixture AM5 CPU 7600",
      quantity: 1,
      priceMinor: 24900,
      currency: "USD",
      specs: [],
      provenance: { model: provenance },
      match: { basis: "lexical", confidence: "unverified" },
    },
  ],
  totals: { recordCount: 1, unitCount: 1, valueMinor: 24900 },
};

const parsed: ParsedList = {
  matchVersion: "diagnosis-match-v1",
  snapshotId: "snap-1",
  currency: "USD",
  lineCount: 4,
  limits: { maxLines: 40, maxBytes: 8192, maxCandidates: 5 },
  lines: [
    {
      line: 1,
      raw: "gpu: Fixture GeForce RTX 4070",
      status: "ambiguous",
      category: "gpu",
      model: "Fixture GeForce RTX 4070",
      quantity: 1,
      priceMinor: null,
      issues: [],
      disambiguation:
        "The closest records add tokens you did not paste: twin, wind. Confirm the full model, add one of those tokens, or paste the SKU.",
      candidates: [
        {
          sku: "FIX-GPU-4070-TWIN",
          category: "gpu",
          model: "Fixture GeForce RTX 4070 Twin",
          priceMinor: 39900,
          currency: "USD",
          available: 1,
          coverage: 1,
          exact: false,
          sharedTokens: ["fixture", "geforce", "rtx", "4070"],
          extraTokens: ["twin"],
          missingTokens: [],
          provenance,
        },
        {
          sku: "FIX-GPU-4070-WIND",
          category: "gpu",
          model: "Fixture GeForce RTX 4070 Wind",
          priceMinor: 37900,
          currency: "USD",
          available: 1,
          coverage: 1,
          exact: false,
          sharedTokens: ["fixture", "geforce", "rtx", "4070"],
          extraTokens: ["wind"],
          missingTokens: [],
          provenance,
        },
      ],
    },
    {
      line: 2,
      raw: "gpu: Mystery X9000",
      status: "unmatched",
      category: "gpu",
      model: "Mystery X9000",
      quantity: 1,
      priceMinor: null,
      issues: [],
      disambiguation:
        "No snapshot record of this category shares enough of the pasted tokens. Paste the full model or the SKU, or describe the part by hand with its specifications.",
      candidates: [],
    },
    {
      line: 3,
      raw: "cpu: Fixture AM5 CPU 7600",
      status: "matched",
      category: "cpu",
      model: "Fixture AM5 CPU 7600",
      quantity: 1,
      priceMinor: null,
      issues: [],
      candidates: [
        {
          sku: "FIX-AM5-CPU-7600",
          category: "cpu",
          model: "Fixture AM5 CPU 7600",
          priceMinor: 24900,
          currency: "USD",
          available: 1,
          coverage: 1,
          exact: true,
          sharedTokens: ["am5", "cpu", "fixture", "7600"],
          extraTokens: ["shouldnotshow"],
          missingTokens: ["shouldnotshow2"],
          provenance,
        },
      ],
    },
    {
      line: 4,
      raw: "cpu: Fixture AM5 CPU 7600",
      status: "matched",
      category: "cpu",
      model: "Fixture AM5 CPU 7600",
      quantity: 1,
      priceMinor: null,
      issues: [
        { code: "duplicate_row", message: "Line 3 already describes this part; resolve one of them." },
      ],
      candidates: [],
      duplicateOf: 3,
    },
  ],
};

function diagnosis(): DiagnosisReport {
  return {
    id: "diag-1",
    createdAt: "2026-10-31T12:05:00Z",
    snapshotId: "snap-1",
    snapshotRevision: 1,
    ruleSetVersion: "compat-rules-v1",
    matchVersion: "diagnosis-match-v1",
    adviceVersion: "diagnosis-advice-v1",
    currency: "USD",
    build: [
      {
        row: 1,
        source: "snapshot",
        sku: "FIX-GPU-4070-TWIN",
        category: "gpu",
        model: "Fixture GeForce RTX 4070 Twin",
        quantity: 1,
        owned: true,
        reportedPriceMinor: 39900,
        specs: [],
        provenance: { model: provenance },
        match: { basis: "lexical", confidence: "unverified" },
      },
      {
        row: 2,
        source: "snapshot",
        sku: "FIX-AM5-CPU-7600",
        category: "cpu",
        model: "Fixture AM5 CPU 7600",
        quantity: 1,
        owned: true,
        reportedPriceMinor: 24900,
        specs: [],
        provenance: { model: provenance },
        match: { basis: "lexical", confidence: "unverified" },
      },
      {
        row: 3,
        source: "description",
        sku: "OWNED-03",
        category: "ram",
        model: "Fixture DDR5 16GB Kit",
        quantity: 1,
        owned: true,
        reportedPriceMinor: null,
        specs: [],
        provenance: { model: provenance },
        match: { basis: "user-described", confidence: "unverified" },
      },
    ],
    check: {
      id: "check-1",
      snapshotId: "diagnosis:described-build",
      snapshotRevision: 0,
      ruleSetVersion: "compat-rules-v1",
      checkedAt: "2026-10-31T12:05:00Z",
      selection: [],
      completeness: { status: "complete", missing: [] },
      overallStatus: "incompatible",
      rules: [
        {
          ruleId: "cpu-socket",
          ruleVersion: "compat-rules-v1",
          title: "CPU and motherboard Socket",
          category: "cpu",
          status: "incompatible",
          parts: [
            { recordId: "diagnosis:1", sku: "FIX-GPU-4070-TWIN", category: "gpu", model: "Fixture GeForce RTX 4070 Twin" },
            { recordId: "diagnosis:2", sku: "FIX-AM5-CPU-7600", category: "cpu", model: "Fixture AM5 CPU 7600" },
          ],
          evidence: [
            {
              part: {
                recordId: "diagnosis:2",
                sku: "FIX-AM5-CPU-7600",
                category: "cpu",
                model: "Fixture AM5 CPU 7600",
              },
              field: "socket",
              value: "AM5",
              provenance,
            },
          ],
          explanation: "Fixture conflict explanation for the browser fixture.",
          followUp: null,
          method: "Exact Socket comparison after trimming.",
        },
        {
          ruleId: "cpu-bios",
          ruleVersion: "compat-rules-v1",
          title: "BIOS support evidence",
          category: "cpu",
          status: "unknown",
          parts: [
            { recordId: "diagnosis:2", sku: "FIX-AM5-CPU-7600", category: "cpu", model: "Fixture AM5 CPU 7600" },
          ],
          evidence: [],
          explanation: "No sourced BIOS support fact is recorded.",
          followUp: "Confirm the motherboard's installed BIOS version supports Fixture AM5 CPU 7600.",
          method: "Matches the recorded BIOS support fact against the CPU model.",
        },
      ],
      summary: { compatible: 0, incompatible: 1, unknown: 1 },
      stale: false,
    },
    advice: [
      {
        row: 1,
        sku: "FIX-GPU-4070-TWIN",
        category: "gpu",
        model: "Fixture GeForce RTX 4070 Twin",
        action: "must_replace",
        conditional: false,
        reason: "The checker found a definite conflict involving this part.",
        ruleIds: ["cpu-socket"],
        evidence: [
          {
            part: {
              recordId: "diagnosis:1",
              sku: "FIX-GPU-4070-TWIN",
              category: "gpu",
              model: "Fixture GeForce RTX 4070 Twin",
            },
            field: "length",
            value: "300 mm",
            provenance,
          },
        ],
        cost: {
          currency: "USD",
          keepMinor: 0,
          replacementSku: "FIX-GPU-4070-WIND",
          replacementModel: "Fixture GeForce RTX 4070 Wind",
          replacementMinor: 37900,
          additionalMinor: 37900,
          note: "Cheapest replacement in this snapshot that the checker validates as compatible.",
        },
      },
      {
        row: 2,
        sku: "FIX-AM5-CPU-7600",
        category: "cpu",
        model: "Fixture AM5 CPU 7600",
        action: "should_upgrade",
        conditional: true,
        reason:
          "No definite conflict is recorded for this part, but the checker cannot confirm it: Confirm the motherboard's installed BIOS version.",
        ruleIds: ["cpu-bios"],
        evidence: [],
        verification: "Confirm the motherboard's installed BIOS version.",
        cost: {
          currency: "USD",
          keepMinor: 0,
          replacementSku: null,
          replacementModel: null,
          replacementMinor: null,
          additionalMinor: null,
          note: "Keep this part until the missing fact is confirmed; nothing is bought today.",
        },
      },
      {
        row: 3,
        sku: "OWNED-03",
        category: "ram",
        model: "Fixture DDR5 16GB Kit",
        action: "unverified",
        conditional: false,
        reason:
          "No rule that could run involves this part, so the checker did not evaluate it. Its compatibility is not verified and it is retained exactly as described.",
        ruleIds: [],
        evidence: [],
        cost: {
          currency: "USD",
          keepMinor: 0,
          replacementSku: null,
          replacementModel: null,
          replacementMinor: null,
          additionalMinor: null,
          note: "Already owned: keeping this part adds no purchase cost and takes no stock.",
        },
      },
    ],
    groups: { mustReplace: [1], shouldUpgrade: [2], keep: [], unverified: [3] },
    plan: {
      status: "validated",
      summary:
        "The checker validates the upgraded build as compatible. 1 step adds USD 379.00; no trade-in or resale value is assumed.",
      steps: [
        {
          row: 1,
          category: "gpu",
          fromSku: "FIX-GPU-4070-TWIN",
          fromModel: "Fixture GeForce RTX 4070 Twin",
          toSku: "FIX-GPU-4070-WIND",
          toModel: "Fixture GeForce RTX 4070 Wind",
          priceMinor: 37900,
          ruleIds: ["cpu-socket"],
          reason: "Replaces the part that resolves cpu-socket.",
        },
      ],
      options: [
        {
          row: 1,
          category: "gpu",
          sku: "FIX-GPU-4070-WIND",
          model: "Fixture GeForce RTX 4070 Wind",
          priceMinor: 37900,
          status: "compatible",
          resolves: ["cpu-socket"],
          ruleIds: [],
          followUps: [],
        },
      ],
      rejected: [],
      additionalCostMinor: 37900,
      currency: "USD",
      validation: {
        id: "check-plan",
        snapshotId: "diagnosis:described-build",
        snapshotRevision: 0,
        ruleSetVersion: "compat-rules-v1",
        checkedAt: "2026-10-31T12:05:00Z",
        selection: [],
        completeness: { status: "complete", missing: [] },
        overallStatus: "compatible",
        rules: [],
        summary: { compatible: 0, incompatible: 0, unknown: 0 },
        stale: false,
      },
      followUps: [],
      evaluated: 2,
      method: "Each candidate was substituted into the described build and re-checked.",
    },
    stale: false,
  };
}

function jsonResponse(data: unknown, status = 200): Promise<Response> {
  return Promise.resolve({
    ok: status < 400,
    status,
    text: async () => JSON.stringify(data),
  } as Response);
}

interface Calls {
  diagnoseBodies: Array<{ build: Array<{ sku?: string; source: string }> }>;
}

function installFetch(
  onDiagnose?: (body: string) => Promise<Response>,
): { calls: Calls } {
  const calls: Calls = { diagnoseBodies: [] };
  const fetchMock = vi.fn((input: RequestInfo | URL, init?: RequestInit) => {
    const url = String(input);
    if (url === "/api/inventory/snapshots") {
      return jsonResponse([summary]);
    }
    if (url === "/api/inventory/snapshots/snap-1") {
      return jsonResponse(snapshot);
    }
    if (url === "/api/diagnoses/parse" && init?.method === "POST") {
      return jsonResponse(parsed);
    }
    if (url === "/api/diagnoses" && init?.method === "POST") {
      const body = String(init.body);
      calls.diagnoseBodies.push(JSON.parse(body));
      return onDiagnose ? onDiagnose(body) : jsonResponse(diagnosis());
    }
    return jsonResponse({ error: { code: "unexpected", message: url } }, 500);
  });
  vi.stubGlobal("fetch", fetchMock);
  return { calls };
}

async function matchList(user: ReturnType<typeof userEvent.setup>) {
  const textarea = await screen.findByLabelText("Parts list");
  await user.type(textarea, "gpu: Fixture GeForce RTX 4070");
  await user.click(screen.getByRole("button", { name: "Match this list" }));
  const heading = await screen.findByRole("heading", {
    name: "3. Review the matches",
  });
  await waitFor(() => expect(heading).toHaveFocus());
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("build diagnosis page", () => {
  it("resolves an ambiguous line by keyboard, leaves the unmatched line out and validates the plan", async () => {
    const { calls } = installFetch();
    const user = userEvent.setup();
    render(<DiagnosePage />);

    expect(
      await screen.findByRole("heading", { level: 1, name: "Diagnose an existing build" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("main", { name: "Diagnose an existing build" }),
    ).toBeInTheDocument();
    // This route renders no form; nothing may claim to be one without a name.
    expect(screen.queryAllByRole("form")).toHaveLength(0);
    await matchList(user);

    const ambiguous = screen.getByRole("radio", {
      name: /Fixture GeForce RTX 4070 Twin/,
    });
    const wind = screen.getByRole("radio", {
      name: /Fixture GeForce RTX 4070 Wind/,
    });
    expect(ambiguous).not.toBeChecked();
    expect(wind).not.toBeChecked();
    expect(
      screen.getByText(/add tokens you did not paste: twin, wind/),
    ).toBeInTheDocument();
    expect(screen.getAllByText(/adds twin/).length).toBeGreaterThan(0);
    expect(screen.queryByText(/adds shouldnotshow/)).not.toBeInTheDocument();
    expect(screen.queryByText(/missing shouldnotshow2/)).not.toBeInTheDocument();

    // The exact match is a preselected suggestion, not a silent confirmation.
    expect(
      screen.getByRole("radio", { name: /Fixture AM5 CPU 7600/ }),
    ).toBeChecked();

    // Keyboard resolution: focus the candidate and select it with the keyboard.
    wind.focus();
    await user.keyboard(" ");
    expect(wind).toBeChecked();

    // Each line has its own explicit "leave out" decision; choose the second.
    const skipChoices = screen.getAllByRole("radio", {
      name: "Leave this line out of the diagnosis",
    });
    expect(skipChoices.length).toBeGreaterThanOrEqual(2);
    await user.click(skipChoices[1]);

    expect(screen.getByText(/Left out/)).toBeInTheDocument();
    expect(screen.getAllByText(/Line 3 already describes this part/).length).toBeGreaterThan(0);

    await user.click(screen.getByRole("button", { name: "Diagnose this build" }));
    const heading = await screen.findByRole("heading", { name: "Build diagnosis" });
    await waitFor(() => expect(heading).toHaveFocus());

    expect(calls.diagnoseBodies).toHaveLength(1);
    const sent = calls.diagnoseBodies[0].build;
    expect(sent.map((part) => part.sku)).toEqual([
      "FIX-GPU-4070-WIND",
      "FIX-AM5-CPU-7600",
    ]);
    expect(sent.every((part) => part.source === "snapshot")).toBe(true);

    expect(
      screen.getByRole("heading", { name: /Must replace/ }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: /Should upgrade — verify first/ }),
    ).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: /Fine as is/ })).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: /Not verified — no rule evaluated/ }),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/No rule that could run involves this part, so the checker did not evaluate it/),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/2 of 3 described parts were checked by at least one rule/),
    ).toBeInTheDocument();
    expect(
      screen.queryByText("Every described part was checked by at least one rule."),
    ).not.toBeInTheDocument();
    const unverifiedGroup = screen
      .getByRole("heading", { name: /Not verified — no rule evaluated/ })
      .closest("section");
    expect(unverifiedGroup?.querySelector(".advice-group__head .badge")).toHaveTextContent(
      "Not verified",
    );
    const checkedAt = document.querySelector('time[datetime="2026-10-31T12:05:00Z"]');
    expect(checkedAt).not.toBeNull();
    expect(checkedAt).toHaveTextContent("2026-10-31T12:05:00Z");
    expect(
      screen.getByText(/The checker found a definite conflict involving this part/),
    ).toBeInTheDocument();
    expect(screen.getAllByText("Validated").length).toBeGreaterThan(0);
    expect(
      screen.getByRole("link", { name: "cpu-socket" }),
    ).toHaveAttribute("href", "#rule-cpu-socket");
    expect(screen.getByText(/USD 379.00/)).toBeInTheDocument();
    expect(
      screen.getAllByRole("status").some((node) =>
        /Diagnosis complete: incompatible/.test(node.textContent ?? ""),
      ),
    ).toBe(true);

    // Editing the list invalidates the match review and the displayed report.
    await user.type(screen.getByLabelText("Parts list"), " ");
    await waitFor(() =>
      expect(
        screen.queryByRole("heading", { name: "Build diagnosis" }),
      ).not.toBeInTheDocument(),
    );
  });

  it("renders a stable numbered skeleton with a review empty state", async () => {
    installFetch();
    render(<DiagnosePage />);
    await screen.findByLabelText("Parts list");

    for (const title of [
      "1. Choose an inventory snapshot",
      "2. Paste the parts list",
      "3. Review the matches",
      "4. Add a part by hand",
      "5. Read the diagnosis",
    ]) {
      expect(screen.getByRole("heading", { name: title })).toBeInTheDocument();
    }
    expect(
      screen.getByText(/Match a pasted list in step 2 to review its lines here/),
    ).toBeInTheDocument();
  });

  it("never preselects an ambiguous line and blocks an unresolved review", async () => {
    installFetch();
    const user = userEvent.setup();
    render(<DiagnosePage />);
    await matchList(user);

    await user.click(screen.getByRole("button", { name: "Diagnose this build" }));
    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent("Line 1 is ambiguous");
    expect(alert).toHaveTextContent("Line 2 has no match");
    await waitFor(() => expect(alert).toHaveFocus());
    expect(screen.queryByRole("heading", { name: "Build diagnosis" })).not.toBeInTheDocument();
  });

  it("uses the shipped sample's parts and guards matching and diagnosis until there is input", async () => {
    installFetch();
    const user = userEvent.setup();
    render(<DiagnosePage />);

    const textarea = await screen.findByLabelText("Parts list");
    expect(textarea).toHaveAttribute(
      "placeholder",
      "cpu: AMD Ryzen 7 7800X3D\ngpu: NVIDIA GeForce RTX 4070 Founders Edition\npsu: Corsair RM750e",
    );
    expect(screen.getByLabelText("Specifications you have confirmed")).toHaveAttribute(
      "placeholder",
      "socket=AM5;tdp=120W;igpu=yes",
    );

    const match = screen.getByRole("button", { name: "Match this list" });
    expect(match).toBeDisabled();
    expect(match).toHaveAttribute("aria-describedby", "diagnose-match-hint");
    expect(
      screen.getByText("Paste at least one part above to enable matching."),
    ).toBeInTheDocument();

    const diagnose = screen.getByRole("button", { name: "Diagnose this build" });
    expect(diagnose).toBeDisabled();
    expect(diagnose).toHaveAttribute("aria-describedby", "diagnose-run-hint");
    expect(
      screen.getByText("Add a matched line or a part by hand to enable the diagnosis."),
    ).toBeInTheDocument();

    await user.type(textarea, "cpu: AMD Ryzen 7 7800X3D");
    expect(match).toBeEnabled();
    expect(
      screen.getByText("Matching never guesses a SKU for an ambiguous line."),
    ).toBeInTheDocument();
  });

  it("adds an owned part by hand and sends its specifications", async () => {
    const { calls } = installFetch();
    const user = userEvent.setup();
    render(<DiagnosePage />);
    await screen.findByLabelText("Snapshot");

    await user.selectOptions(
      screen.getByLabelText("Category"),
      "motherboard",
    );
    await user.type(screen.getByLabelText("Model"), "Fixture B650 Board");
    await user.clear(screen.getByLabelText("Quantity"));
    await user.type(screen.getByLabelText("Quantity"), "1");
    await user.type(
      screen.getByLabelText("Specifications you have confirmed"),
      "socket=AM5;memory_type=DDR5",
    );
    await user.click(screen.getByRole("button", { name: "Add this part" }));
    expect(screen.getAllByText(/Fixture B650 Board/).length).toBeGreaterThan(0);
    expect(screen.getByText("1 part added by hand")).toBeInTheDocument();
    expect(screen.getByText(/1 unit/)).toBeInTheDocument();
    expect(screen.getByText(/1 part will be diagnosed/)).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Diagnose this build" }));
    await screen.findByRole("heading", { name: "Build diagnosis" });
    expect(calls.diagnoseBodies).toHaveLength(1);
    expect(calls.diagnoseBodies[0].build).toEqual([
      {
        source: "owned",
        category: "motherboard",
        model: "Fixture B650 Board",
        quantity: 1,
        specs: "socket=AM5;memory_type=DDR5",
      },
    ]);
  });

  it("states all-checked coverage once when every part was evaluated", async () => {
    installFetch(async () => {
      const report = diagnosis();
      report.groups = {
        mustReplace: [1],
        shouldUpgrade: [2],
        keep: [3],
        unverified: [],
      };
      return jsonResponse(report);
    });
    const user = userEvent.setup();
    render(<DiagnosePage />);
    await screen.findByLabelText("Snapshot");

    await user.type(screen.getByLabelText("Model"), "Fixture B650 Board");
    await user.click(screen.getByRole("button", { name: "Add this part" }));
    await user.click(screen.getByRole("button", { name: "Diagnose this build" }));

    await screen.findByRole("heading", { name: "Build diagnosis" });
    expect(
      screen.getAllByText("Every described part was checked by at least one rule."),
    ).toHaveLength(1);
    expect(
      screen.queryByText(/of 3 described parts were checked by at least one rule/),
    ).not.toBeInTheDocument();
  });

  it("keeps the entered work when the diagnosis fails", async () => {
    let fail = true;
    installFetch(async () => {
      if (fail) {
        fail = false;
        return jsonResponse(
          {
            error: {
              code: "internal_error",
              message: "The inventory store could not complete this request.",
            },
          },
          500,
        );
      }
      return jsonResponse(diagnosis());
    });
    const user = userEvent.setup();
    render(<DiagnosePage />);
    await screen.findByLabelText("Snapshot");

    await user.type(screen.getByLabelText("Model"), "Fixture B650 Board");
    await user.click(screen.getByRole("button", { name: "Add this part" }));
    await user.click(screen.getByRole("button", { name: "Diagnose this build" }));

    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent("Request failed");
    await waitFor(() => expect(alert).toHaveFocus());
    expect(screen.getAllByText(/Fixture B650 Board/).length).toBeGreaterThan(0);

    await user.click(screen.getByRole("button", { name: "Diagnose this build" }));
    expect(
      await screen.findByRole("heading", { name: "Build diagnosis" }),
    ).toBeInTheDocument();
  });

  it("explains the empty inventory state with a link to import", async () => {
    installFetch();
    vi.stubGlobal(
      "fetch",
      vi.fn((input: RequestInfo | URL) => {
        if (String(input) === "/api/inventory/snapshots") {
          return jsonResponse([]);
        }
        return jsonResponse({ error: { code: "unexpected", message: String(input) } }, 500);
      }),
    );
    render(<DiagnosePage />);
    expect(await screen.findByText(/No inventory snapshot exists yet/)).toBeInTheDocument();
    const link = screen.getByRole("link", { name: "Import a CSV in Inventory" });
    expect(link).toHaveAttribute("href", "/inventory");
    expect(
      screen.queryByRole("heading", { name: "3. Review the matches" }),
    ).not.toBeInTheDocument();
  });
});

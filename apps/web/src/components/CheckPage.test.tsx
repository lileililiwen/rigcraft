import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { CheckPage } from "./CheckPage";
import type {
  CheckReport,
  RecordItem,
  SelectionItem,
  Snapshot,
} from "../types";

const provenance = {
  source: "user",
  reference: "csv:fixture.csv",
  observedAt: "2026-10-31T12:00:00Z",
};

function record(
  id: string,
  category: string,
  sku: string,
  model: string,
  quantity = 3,
): RecordItem {
  return {
    id,
    lot: 1,
    sku,
    category,
    modelRaw: model,
    modelNormalized: model,
    quantity,
    priceMinor: 10000,
    currency: "USD",
    specs: [
      {
        key: "socket",
        raw: "socket=AM5",
        text: "AM5",
        status: "confirmed",
        provenance,
      },
    ],
    provenance: { model: provenance },
    match: { basis: "lexical", confidence: "unverified" },
  };
}

const snapshot: Snapshot = {
  id: "snap-1",
  createdAt: "2026-10-31T12:00:00Z",
  revision: 1,
  parentSnapshotId: null,
  currency: "USD",
  source: { filename: "fixture.csv", sha256: "a".repeat(64) },
  note: null,
  records: [
    record("r-cpu", "cpu", "FIX-CPU-AM5", "Fixture AM5 CPU"),
    record("r-board", "motherboard", "FIX-MB-AM4", "Fixture AM4 Board"),
    record("r-gpu", "gpu", "FIX-GPU", "Fixture GPU"),
    record("r-ram", "ram", "FIX-RAM", "Fixture DDR5 Kit"),
    record("r-psu", "psu", "FIX-PSU", "Fixture PSU"),
    record("r-case", "case", "FIX-CASE", "Fixture Case"),
    record("r-cooler", "cooler", "FIX-COOLER", "Fixture Cooler"),
    record("r-storage", "storage", "FIX-SSD", "Fixture SSD"),
  ],
  totals: { recordCount: 8, unitCount: 24, valueMinor: 80000 },
};

const summary = {
  id: snapshot.id,
  createdAt: snapshot.createdAt,
  revision: 1,
  parentSnapshotId: null,
  currency: "USD",
  sourceFilename: "fixture.csv",
  recordCount: 8,
  unitCount: 24,
  valueMinor: 80000,
  note: null,
};

function reportFor(selection: SelectionItem[], attempt: number): CheckReport {
  return {
    id: `check-${attempt}`,
    snapshotId: "snap-1",
    snapshotRevision: 1,
    ruleSetVersion: "compat-rules-v1",
    checkedAt: "2026-10-31T12:01:00Z",
    selection: selection.map((item) => {
      const source = snapshot.records.find((entry) => entry.sku === item.sku);
      return {
        recordId: source?.id ?? "unknown",
        sku: item.sku,
        category: source?.category ?? "unknown",
        model: source?.modelRaw ?? item.sku,
        quantity: item.quantity,
        lots: 1,
        available: source?.quantity ?? 0,
      };
    }),
    completeness: {
      status: "incomplete",
      missing: [
        {
          category: "storage",
          reason: "Startup storage is required.",
          followUp: "Select startup storage from this snapshot.",
        },
      ],
    },
    overallStatus: "incompatible",
    rules: [
      {
        ruleId: "cpu-socket",
        ruleVersion: "compat-rules-v1",
        title: "CPU and motherboard Socket",
        category: "cpu",
        status: "incompatible",
        parts: [
          {
            recordId: "r-cpu",
            sku: "FIX-CPU-AM5",
            category: "cpu",
            model: "Fixture AM5 CPU",
          },
          {
            recordId: "r-board",
            sku: "FIX-MB-AM4",
            category: "motherboard",
            model: "Fixture AM4 Board",
          },
        ],
        evidence: [
          {
            part: {
              recordId: "r-cpu",
              sku: "FIX-CPU-AM5",
              category: "cpu",
              model: "Fixture AM5 CPU",
            },
            field: "socket",
            value: "AM5",
            provenance,
          },
          {
            part: {
              recordId: "r-board",
              sku: "FIX-MB-AM4",
              category: "motherboard",
              model: "Fixture AM4 Board",
            },
            field: "socket",
            value: "AM4",
            provenance,
          },
        ],
        explanation: "CPU declares Socket AM5 but the motherboard declares Socket AM4.",
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
          {
            recordId: "r-board",
            sku: "FIX-MB-AM4",
            category: "motherboard",
            model: "Fixture AM4 Board",
          },
        ],
        evidence: [
          {
            part: {
              recordId: "r-board",
              sku: "FIX-MB-AM4",
              category: "motherboard",
              model: "Fixture AM4 Board",
            },
            field: "bios_supports",
            value: null,
            provenance: null,
          },
        ],
        explanation: "No sourced BIOS support fact is recorded for Fixture AM5 CPU.",
        followUp: "Confirm the motherboard's installed BIOS version supports Fixture AM5 CPU.",
        method: "Matches the recorded BIOS support fact against the CPU model and SKU.",
      },
    ],
    summary: { compatible: 0, incompatible: 1, unknown: 1 },
    stale: false,
    currentSnapshotRevision: 1,
  };
}

function jsonResponse(data: unknown, status = 200): Promise<Response> {
  return Promise.resolve({
    ok: status < 400,
    status,
    text: async () => JSON.stringify(data),
  } as Response);
}

function installFetch(handler: (url: string, init?: RequestInit) => Promise<Response>) {
  const fetchMock = vi.fn((input: RequestInfo | URL, init?: RequestInit) =>
    handler(String(input), init),
  );
  vi.stubGlobal("fetch", fetchMock);
  return fetchMock;
}

function standardHandler(
  check: (selection: SelectionItem[], attempt: number) => Promise<Response>,
) {
  let attempts = 0;
  return async (url: string, init?: RequestInit) => {
    if (url === "/api/inventory/snapshots") {
      return jsonResponse([summary]);
    }
    if (url === "/api/inventory/snapshots/snap-1") {
      return jsonResponse(snapshot);
    }
    if (url === "/api/builds/check" && init?.method === "POST") {
      attempts += 1;
      const body = JSON.parse(String(init.body)) as { selection: SelectionItem[] };
      return check(body.selection, attempts);
    }
    return jsonResponse({ error: { code: "unexpected", message: url } }, 500);
  };
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("compatibility check page", () => {
  it("checks a selection, announces the verdict, and marks it stale after an edit", async () => {
    installFetch(
      standardHandler((selection, attempt) =>
        jsonResponse(reportFor(selection, attempt)),
      ),
    );
    const user = userEvent.setup();
    render(<CheckPage />);

    expect(
      screen.getByRole("heading", { level: 1, name: "Compatibility check" }),
    ).toBeInTheDocument();
    const snapshotSelect = await screen.findByLabelText("Snapshot");
    await waitFor(() => expect(snapshotSelect).toHaveValue("snap-1"));
    expect(
      await screen.findByLabelText("Quantity for FIX-CPU-AM5"),
    ).toBeInTheDocument();

    await user.click(screen.getByRole("radio", { name: /Fixture AM5 CPU/ }));
    await user.click(screen.getByRole("button", { name: "Check compatibility" }));

    const heading = await screen.findByRole("heading", {
      name: "Compatibility result",
    });
    await waitFor(() => expect(heading).toHaveFocus());
    expect(
      screen.getAllByRole("status").some((node) => /Check complete: Incompatible/.test(node.textContent ?? "")),
    ).toBe(true);
    expect(screen.getByText("Conflict diagnostics")).toBeInTheDocument();
    expect(
      screen.getAllByText(/CPU declares Socket AM5 but the motherboard declares Socket AM4/)
        .length,
    ).toBeGreaterThan(0);
    expect(screen.getByText("Facts to confirm")).toBeInTheDocument();
    expect(
      screen.getAllByText(/Confirm the motherboard's installed BIOS version/).length,
    ).toBeGreaterThan(0);
    expect(screen.getAllByText("Not provided").length).toBeGreaterThan(0);
    expect(screen.getAllByText(/csv:fixture.csv/).length).toBeGreaterThan(0);
    expect(screen.getByText("Incomplete build")).toBeInTheDocument();
    expect(screen.getByText(/Startup storage is required\./)).toBeInTheDocument();
    expect(
      screen.getByText("Select startup storage from this snapshot."),
    ).toBeInTheDocument();

    expect(screen.getByRole("heading", { name: "Rule matrix" })).toBeInTheDocument();
    expect(screen.getAllByText("Rule method and version")).toHaveLength(2);
    expect(screen.getAllByText("AM5").length).toBeGreaterThan(0);
    expect(screen.getAllByText("AM4").length).toBeGreaterThan(0);
    const summary = screen.getByText("Rules passed").closest("dl");
    expect(summary).toHaveTextContent("Parts not evaluated");
    expect(summary?.querySelectorAll("dd")[3]).toHaveTextContent("0");
    expect(screen.getByText("1 unit selected")).toBeInTheDocument();

    const quantity = screen.getByLabelText("Quantity for FIX-CPU-AM5");
    await user.clear(quantity);
    await user.type(quantity, "2");
    expect(screen.getByText("2 units selected")).toBeInTheDocument();
    expect(screen.getByText("This verdict is stale.")).toBeInTheDocument();
    expect(
      screen.getAllByRole("status").some((node) =>
        /previous verdict is stale/.test(node.textContent ?? ""),
      ),
    ).toBe(true);

    await user.click(screen.getByRole("button", { name: "Re-check this selection" }));
    await waitFor(() =>
      expect(screen.queryByText("This verdict is stale.")).not.toBeInTheDocument(),
    );
    expect(screen.getByText(/Quantity for FIX-CPU-AM5/)).toBeInTheDocument();
  });

  it("names its main region and the parts form once a snapshot loads", async () => {
    installFetch(
      standardHandler((selection, attempt) =>
        jsonResponse(reportFor(selection, attempt)),
      ),
    );
    const { container } = render(<CheckPage />);

    expect(
      screen.getByRole("main", { name: "Compatibility check" }),
    ).toBeInTheDocument();
    await screen.findByLabelText("Snapshot");

    expect(screen.getByRole("form", { name: "2. Select parts" })).toBeInTheDocument();
    // The name matcher only matches a form with a non-empty accessible name, so
    // equal counts prove that none of the rendered forms is unnamed.
    expect(screen.getAllByRole("form", { name: /./ })).toHaveLength(
      container.querySelectorAll("form").length,
    );
  });

  it("counts selected parts that no rule evaluated", async () => {
    installFetch(
      standardHandler((selection, attempt) =>
        jsonResponse(reportFor(selection, attempt)),
      ),
    );
    const user = userEvent.setup();
    render(<CheckPage />);

    await screen.findByLabelText("Snapshot");
    await user.click(screen.getByRole("radio", { name: /Fixture AM5 CPU/ }));
    await user.click(screen.getByRole("radio", { name: /Fixture Case/ }));
    await user.click(screen.getByRole("button", { name: "Check compatibility" }));

    await screen.findByRole("heading", { name: "Compatibility result" });
    const summary = screen.getByText("Parts not evaluated").closest("dl");
    expect(summary?.querySelectorAll("dd")[3]).toHaveTextContent("1");
  });

  it("explains the empty inventory state with a link to import", async () => {
    installFetch(async (url) => {
      if (url === "/api/inventory/snapshots") {
        return jsonResponse([]);
      }
      return jsonResponse({ error: { code: "unexpected", message: url } }, 500);
    });
    render(<CheckPage />);
    expect(
      await screen.findByText(/No inventory snapshot exists yet/),
    ).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Import a CSV in Inventory" })).toHaveAttribute(
      "href",
      "/inventory",
    );
  });

  it("blocks an empty selection and keeps the page usable", async () => {
    installFetch(
      standardHandler((selection, attempt) =>
        jsonResponse(reportFor(selection, attempt)),
      ),
    );
    const user = userEvent.setup();
    render(<CheckPage />);

    await screen.findByLabelText("Snapshot");
    await user.click(screen.getByRole("button", { name: "Check compatibility" }));
    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent("Select at least one part");
    await waitFor(() => expect(alert).toHaveFocus());
  });

  it("preserves the selection and offers retry after a server failure", async () => {
    let fail = true;
    installFetch(
      standardHandler(async (selection, attempt) => {
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
        return jsonResponse(reportFor(selection, attempt));
      }),
    );
    const user = userEvent.setup();
    render(<CheckPage />);

    await screen.findByLabelText("Snapshot");
    const cpu = screen.getByRole("radio", { name: /Fixture AM5 CPU/ });
    await user.click(cpu);
    await user.click(screen.getByRole("button", { name: "Check compatibility" }));

    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent("Request failed");
    expect(cpu).toBeChecked();
    await user.click(screen.getByRole("button", { name: "Retry check" }));
    expect(
      await screen.findByRole("heading", { name: "Compatibility result" }),
    ).toBeInTheDocument();
  });
});

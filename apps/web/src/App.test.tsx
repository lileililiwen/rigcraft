import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import type { Preview, Snapshot } from "./types";

function response(data: unknown, status = 200): Promise<Response> {
  return Promise.resolve({
    ok: status < 400,
    status,
    text: async () => JSON.stringify(data),
  } as Response);
}

const preview: Preview = {
  previewId: "preview-1",
  createdAt: "2026-10-31T12:00:00Z",
  expiresAt: "2026-11-01T12:00:00Z",
  currency: "USD",
  filename: "stock.csv",
  columns: ["sku", "category", "model", "quantity", "price"],
  mapping: {
    sku: "sku",
    category: "category",
    model: "model",
    quantity: "quantity",
    price: "price",
  },
  rows: [
    {
      row: 2,
      status: "valid",
      values: {
        sku: "CPU-7800X3D",
        category: "cpu",
        model: "AMD Ryzen 7 7800X3D",
        quantity: "2",
        price: "189.00",
        currency: "USD",
        specs: "socket=AM5",
      },
      normalized: {
        sku: "CPU-7800X3D",
        category: "cpu",
        modelNormalized: "AMD Ryzen 7 7800X3D",
        quantity: 2,
        priceMinor: 18900,
        currency: "USD",
      },
      specs: [
        {
          key: "socket",
          raw: "socket=AM5",
          text: "AM5",
          status: "confirmed",
          provenance: {
            source: "user",
            reference: "csv:stock.csv",
            observedAt: "2026-10-31T12:00:00Z",
          },
        },
      ],
      errors: [],
      duplicateGroupId: null,
      provenance: {
        sku: {
          source: "user",
          reference: "csv:stock.csv",
          observedAt: "2026-10-31T12:00:00Z",
        },
      },
      match: { basis: "lexical", confidence: "unverified" },
    },
  ],
  duplicateGroups: [],
  summary: { total: 1, valid: 1, errors: 0, duplicateRows: 0 },
};

const snapshot: Snapshot = {
  id: "11111111-2222-4333-8444-555555555555",
  createdAt: "2026-10-31T12:05:00Z",
  revision: 1,
  parentSnapshotId: null,
  currency: "USD",
  source: { filename: "stock.csv", sha256: "a".repeat(64) },
  note: null,
  totals: { recordCount: 1, unitCount: 2, valueMinor: 37800 },
  records: [
    {
      id: "record-1",
      lot: 1,
      sku: "CPU-7800X3D",
      category: "cpu",
      modelRaw: "AMD Ryzen 7 7800X3D",
      modelNormalized: "AMD Ryzen 7 7800X3D",
      quantity: 2,
      priceMinor: 18900,
      currency: "USD",
      specs: preview.rows[0].specs,
      provenance: preview.rows[0].provenance,
      match: { basis: "lexical", confidence: "unverified" },
    },
  ],
};

afterEach(() => {
  vi.unstubAllGlobals();
  window.history.pushState({}, "", "/");
});

describe("inventory page", () => {
  it("imports, reviews, commits, and shows provenance and version", async () => {
    const fetchMock = vi.fn((input: RequestInfo | URL) => {
      const url = String(input);
      if (url === "/api/imports") {
        return response(preview);
      }
      if (url === "/api/imports/preview-1/commit") {
        return response({ snapshotId: snapshot.id, snapshot });
      }
      if (url === "/api/catalog/match") {
        return response({ matches: [] });
      }
      if (url === "/api/inventory/snapshots") {
        return response([
          {
            id: snapshot.id,
            createdAt: snapshot.createdAt,
            revision: 1,
            parentSnapshotId: null,
            currency: "USD",
            sourceFilename: "stock.csv",
            recordCount: 1,
            unitCount: 2,
            valueMinor: 37800,
            note: null,
          },
        ]);
      }
      return response({ error: { code: "unexpected", message: url } }, 500);
    });
    vi.stubGlobal("fetch", fetchMock);

    const user = userEvent.setup();
    render(<App />);
    expect(
      screen.getByRole("heading", { level: 1, name: "Inventory" }),
    ).toBeInTheDocument();
    const sampleLink = screen.getByRole("link", { name: "Download sample CSV" });
    expect(sampleLink).toHaveAttribute("href", "/api/imports/sample");
    expect(sampleLink).toHaveAttribute("aria-describedby", "sample-licence");
    expect(screen.getByText(/CC BY 4\.0/)).toBeInTheDocument();
    expect(screen.getByText(/2026-10-31/)).toBeInTheDocument();

    const file = new File(["sku,category,model,quantity,price\n"], "stock.csv", {
      type: "text/csv",
    });
    await user.upload(screen.getByLabelText("Inventory CSV (UTF-8)"), file);
    await user.click(screen.getByRole("button", { name: "Preview CSV" }));

    expect(await screen.findByRole("heading", { name: "Review import" })).toBeInTheDocument();
    expect(screen.getByText(/Preview ready: 1 row/)).toBeInTheDocument();
    expect(screen.getByText("US$189.00")).toBeInTheDocument();
    expect(screen.getByText("socket")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Commit snapshot" }));
    const heading = await screen.findByRole("heading", {
      name: /Committed snapshot/,
    });
    await waitFor(() => expect(heading).toHaveFocus());
    expect(screen.getByText(/sha256 aaaaaaaaaaaa/)).toBeInTheDocument();
    expect(screen.getAllByText(/User-provided/).length).toBeGreaterThan(0);
    expect(screen.getByText("US$378.00")).toBeInTheDocument();
    expect(screen.getByText(/Snapshot history/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Adjust stock or price" })).toBeInTheDocument();
  });

  it("preserves the selected file and retries after a failed preview", async () => {
    let attempts = 0;
    const fetchMock = vi.fn((input: RequestInfo | URL) => {
      const url = String(input);
      if (url === "/api/imports") {
        attempts += 1;
        if (attempts === 1) {
          return response(
            {
              error: {
                code: "mapping_missing",
                message: "Required columns are missing: category.",
                field: "category",
              },
            },
            400,
          );
        }
        return response(preview);
      }
      if (url === "/api/catalog/match") {
        return response({ matches: [] });
      }
      if (url === "/api/inventory/snapshots") {
        return response([]);
      }
      return response({ error: { code: "unexpected", message: url } }, 500);
    });
    vi.stubGlobal("fetch", fetchMock);

    const user = userEvent.setup();
    render(<App />);
    const file = new File(["sku\n"], "stock.csv", { type: "text/csv" });
    await user.upload(screen.getByLabelText("Inventory CSV (UTF-8)"), file);
    await user.click(screen.getByRole("button", { name: "Preview CSV" }));

    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent("Required columns are missing: category.");
    expect(alert).toHaveTextContent("mapping_missing");
    expect(screen.getByText(/Selected stock.csv/)).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Retry" }));
    expect(await screen.findByRole("heading", { name: "Review import" })).toBeInTheDocument();
    expect(attempts).toBe(2);
  });
});

describe("diagnosis route", () => {
  it("renders the diagnosis page for /diagnose and marks the nav item", async () => {
    window.history.pushState({}, "", "/diagnose");
    vi.stubGlobal(
      "fetch",
      vi.fn((input: RequestInfo | URL) => {
        if (String(input) === "/api/inventory/snapshots") {
          return response([]);
        }
        return response({ error: { code: "unexpected", message: String(input) } }, 500);
      }),
    );
    render(<App />);
    expect(
      await screen.findByRole("heading", {
        level: 1,
        name: "Diagnose an existing build",
      }),
    ).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Diagnose" })).toHaveAttribute(
      "aria-current",
      "page",
    );
    await waitFor(() => expect(document.title).toBe("RigCraft build diagnosis"));
  });
});

describe("catalog route", () => {
  it("renders the catalog page for /catalog and marks the nav item", async () => {
    window.history.pushState({}, "", "/catalog");
    vi.stubGlobal(
      "fetch",
      vi.fn((input: RequestInfo | URL) => {
        if (String(input) === "/api/catalog/datasets") {
          return response([]);
        }
        return response({ error: { code: "unexpected", message: String(input) } }, 500);
      }),
    );
    render(<App />);
    expect(
      await screen.findByRole("heading", { level: 1, name: "Catalog" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Catalog" })).toHaveAttribute(
      "aria-current",
      "page",
    );
    await waitFor(() => expect(document.title).toBe("RigCraft catalog"));
  });
});

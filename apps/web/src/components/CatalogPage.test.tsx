import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { CatalogDataset, CatalogPartDetail, CatalogReport } from "../types";
import { CatalogPage } from "./CatalogPage";

function response(data: unknown, status = 200): Promise<Response> {
  return Promise.resolve({
    ok: status < 400,
    status,
    text: async () => JSON.stringify(data),
  } as Response);
}

const dataset: CatalogDataset = {
  id: "dataset-1",
  name: "synthetic-fixture",
  sourceUrl: "https://example.invalid/catalog",
  commit: "fixture-1",
  license: "CC0-1.0",
  retrievedAt: "2026-11-01T00:00:00Z",
  importedAt: "2026-11-01T00:00:00Z",
  version: "fixture-1",
  rowCount: 3,
  categoryCounts: { cpu: 2, gpu: 1 },
  active: true,
};

const report: CatalogReport = {
  accepted: true,
  replaced: false,
  dataset,
  partCount: 3,
  categoryCounts: { cpu: 2, gpu: 1 },
  specKeys: ["socket", "tdp"],
};

const part: CatalogPartDetail = {
  part: {
    id: "part-1",
    datasetId: dataset.id,
    externalId: "MPN-CPU-1",
    sku: "SYN-CPU-1",
    category: "cpu",
    model: "Synthetic CPU One",
    modelNormalized: "Synthetic CPU One",
    manufacturer: "Synthetic Labs",
    specs: [
      {
        key: "socket",
        raw: "socket=AM5",
        text: "AM5",
        status: "confirmed",
        provenance: {
          source: "catalog:synthetic-fixture@fixture-1",
          reference: "https://example.invalid/cpu-one",
          observedAt: "2026-11-01T00:00:00Z",
        },
      },
    ],
    sourceUrl: "https://example.invalid/cpu-one",
    verifiedAt: "2026-11-01T00:00:00Z",
  },
  dataset,
  provenance: {
    model: {
      source: "catalog:synthetic-fixture@fixture-1",
      reference: "https://example.invalid/cpu-one",
      observedAt: "2026-11-01T00:00:00Z",
    },
  },
};

afterEach(() => {
  vi.unstubAllGlobals();
});

function catalogFile() {
  return new File(
    [
      JSON.stringify({
        format: "rigcraft-catalog",
        version: 1,
        dataset: { name: "synthetic-fixture", license: "CC0-1.0" },
        parts: [{ sku: "SYN-CPU-1", category: "cpu", model: "Synthetic CPU One" }],
      }),
    ],
    "catalog.json",
    { type: "application/json" },
  );
}

describe("catalog page", () => {
  it("uploads a document, shows the accepted report and the registry licence", async () => {
    let datasets: CatalogDataset[] = [];
    const fetchMock = vi.fn((input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input);
      if (url === "/api/catalog/datasets" && init?.method === "POST") {
        datasets = [dataset];
        return response(report);
      }
      if (url === "/api/catalog/datasets") {
        return response(datasets);
      }
      return response({ error: { code: "unexpected", message: url } }, 500);
    });
    vi.stubGlobal("fetch", fetchMock);

    const user = userEvent.setup();
    render(<CatalogPage />);
    expect(await screen.findByText("No dataset is stored in this instance yet.")).toBeInTheDocument();
    await user.upload(screen.getByLabelText("Catalog JSON document"), catalogFile());
    expect(screen.getByText(/Selected catalog.json/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Import catalog" })).toBeDisabled();
    await user.click(screen.getByLabelText(/I have the right to use this data/));
    await user.click(screen.getByRole("button", { name: "Import catalog" }));

    const accepted = await screen.findByRole("heading", { name: "Catalog accepted" });
    await waitFor(() => expect(accepted).toHaveFocus());
    expect(screen.getAllByText(/3 parts/).length).toBeGreaterThan(0);
    expect(screen.getAllByText("CC0-1.0").length).toBeGreaterThan(0);
    expect(screen.getByText(/catalog:synthetic-fixture@fixture-1/)).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Catalog" })).toHaveAttribute(
      "aria-current",
      "page",
    );
    const call = fetchMock.mock.calls.find(
      (entry) => String(entry[0]) === "/api/catalog/datasets" && entry[1]?.method === "POST",
    );
    const body = JSON.parse(String(call?.[1]?.body)) as {
      acknowledged: boolean;
      catalog: { format: string };
    };
    expect(body.acknowledged).toBe(true);
    expect(body.catalog.format).toBe("rigcraft-catalog");
  });

  it("shows a refused document with its row and field and adds nothing", async () => {
    const fetchMock = vi.fn((input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input);
      if (url === "/api/catalog/datasets" && init?.method !== "POST") {
        return response([]);
      }
      return response(
        {
          error: {
            code: "catalog_invalid",
            message: "The catalog document was refused. Nothing was stored.",
            row: 1,
            details: {
              accepted: false,
              rows: [
                {
                  row: 1,
                  field: "model",
                  code: "required",
                  message: "`model` is required.",
                },
              ],
            },
          },
        },
        422,
      );
    });
    vi.stubGlobal("fetch", fetchMock);

    const user = userEvent.setup();
    render(<CatalogPage />);
    await user.upload(screen.getByLabelText("Catalog JSON document"), catalogFile());
    await user.click(screen.getByLabelText(/I have the right to use this data/));
    await user.click(screen.getByRole("button", { name: "Import catalog" }));

    const alert = await screen.findByRole("alert");
    await waitFor(() => expect(alert).toHaveFocus());
    expect(alert).toHaveTextContent("Catalog refused");
    expect(alert).toHaveTextContent("Row 1");
    expect(alert).toHaveTextContent("model");
    expect(screen.queryByRole("heading", { name: "Catalog accepted" })).not.toBeInTheDocument();
  });

  it("deactivates a dataset and removes one only after confirmation", async () => {
    let active = true;
    const calls: Array<{ method: string; url: string }> = [];
    const fetchMock = vi.fn((input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input);
      const method = init?.method ?? "GET";
      calls.push({ method, url });
      if (url === "/api/catalog/datasets" && method === "GET") {
        return response([{ ...dataset, active }]);
      }
      if (url === `/api/catalog/datasets/${dataset.id}` && method === "PATCH") {
        active = false;
        return response({ ...dataset, active });
      }
      if (url.startsWith(`/api/catalog/datasets/${dataset.id}?`) && method === "DELETE") {
        return response({ removed: true });
      }
      return response({ error: { code: "unexpected", message: url } }, 500);
    });
    vi.stubGlobal("fetch", fetchMock);

    const user = userEvent.setup();
    render(<CatalogPage />);
    expect(await screen.findByText(dataset.name)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Deactivate" }));
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Activate" })).toBeInTheDocument(),
    );
    expect(calls.some((call) => call.method === "PATCH")).toBe(true);

    await user.click(screen.getByRole("button", { name: "Remove" }));
    expect(screen.getByText(/Remove synthetic-fixture\?/)).toBeInTheDocument();
    expect(screen.getByText(/Inventory, snapshots/)).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Confirm removal" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Confirm removal" }));
    await waitFor(() =>
      expect(calls.some((call) => call.method === "DELETE")).toBe(true),
    );
    const deletion = calls.find((call) => call.method === "DELETE");
    expect(deletion?.url).toContain("confirm=remove");
  });

  it("shows registry counts and refuses to search an empty query", async () => {
    const fetchMock = vi.fn((input: RequestInfo | URL) => {
      const url = String(input);
      if (url === "/api/catalog/datasets") {
        return response([dataset]);
      }
      return response({ error: { code: "unexpected", message: url } }, 500);
    });
    vi.stubGlobal("fetch", fetchMock);

    const user = userEvent.setup();
    render(<CatalogPage />);
    expect(await screen.findByText(dataset.name)).toBeInTheDocument();

    const registry = screen
      .getByRole("heading", { name: "Datasets" })
      .closest("section");
    expect(registry as HTMLElement).toHaveTextContent("Datasets1");
    expect(registry as HTMLElement).toHaveTextContent("Parts3");
    expect(registry as HTMLElement).toHaveTextContent("Active1");
    for (const column of [
      "Licence",
      "Version",
      "Parts",
      "Categories",
      "Active",
      "Actions",
    ]) {
      expect(
        within(registry as HTMLElement).getByRole("columnheader", { name: column }),
      ).toBeInTheDocument();
    }

    const search = screen.getByRole("button", { name: "Search catalog" });
    expect(search).toBeDisabled();
    expect(search).toHaveAttribute("aria-describedby", "catalog-search-hint");
    expect(
      screen.getByText("Enter a SKU, MPN, model or word to enable search."),
    ).toBeInTheDocument();

    await user.type(screen.getByLabelText("SKU, MPN, model or word"), "S");
    expect(search).toBeEnabled();
    expect(
      screen.getByText(
        "Only active datasets are searched; an exact SKU or MPN ranks first.",
      ),
    ).toBeInTheDocument();
  });

  it("searches the active catalog and opens a stored part with provenance", async () => {
    const fetchMock = vi.fn((input: RequestInfo | URL) => {
      const url = String(input);
      if (url === "/api/catalog/datasets") {
        return response([dataset]);
      }
      if (url.startsWith("/api/catalog/search?")) {
        return response({
          query: "SYN-CPU-1",
          category: "cpu",
          limit: 20,
          offset: 0,
          total: 1,
          hits: [{ part: part.part, dataset }],
        });
      }
      if (url === "/api/catalog/parts/part-1") {
        return response(part);
      }
      return response({ error: { code: "unexpected", message: url } }, 500);
    });
    vi.stubGlobal("fetch", fetchMock);

    const user = userEvent.setup();
    render(<CatalogPage />);
    await user.type(screen.getByLabelText("SKU, MPN, model or word"), "SYN-CPU-1");
    await user.selectOptions(screen.getByLabelText("Category (optional)"), "cpu");
    await user.click(screen.getByRole("button", { name: "Search catalog" }));

    expect(await screen.findByText(/1 ranked hit/)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "View part" }));
    const heading = await screen.findByRole("heading", { name: "Synthetic CPU One" });
    await waitFor(() => expect(heading).toHaveFocus());
    expect(screen.getAllByText("Catalog").length).toBeGreaterThan(0);
    expect(
      screen.getByText("https://example.invalid/cpu-one"),
    ).toBeInTheDocument();
    expect(screen.getByText(/Synthetic Labs/)).toBeInTheDocument();
    expect(screen.getByText("socket")).toBeInTheDocument();
  });
});

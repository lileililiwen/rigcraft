import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Snapshot, SnapshotSummary } from "../types";

vi.mock("../api", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../api")>();
  return {
    ...actual,
    listSnapshots: vi.fn(),
    getSnapshot: vi.fn(),
  };
});

import { getSnapshot, listSnapshots } from "../api";
import { InventoryPage } from "./InventoryPage";

function summary(
  id: string,
  revision: number,
  recordCount: number,
  valueMinor: number,
): SnapshotSummary {
  return {
    id,
    createdAt: `2026-11-0${revision}T00:00:00Z`,
    revision,
    parentSnapshotId: revision > 1 ? "snap-1" : null,
    currency: "USD",
    sourceFilename: `stock-r${revision}.csv`,
    recordCount,
    unitCount: recordCount,
    valueMinor,
    note: revision > 1 ? "Stock recount" : null,
  };
}

const newest = summary("snap-2", 2, 10, 212560);
const older = summary("snap-1", 1, 8, 150000);

function snapshot(entry: SnapshotSummary): Snapshot {
  return {
    id: entry.id,
    createdAt: entry.createdAt,
    revision: entry.revision,
    parentSnapshotId: entry.parentSnapshotId,
    currency: entry.currency,
    source: { filename: entry.sourceFilename, sha256: "abc123" },
    note: entry.note,
    records: Array.from({ length: entry.recordCount }, (_, index) => ({
      id: `${entry.id}-r${index}`,
      lot: 1,
      sku: `CPU-${index}`,
      category: "cpu",
      modelRaw: "AMD Ryzen 7 7800X3D",
      modelNormalized: "AMD Ryzen 7 7800X3D",
      quantity: 1,
      priceMinor: 44900,
      currency: "USD",
      specs: [],
      provenance: {},
      match: { basis: "lexical" as const, confidence: "verified" as const },
    })),
    totals: {
      recordCount: entry.recordCount,
      unitCount: entry.unitCount,
      valueMinor: entry.valueMinor,
    },
  };
}

beforeEach(() => {
  vi.mocked(listSnapshots).mockReset();
  vi.mocked(getSnapshot).mockReset();
  vi.mocked(listSnapshots).mockResolvedValue([newest, older]);
  vi.mocked(getSnapshot).mockImplementation(async (id: string) =>
    snapshot(id === newest.id ? newest : older),
  );
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("inventory console", () => {
  it("opens the newest revision on load, shows its state and enables export", async () => {
    render(<InventoryPage />);

    const bar = await screen.findByRole("heading", { name: "Snapshot state" });
    expect(getSnapshot).toHaveBeenCalledWith(newest.id);
    const barSection = bar.closest("section");
    expect(barSection).not.toBeNull();
    expect(barSection as HTMLElement).toHaveTextContent("Revision2");
    expect(barSection as HTMLElement).toHaveTextContent("Records10");
    expect(barSection as HTMLElement).toHaveTextContent("Units in stock10");
    expect(barSection as HTMLElement).toHaveTextContent("US$2,125.60");
    expect(barSection as HTMLElement).toHaveTextContent("Latest revision");
    expect(barSection as HTMLElement).toHaveTextContent("stock-r2.csv");
    expect(barSection as HTMLElement).toHaveTextContent("2026-11-02T00:00:00Z");

    // Export is usable without an extra click, and the copy says so.
    expect(screen.getByRole("button", { name: "Download CSV" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "Download JSON" })).toBeEnabled();
    expect(screen.getByText("Ready: revision 2, 10 records.")).toBeInTheDocument();

    // The initial open announces the state without taking focus.
    expect(document.getElementById("snapshot-title")).not.toHaveFocus();
    expect(
      screen
        .getAllByRole("status")
        .some((node) =>
          (node.textContent ?? "").includes(
            "Opened the newest snapshot, revision 2: 10 records, US$2,125.60 stock value.",
          ),
        ),
    ).toBe(true);
  });

  it("lists every revision as a card and opens an earlier one explicitly", async () => {
    const user = userEvent.setup();
    render(<InventoryPage />);
    await screen.findByRole("heading", { name: "Snapshot state" });

    const history = screen.getByRole("heading", { name: "Snapshot history" }).closest("section");
    expect(history).not.toBeNull();
    expect(
      screen.getByRole("heading", { name: "Revision 2", level: 3 }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: "Revision 1", level: 3 }),
    ).toBeInTheDocument();
    expect(screen.getByText("Viewing now")).toBeInTheDocument();
    expect(screen.getAllByText("Stock recount").length).toBeGreaterThan(0);

    const views = screen.getAllByRole("button", { name: /^View / });
    expect(views).toHaveLength(2);
    await user.click(views[1]);

    await waitFor(() =>
      expect(
        screen.getByRole("heading", {
          name: "Committed snapshot · revision 1",
          level: 2,
        }),
      ).toBeInTheDocument(),
    );
    expect(screen.getByText("Earlier revision")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Download CSV" })).toBeEnabled();
  });

  it("keeps the import panel and guards Preview CSV until a file and currency are valid", async () => {
    const user = userEvent.setup();
    render(<InventoryPage />);
    await screen.findByRole("heading", { name: "Snapshot state" });

    const preview = screen.getByRole("button", { name: "Preview CSV" });
    expect(preview).toBeDisabled();
    expect(preview).toHaveAttribute("aria-describedby", "preview-hint");
    expect(
      screen.getByText("Choose a CSV file to enable Preview CSV."),
    ).toBeInTheDocument();

    await user.upload(
      screen.getByLabelText("Inventory CSV (UTF-8)"),
      new File(["sku,category,model,quantity,price"], "stock.csv", { type: "text/csv" }),
    );
    expect(preview).toBeEnabled();
    expect(
      screen.getByText("Preview validates every row and writes nothing."),
    ).toBeInTheDocument();

    const currency = screen.getByLabelText("Batch currency");
    await user.clear(currency);
    await user.type(currency, "12");
    expect(preview).toBeDisabled();
    expect(
      screen.getByText(
        "Enter a three-letter ISO 4217 currency code to enable Preview CSV.",
      ),
    ).toBeInTheDocument();

    await user.clear(currency);
    await user.type(currency, "eur");
    expect(preview).toBeEnabled();
  });

  it("closes the opened snapshot for good when a new import starts", async () => {
    const user = userEvent.setup();
    render(<InventoryPage />);
    await screen.findByRole("heading", { name: "Snapshot state" });

    await user.click(screen.getByRole("button", { name: "Start a new import" }));

    expect(screen.queryByRole("heading", { name: "Snapshot state" })).toBeNull();
    expect(screen.getByRole("button", { name: "Download CSV" })).toBeDisabled();
    expect(
      screen.getByText(
        "No snapshot is open. Choose View on a revision in Snapshot history, or commit an import.",
      ),
    ).toBeInTheDocument();
    // The history survives, and the list was fetched once: nothing reopens it.
    expect(screen.getByRole("heading", { name: "Snapshot history" })).toBeInTheDocument();
    expect(listSnapshots).toHaveBeenCalledTimes(1);
  });

  it("names its main region and every form, including the editors", async () => {
    const user = userEvent.setup();
    const { container } = render(<InventoryPage />);
    await screen.findByRole("heading", { name: "Snapshot state" });

    expect(screen.getByRole("main", { name: "Inventory" })).toBeInTheDocument();
    expect(screen.getByRole("form", { name: "Import a CSV" })).toBeInTheDocument();
    expect(
      screen.getByRole("form", { name: "Restore from a backup" }),
    ).toBeInTheDocument();
    // The name matcher only matches a form with a non-empty accessible name, so
    // equal counts prove that none of the rendered forms is unnamed.
    expect(screen.getAllByRole("form", { name: /./ })).toHaveLength(
      container.querySelectorAll("form").length,
    );

    await user.click(screen.getAllByRole("button", { name: /^Adjust stock or price$/ })[0]);
    expect(
      screen.getByRole("form", { name: /^Create a new revision from / }),
    ).toBeInTheDocument();
  });
});

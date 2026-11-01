import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { BackupInspection, Snapshot, SnapshotSummary } from "../types";

vi.mock("../api", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../api")>();
  return {
    ...actual,
    downloadSnapshotExport: vi.fn(),
    downloadBackup: vi.fn(),
    inspectBackup: vi.fn(),
    restoreBackup: vi.fn(),
  };
});

import {
  ApiFailure,
  downloadBackup,
  downloadSnapshotExport,
  inspectBackup,
  restoreBackup,
} from "../api";
import { DataPortability } from "./DataPortability";

const snapshot: Snapshot = {
  id: "snap-1",
  createdAt: "2026-11-01T00:00:00Z",
  revision: 2,
  parentSnapshotId: "snap-0",
  currency: "USD",
  source: { filename: "stock.csv", sha256: "abc" },
  note: null,
  records: [
    {
      id: "r1",
      lot: 1,
      sku: "CPU-1",
      category: "cpu",
      modelRaw: "Example CPU",
      modelNormalized: "Example CPU",
      quantity: 1,
      priceMinor: 10000,
      currency: "USD",
      specs: [],
      provenance: {},
      match: { basis: "lexical", confidence: "unverified" },
    },
  ],
  totals: { recordCount: 1, unitCount: 1, valueMinor: 10000 },
};

const history: SnapshotSummary[] = [
  {
    id: "snap-1",
    createdAt: "2026-11-01T00:00:00Z",
    revision: 2,
    parentSnapshotId: "snap-0",
    currency: "USD",
    sourceFilename: "stock.csv",
    recordCount: 1,
    unitCount: 1,
    valueMinor: 10000,
    note: null,
  },
];

const inspection: BackupInspection = {
  fileName: "backup.sqlite3",
  sizeBytes: 4096,
  integrity: "ok",
  counts: {
    schemaVersion: 4,
    snapshotCount: 3,
    latestRevision: 2,
    createdAt: "2026-11-01T00:00:00Z",
    recordCount: 30,
    lotCount: 10,
  },
};

function renderPanel(overrides: Partial<Parameters<typeof DataPortability>[0]> = {}) {
  const onRestored = vi.fn();
  const { container } = render(
    <DataPortability
      snapshot={snapshot}
      history={history}
      onRestored={onRestored}
      {...overrides}
    />,
  );
  return { onRestored, container };
}

beforeEach(() => {
  vi.mocked(downloadSnapshotExport).mockReset();
  vi.mocked(downloadBackup).mockReset();
  vi.mocked(inspectBackup).mockReset();
  vi.mocked(restoreBackup).mockReset();
});

describe("DataPortability", () => {
  it("disables export until a snapshot is selected", () => {
    renderPanel({ snapshot: null });
    expect(screen.getByRole("button", { name: "Download CSV" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Download JSON" })).toBeDisabled();
    expect(
      screen.getByText(
        "No snapshot is open. Choose View on a revision in Snapshot history, or commit an import.",
      ),
    ).toBeInTheDocument();
  });

  it("exposes the static hint as a note and keeps real action results live", async () => {
    vi.mocked(downloadBackup).mockResolvedValue("rigcraft-backup-2026.sqlite3");
    const { container } = renderPanel();

    const note = screen.getByRole("note");
    expect(note).toHaveTextContent(
      "Exports and backups are created by this local service only.",
    );
    // Nothing is announced on load: the status region is empty.
    expect(screen.getByRole("status")).toBeEmptyDOMElement();
    expect(
      screen.getByRole("form", { name: "Restore from a backup" }),
    ).toBeInTheDocument();
    // The name matcher only matches a form with a non-empty accessible name, so
    // equal counts prove that none of the rendered forms is unnamed.
    expect(screen.getAllByRole("form", { name: /./ })).toHaveLength(
      container.querySelectorAll("form").length,
    );

    await userEvent.click(screen.getByRole("button", { name: "Download backup" }));
    await waitFor(() =>
      expect(screen.getByRole("status")).toHaveTextContent(
        "rigcraft-backup-2026.sqlite3",
      ),
    );
    expect(note).toHaveTextContent("Downloaded rigcraft-backup-2026.sqlite3.");
  });

  it("names the restore confirmation form", async () => {
    vi.mocked(inspectBackup).mockResolvedValue(inspection);
    const { container } = renderPanel();

    await userEvent.upload(
      screen.getByLabelText("SQLite backup file"),
      new File(["sqlite"], "backup.sqlite3", { type: "application/vnd.sqlite3" }),
    );
    await userEvent.click(screen.getByRole("button", { name: "Inspect backup" }));
    await screen.findByRole("heading", { name: /Backup inspected/ });

    expect(
      screen.getByRole("form", { name: "Confirm replacing the inventory" }),
    ).toBeInTheDocument();
    expect(screen.getAllByRole("form", { name: /./ })).toHaveLength(
      container.querySelectorAll("form").length,
    );
  });

  it("exports both formats and reports the filename", async () => {
    vi.mocked(downloadSnapshotExport).mockResolvedValue({
      filename: "rigcraft-inventory-snapshot-r2-snap-1.csv",
      note: "CSV export is lossy.",
    });
    renderPanel();
    await userEvent.click(screen.getByRole("button", { name: "Download CSV" }));
    expect(downloadSnapshotExport).toHaveBeenCalledWith("snap-1", "csv");
    await waitFor(() =>
      expect(screen.getByRole("status")).toHaveTextContent(
        "rigcraft-inventory-snapshot-r2-snap-1.csv",
      ),
    );
    await userEvent.click(screen.getByRole("button", { name: "Download JSON" }));
    expect(downloadSnapshotExport).toHaveBeenCalledWith("snap-1", "json");
  });

  it("downloads a whole-database backup", async () => {
    vi.mocked(downloadBackup).mockResolvedValue("rigcraft-backup-2026.sqlite3");
    renderPanel();
    await userEvent.click(screen.getByRole("button", { name: "Download backup" }));
    expect(downloadBackup).toHaveBeenCalled();
    await waitFor(() =>
      expect(screen.getByRole("status")).toHaveTextContent(
        "rigcraft-backup-2026.sqlite3",
      ),
    );
  });

  it("previews an upload and refuses to replace until RESTORE is typed", async () => {
    vi.mocked(inspectBackup).mockResolvedValue(inspection);
    vi.mocked(restoreBackup).mockResolvedValue({
      restored: true,
      sourceSchemaVersion: 4,
      preRestorePath: "/data/inventory.sqlite3.pre-restore-20261101T000000000000Z",
      counts: inspection.counts,
    });
    const { onRestored } = renderPanel();

    await userEvent.upload(
      screen.getByLabelText("SQLite backup file"),
      new File(["sqlite"], "backup.sqlite3", { type: "application/vnd.sqlite3" }),
    );
    await userEvent.click(screen.getByRole("button", { name: "Inspect backup" }));

    expect(await screen.findByRole("heading", { name: /Backup inspected/ })).toBeVisible();
    expect(screen.getByText("3")).toBeInTheDocument();
    expect(screen.getByText("30")).toBeInTheDocument();
    const replace = screen.getByRole("button", { name: "Replace this inventory" });
    expect(replace).toBeDisabled();

    const confirm = screen.getByLabelText(/Type RESTORE to confirm/);
    await userEvent.type(confirm, "restore");
    expect(replace).toBeEnabled();
    await userEvent.click(replace);

    expect(restoreBackup).toHaveBeenCalledTimes(1);
    expect(
      await screen.findByRole("heading", { name: "Inventory restored" }),
    ).toBeVisible();
    expect(
      screen.getByText(
        "/data/inventory.sqlite3.pre-restore-20261101T000000000000Z",
      ),
    ).toBeInTheDocument();
    expect(onRestored).toHaveBeenCalledWith(
      expect.objectContaining({ restored: true }),
    );
  });

  it("shows the documented code when an upload is rejected and keeps the current inventory", async () => {
    vi.mocked(inspectBackup).mockRejectedValue(
      new ApiFailure(400, {
        code: "backup_not_sqlite",
        message: "The uploaded file is not a SQLite database.",
      }),
    );
    const { onRestored } = renderPanel();

    await userEvent.upload(
      screen.getByLabelText("SQLite backup file"),
      new File(["text"], "notes.sqlite3", { type: "application/vnd.sqlite3" }),
    );
    await userEvent.click(screen.getByRole("button", { name: "Inspect backup" }));

    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent("This backup cannot be restored");
    expect(alert).toHaveTextContent("backup_not_sqlite");
    expect(screen.queryByRole("heading", { name: /Backup inspected/ })).toBeNull();
    expect(onRestored).not.toHaveBeenCalled();
    expect(screen.getByText("Ready: revision 2, 1 record.")).toBeInTheDocument();
  });
});

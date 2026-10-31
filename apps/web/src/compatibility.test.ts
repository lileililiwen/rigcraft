import { describe, expect, it } from "vitest";
import type { CheckReport, RecordItem, Snapshot } from "./types";
import {
  buildSelection,
  emptySelection,
  isRecordSelected,
  reportIsStale,
  selectionErrors,
  verdictMeta,
} from "./compatibility";

function record(id: string, category: string, sku: string): RecordItem {
  return {
    id,
    lot: 1,
    sku,
    category,
    modelRaw: `Fixture ${category}`,
    modelNormalized: `Fixture ${category}`,
    quantity: 3,
    priceMinor: 1000,
    currency: "USD",
    specs: [],
    provenance: {},
    match: { basis: "lexical", confidence: "unverified" },
  };
}

const records = [
  record("r-cpu", "cpu", "CPU-A"),
  record("r-board", "motherboard", "MB-A"),
  record("r-ram", "ram", "RAM-A"),
];

const snapshot: Snapshot = {
  id: "snap-1",
  createdAt: "2026-10-31T12:00:00Z",
  revision: 1,
  parentSnapshotId: null,
  currency: "USD",
  source: { filename: "fixture.csv", sha256: "a".repeat(64) },
  note: null,
  records,
  totals: { recordCount: 3, unitCount: 9, valueMinor: 3000 },
};

function report(selection: Array<{ sku: string; quantity: number }>): CheckReport {
  return {
    id: "check-1",
    snapshotId: "snap-1",
    snapshotRevision: 1,
    ruleSetVersion: "compat-rules-v1",
    checkedAt: "2026-10-31T12:01:00Z",
    selection: selection.map((item) => ({
      recordId: "r-cpu",
      sku: item.sku,
      category: "cpu",
      model: "Fixture cpu",
      quantity: item.quantity,
      lots: 1,
      available: 3,
    })),
    completeness: { status: "complete", missing: [] },
    overallStatus: "compatible",
    rules: [],
    summary: { compatible: 0, incompatible: 0, unknown: 0 },
    stale: false,
    currentSnapshotRevision: 1,
  };
}

describe("compatibility selection helpers", () => {
  it("aggregates the same SKU across lots and reports over-stock selection", () => {
    const state = {
      single: { cpu: "r-cpu" },
      multi: { "r-ram": true },
      quantities: { "r-cpu": "2", "r-ram": "3" },
    };
    expect(isRecordSelected(state, records[0])).toBe(true);
    expect(buildSelection(snapshot, state)).toEqual([
      { sku: "CPU-A", quantity: 2 },
      { sku: "RAM-A", quantity: 3 },
    ]);
    expect(selectionErrors(snapshot, state)).toEqual([]);

    const over = { ...state, quantities: { "r-cpu": "4", "r-ram": "3" } };
    expect(selectionErrors(snapshot, over)[0]).toContain("needs 4 unit");
  });

  it("requires at least one selected part", () => {
    expect(selectionErrors(snapshot, emptySelection())).toEqual([
      "Select at least one part from the snapshot.",
    ]);
  });

  it("marks a report stale after a selection or snapshot change", () => {
    const checked = report([{ sku: "CPU-A", quantity: 1 }]);
    expect(reportIsStale(checked, "snap-1", [{ sku: "CPU-A", quantity: 1 }])).toBe(
      false,
    );
    expect(reportIsStale(checked, "snap-1", [{ sku: "CPU-A", quantity: 2 }])).toBe(
      true,
    );
    expect(reportIsStale(checked, "snap-2", [{ sku: "CPU-A", quantity: 1 }])).toBe(
      true,
    );
  });

  it("labels verdicts with a symbol and text, not color alone", () => {
    expect(verdictMeta("compatible")).toMatchObject({ symbol: "✓", label: "Compatible" });
    expect(verdictMeta("incompatible")).toMatchObject({
      symbol: "✕",
      label: "Incompatible",
    });
    expect(verdictMeta("unknown")).toMatchObject({ symbol: "?", label: "Unknown" });
  });
});

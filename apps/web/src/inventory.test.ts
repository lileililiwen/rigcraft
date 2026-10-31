import { describe, expect, it } from "vitest";
import type { Preview, PreviewRow } from "./types";
import {
  canCommit,
  defaultDecisions,
  formatMoney,
  priceToMinor,
  rowIssues,
  rowsNeedingResolution,
} from "./inventory";

function row(overrides: Partial<PreviewRow> & { row: number }): PreviewRow {
  return {
    status: "valid",
    values: {
      sku: `SKU-${overrides.row}`,
      category: "cpu",
      model: "Example CPU",
      quantity: "1",
      price: "10.00",
      currency: "USD",
      specs: "socket=AM5",
    },
    normalized: {
      sku: `SKU-${overrides.row}`,
      category: "cpu",
      modelNormalized: "Example CPU",
      quantity: 1,
      priceMinor: 1000,
      currency: "USD",
    },
    specs: [],
    errors: [],
    duplicateGroupId: null,
    provenance: {},
    match: { basis: "lexical", confidence: "unverified" },
    ...overrides,
  };
}

function preview(rows: PreviewRow[]): Preview {
  return {
    previewId: "p1",
    createdAt: "2026-10-31T00:00:00Z",
    expiresAt: "2026-11-01T00:00:00Z",
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
    rows,
    duplicateGroups: [],
    summary: {
      total: rows.length,
      valid: rows.filter((entry) => entry.status === "valid").length,
      errors: rows.filter((entry) => entry.status === "error").length,
      duplicateRows: rows.filter((entry) => entry.status === "duplicate").length,
    },
  };
}

describe("inventory decisions", () => {
  it("excludes error rows and includes valid rows by default", () => {
    const data = preview([
      row({ row: 2 }),
      row({ row: 3, status: "error", errors: [{ field: "price", code: "required", message: "Required" }] }),
    ]);
    expect(defaultDecisions(data)).toEqual({ 2: "accept", 3: "exclude" });
  });

  it("requires a resolution only when two duplicate rows are included", () => {
    const data = preview([
      row({ row: 2, status: "duplicate", duplicateGroupId: "dup-1" }),
      row({ row: 3, status: "duplicate", duplicateGroupId: "dup-1" }),
    ]);
    data.duplicateGroups = [{ groupId: "dup-1", sku: "A", rows: [2, 3] }];
    const decisions = defaultDecisions(data);
    expect(rowsNeedingResolution(data, decisions, {})).toEqual(["dup-1"]);
    expect(rowsNeedingResolution(data, { 2: "accept", 3: "exclude" }, {})).toEqual([]);
    expect(rowsNeedingResolution(data, decisions, { "dup-1": "merge" })).toEqual([]);
  });

  it("clears a preview error when the field is corrected and validates the edit", () => {
    const bad = row({
      row: 4,
      status: "error",
      errors: [{ field: "quantity", code: "quantity_format", message: "Whole number" }],
    });
    expect(rowIssues(bad, undefined)).toHaveLength(1);
    expect(rowIssues(bad, { quantity: "3" })).toHaveLength(0);
    expect(rowIssues(bad, { quantity: "3.5" })[0].code).toBe("quantity_format");
  });

  it("blocks commit while an included row or duplicate group is unresolved", () => {
    const duplicateRows = [
      row({ row: 2, status: "duplicate", duplicateGroupId: "dup-1" }),
      row({ row: 3, status: "duplicate", duplicateGroupId: "dup-1" }),
    ];
    const data = preview(duplicateRows);
    data.duplicateGroups = [{ groupId: "dup-1", sku: "A", rows: [2, 3] }];
    const decisions = defaultDecisions(data);
    const blocked = canCommit(data, decisions, {}, {}, {});
    expect(blocked.ready).toBe(false);
    expect(blocked.reason).toMatch(/merge, replace, or keep separate/i);
    expect(canCommit(data, decisions, { "dup-1": "keep_separate" }, {}, {}).ready).toBe(true);
    expect(canCommit(data, { 2: "accept", 3: "exclude" }, {}, {}, {}).ready).toBe(true);
  });

  it("formats money and parses price input deterministically", () => {
    expect(priceToMinor("1299")).toBe(129900);
    expect(priceToMinor("1299.5")).toBe(129950);
    expect(priceToMinor("1.234")).toBeNull();
    expect(formatMoney(129900, "USD")).toMatch(/1,299/);
  });
});

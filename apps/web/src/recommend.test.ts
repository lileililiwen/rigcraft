import { describe, expect, it } from "vitest";
import type { NoSolution, SearchStats } from "./types";
import {
  bindingLabel,
  formatWeight,
  parseBudgetMinor,
  plural,
  requestSignature,
  searchSummary,
} from "./recommend";

const search: SearchStats = {
  limit: 100_000,
  nodesExamined: 1200,
  leavesEvaluated: 64,
  validBuilds: 12,
  limited: false,
  method: "depth first",
};

describe("recommendation helpers", () => {
  it("parses a budget into integer minor units", () => {
    expect(parseBudgetMinor("1200")).toBe(120000);
    expect(parseBudgetMinor(" 1299.5 ")).toBe(129950);
    expect(parseBudgetMinor("0.05")).toBe(5);
    expect(parseBudgetMinor("0")).toBeNull();
    expect(parseBudgetMinor("-5")).toBeNull();
    expect(parseBudgetMinor("1.234")).toBeNull();
    expect(parseBudgetMinor("")).toBeNull();
  });

  it("accepts comma and space grouping only when the layout is unambiguous", () => {
    expect(parseBudgetMinor("1,800")).toBe(180000);
    expect(parseBudgetMinor("1 800")).toBe(180000);
    expect(parseBudgetMinor("1,800.50")).toBe(180050);
    expect(parseBudgetMinor("1 800.5")).toBe(180050);
    expect(parseBudgetMinor("1,234,567.89")).toBe(123456789);
    expect(parseBudgetMinor("12,345")).toBe(1234500);
  });

  it("refuses an ambiguous or malformed grouping instead of guessing", () => {
    expect(parseBudgetMinor("1.800,50")).toBeNull();
    expect(parseBudgetMinor("1,80")).toBeNull();
    expect(parseBudgetMinor("1,8000")).toBeNull();
    expect(parseBudgetMinor("1 80")).toBeNull();
    expect(parseBudgetMinor("1,800.")).toBeNull();
    expect(parseBudgetMinor("1,800.123")).toBeNull();
    expect(parseBudgetMinor("1.234")).toBeNull();
    expect(parseBudgetMinor("$1,800")).toBeNull();
    expect(parseBudgetMinor("0,000")).toBeNull();
  });

  it("renders one plural form for one and another for many", () => {
    expect(plural(0, "unit")).toBe("0 units");
    expect(plural(1, "unit")).toBe("1 unit");
    expect(plural(2, "unit")).toBe("2 units");
    expect(plural(1, "materially different build")).toBe("1 materially different build");
    expect(plural(3, "materially different build")).toBe(
      "3 materially different builds",
    );
    expect(plural(2, "person", "people")).toBe("2 people");
  });

  it("formats weights as whole percentages", () => {
    expect(formatWeight(0.55)).toBe("55%");
    expect(formatWeight(0.15)).toBe("15%");
  });

  it("describes a bounded search and its limit", () => {
    expect(searchSummary(search)).toBe("64 combinations checked · limit 100000 nodes");
    expect(searchSummary({ ...search, limited: true })).toContain("not proven optimal");
  });

  it("names the binding constraint", () => {
    const base: NoSolution = {
      binding: "budget",
      headline: "",
      explanation: "",
      blockers: [],
      adjustable: [],
      minimumBuildMinor: null,
      budgetShortfallMinor: null,
    };
    expect(bindingLabel(base)).toBe("Budget constraint");
    expect(bindingLabel({ ...base, binding: "stock" })).toBe("Stock constraint");
    expect(bindingLabel({ ...base, binding: "search_limit" })).toBe("Search limit");
    expect(bindingLabel({ ...base, binding: "compatibility" })).toBe(
      "Compatibility constraint",
    );
  });

  it("changes the request signature when any input changes", () => {
    const base = requestSignature("snap", "1200.00", "gaming", ["A"], ["B"]);
    expect(requestSignature("snap", "1200.00", "gaming", ["A"], ["B"])).toBe(base);
    expect(requestSignature("snap", "1201.00", "gaming", ["A"], ["B"])).not.toBe(base);
    expect(requestSignature("snap", "1200.00", "office", ["A"], ["B"])).not.toBe(base);
    expect(requestSignature("snap", "1200.00", "gaming", [], ["B"])).not.toBe(base);
    expect(requestSignature("snap", "1200.00", "gaming", ["A"], [])).not.toBe(base);
  });
});

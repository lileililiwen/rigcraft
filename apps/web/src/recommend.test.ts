import { describe, expect, it } from "vitest";
import type { NoSolution, SearchStats } from "./types";
import {
  bindingLabel,
  formatWeight,
  parseBudgetMinor,
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

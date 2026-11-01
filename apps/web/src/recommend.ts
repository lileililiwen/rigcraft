import type {
  Candidate,
  CandidatePart,
  NoSolution,
  RecommendRequest,
  SearchStats,
} from "./types";
import { plural } from "./plural";

export { plural };

export const USE_CASES = [
  { value: "gaming", label: "Gaming" },
  { value: "office", label: "Office and general use" },
  { value: "creation", label: "Creation and heavy workloads" },
];

/**
 * Parse a human budget into integer minor units (1200 → 120000, 1299.50 →
 * 129950). A comma or a single space groups thousands only in an unambiguous
 * layout: one to three digits, then groups of exactly three. `1.800,50` is a
 * decimal-comma layout this app does not accept, and guessing it would move the
 * budget by three orders of magnitude, so it is refused.
 */
export function parseBudgetMinor(value: string): number | null {
  const trimmed = value.trim();
  const match = /^(\d{1,3}(?:[, ]\d{3})+|\d+)(?:\.(\d{1,2}))?$/.exec(trimmed);
  if (!match) {
    return null;
  }
  const whole = Number(match[1].replace(/[, ]/g, ""));
  const minor = whole * 100 + Number((match[2] ?? "").padEnd(2, "0"));
  if (!Number.isSafeInteger(minor) || minor <= 0) {
    return null;
  }
  return minor;
}

export function formatScore(value: number | null): string {
  return value === null ? "Not scored" : value.toFixed(2);
}

export function formatWeight(value: number): string {
  return `${Math.round(value * 100)}%`;
}

export function searchSummary(search: SearchStats): string {
  const base = `${plural(search.leavesEvaluated, "combination")} checked · limit ${search.limit} nodes`;
  return search.limited
    ? `${base} · limit reached, results are best effort and not proven optimal`
    : base;
}

export function bindingLabel(noSolution: NoSolution): string {
  switch (noSolution.binding) {
    case "budget":
      return "Budget constraint";
    case "stock":
      return "Stock constraint";
    case "search_limit":
      return "Search limit";
    default:
      return "Compatibility constraint";
  }
}

export function candidatePart(
  candidate: Candidate,
  category: string,
): CandidatePart | undefined {
  return candidate.parts.find((part) => part.category === category);
}

/** The request a generated report is bound to, used to detect stale form edits. */
export function requestSignature(
  snapshotId: string,
  budget: string,
  useCase: string,
  locked: string[],
  excluded: string[],
): string {
  return [
    snapshotId,
    budget.trim(),
    useCase,
    [...locked].sort().join(","),
    [...excluded].sort().join(","),
  ].join("|");
}

export function selectedRequest(
  snapshotId: string,
  budgetMinor: number,
  currency: string,
  useCase: string,
  locked: string[],
  excluded: string[],
): RecommendRequest {
  return {
    snapshotId,
    budgetMinor,
    currency,
    useCase,
    inventoryOnly: true,
    lockedSkus: [...locked].sort(),
    excludedSkus: [...excluded].sort(),
  };
}

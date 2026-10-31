import { describe, expect, it } from "vitest";
import {
  fieldIsEdited,
  minorToBudgetText,
  proposalToForm,
  recordsBySku,
  sameSkus,
  skusOf,
  sourceLabel,
} from "./ai";
import type { AiProposalResponse, RecordItem, Snapshot } from "./types";

function record(id: string, sku: string): RecordItem {
  return {
    id,
    lot: 1,
    sku,
    category: "cpu",
    modelRaw: `Fixture ${sku}`,
    modelNormalized: `Fixture ${sku}`,
    quantity: 2,
    priceMinor: 19900,
    currency: "USD",
    specs: [],
    provenance: {},
    match: { basis: "lexical", confidence: "unverified" },
  };
}

const snapshot: Snapshot = {
  id: "snap-1",
  createdAt: "2026-10-31T12:00:00Z",
  revision: 1,
  parentSnapshotId: null,
  currency: "USD",
  source: { filename: "ai.csv", sha256: "a".repeat(64) },
  note: null,
  records: [record("r1", "FIX-CPU"), record("r2", "FIX-GPU")],
  totals: { recordCount: 2, unitCount: 4, valueMinor: 39800 },
};

const response: AiProposalResponse = {
  mode: "provider",
  promptVersion: "ai-parse-v1",
  provider: { kind: "mock", baseUrl: "https://provider.example/v1", model: "mock" },
  proposal: {
    budgetMinor: { value: 200000, source: "sentence", excerpt: "$2000" },
    currency: { value: "USD", source: "snapshot", note: "Taken from the selected snapshot." },
    useCase: { value: "gaming", source: "sentence", excerpt: "gaming" },
    inventoryOnly: { value: true, source: "default", note: "Always on." },
    lockedSkus: { value: ["FIX-CPU"], source: "sentence", excerpt: "FIX-CPU" },
    excludedSkus: { value: [], source: "default" },
  },
  ambiguities: [],
  ignoredFields: [],
  sent: {
    sentence: true,
    currency: true,
    useCases: ["gaming", "office", "creation"],
    inventoryFields: [],
    inventoryRecords: 0,
  },
  confirmRequired: true,
};

describe("ai helpers", () => {
  it("formats minor units as the budget field expects", () => {
    expect(minorToBudgetText(200000)).toBe("2000.00");
    expect(minorToBudgetText(129950)).toBe("1299.50");
    expect(minorToBudgetText(5)).toBe("0.05");
    expect(minorToBudgetText(null)).toBe("");
  });

  it("names every source in text, not only in colour", () => {
    expect(sourceLabel(response.proposal.budgetMinor)).toBe("✎ From your sentence: “$2000”");
    expect(sourceLabel(response.proposal.currency)).toBe(
      "▣ From this snapshot — Taken from the selected snapshot.",
    );
    expect(sourceLabel(response.proposal.inventoryOnly)).toContain("≡ Default");
    expect(
      sourceLabel({ value: null, source: "unresolved", note: "No budget was stated." }),
    ).toContain("! Needs confirmation");
  });

  it("detects an edit against the seeded value only", () => {
    expect(fieldIsEdited("2000.00", "2000.00")).toBe(false);
    expect(fieldIsEdited("2000", "2000.00")).toBe(true);
    expect(fieldIsEdited(" 2000.00 ", "2000.00")).toBe(false);
  });

  it("seeds the form and maps SKUs back to record ids", () => {
    const seeded = proposalToForm(response);
    expect(seeded.budgetText).toBe("2000.00");
    expect(seeded.useCase).toBe("gaming");
    expect(seeded.lockedSkus).toEqual(["FIX-CPU"]);
    const byId = recordsBySku(snapshot, seeded.lockedSkus);
    expect(byId).toEqual({ r1: true, r2: false });
    expect(skusOf(snapshot, byId)).toEqual(["FIX-CPU"]);
    expect(sameSkus(["a", "b"], ["b", "a"])).toBe(true);
    expect(sameSkus(["a"], ["a", "b"])).toBe(false);
  });
});

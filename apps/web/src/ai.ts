import type {
  AiFieldSource,
  AiProposalResponse,
  ProposedField,
  RecordItem,
  Snapshot,
} from "./types";

/** The symbol and text shown for each source. Colour is never the only cue. */
export const SOURCE_META: Record<AiFieldSource, { symbol: string; label: string }> = {
  sentence: { symbol: "✎", label: "From your sentence" },
  snapshot: { symbol: "▣", label: "From this snapshot" },
  default: { symbol: "≡", label: "Default" },
  unresolved: { symbol: "!", label: "Needs confirmation" },
};

export interface FormValues {
  budgetText: string;
  useCase: string;
  lockedSkus: string[];
  excludedSkus: string[];
}

/** Minor units to the text the budget field expects. */
export function minorToBudgetText(minor: number | null | undefined): string {
  if (minor === null || minor === undefined) {
    return "";
  }
  return `${Math.floor(minor / 100)}.${String(minor % 100).padStart(2, "0")}`;
}

/** One readable sentence describing where a proposed value came from. */
export function sourceLabel(field: ProposedField<unknown>): string {
  const meta = SOURCE_META[field.source];
  switch (field.source) {
    case "sentence":
      return field.excerpt
        ? `${meta.symbol} ${meta.label}: “${field.excerpt}”`
        : `${meta.symbol} ${meta.label}`;
    case "snapshot":
    case "default":
      return field.note ? `${meta.symbol} ${meta.label} — ${field.note}` : `${meta.symbol} ${meta.label}`;
    default:
      return field.note ? `${meta.symbol} ${meta.label} — ${field.note}` : `${meta.symbol} ${meta.label}`;
  }
}

/** An edit is a change from the value the proposal was seeded with. */
export function fieldIsEdited(current: string, seeded: string): boolean {
  return current.trim() !== seeded.trim();
}

export function sameSkus(left: string[], right: string[]): boolean {
  return [...left].sort().join("|") === [...right].sort().join("|");
}

export function recordsBySku(snapshot: Snapshot, skus: string[]): Record<string, boolean> {
  const wanted = new Set(skus);
  const result: Record<string, boolean> = {};
  for (const record of snapshot.records) {
    result[record.id] = wanted.has(record.sku);
  }
  return result;
}

export function skusOf(snapshot: Snapshot, byId: Record<string, boolean>): string[] {
  return snapshot.records.filter((record) => byId[record.id]).map((record) => record.sku);
}

/** Seed the editable preview from a proposal. */
export function proposalToForm(response: AiProposalResponse): FormValues {
  return {
    budgetText: minorToBudgetText(response.proposal.budgetMinor.value),
    useCase: response.proposal.useCase.value ?? "",
    lockedSkus: response.proposal.lockedSkus.value ?? [],
    excludedSkus: response.proposal.excludedSkus.value ?? [],
  };
}

export function recordLabel(record: RecordItem): string {
  return `${record.modelRaw} (${record.sku})`;
}

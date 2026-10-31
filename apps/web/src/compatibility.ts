import type {
  CheckReport,
  RecordItem,
  SelectionItem,
  Snapshot,
  Verdict,
} from "./types";

export interface CategorySpec {
  key: string;
  label: string;
  multi: boolean;
}

export const PART_CATEGORIES: CategorySpec[] = [
  { key: "cpu", label: "CPU", multi: false },
  { key: "motherboard", label: "Motherboard", multi: false },
  { key: "ram", label: "Memory", multi: true },
  { key: "gpu", label: "Graphics card", multi: false },
  { key: "psu", label: "Power supply", multi: false },
  { key: "case", label: "Case", multi: false },
  { key: "cooler", label: "Cooler", multi: false },
  { key: "storage", label: "Storage", multi: true },
];

export interface SelectionState {
  single: Record<string, string>;
  multi: Record<string, boolean>;
  quantities: Record<string, string>;
}

export function emptySelection(): SelectionState {
  return { single: {}, multi: {}, quantities: {} };
}

export function categoryLabel(category: string): string {
  return PART_CATEGORIES.find((entry) => entry.key === category)?.label ?? category;
}

export function isMultiCategory(category: string): boolean {
  return PART_CATEGORIES.find((entry) => entry.key === category)?.multi ?? false;
}

export function defaultQuantity(): string {
  return "1";
}

export function parseQuantity(value: string): number | null {
  const trimmed = value.trim();
  if (!/^\d+$/.test(trimmed)) {
    return null;
  }
  const quantity = Number(trimmed);
  return Number.isSafeInteger(quantity) ? quantity : null;
}

export function isRecordSelected(
  state: SelectionState,
  record: RecordItem,
): boolean {
  if (isMultiCategory(record.category)) {
    return state.multi[record.id] === true;
  }
  return state.single[record.category] === record.id;
}

export interface SelectedRecord {
  record: RecordItem;
  quantity: number;
}

export function selectedRecords(
  snapshot: Snapshot,
  state: SelectionState,
): SelectedRecord[] {
  const selected: SelectedRecord[] = [];
  for (const record of snapshot.records) {
    if (!isRecordSelected(state, record)) {
      continue;
    }
    const quantity = parseQuantity(state.quantities[record.id] ?? "1");
    selected.push({ record, quantity: quantity ?? 0 });
  }
  return selected;
}

export function selectionErrors(
  snapshot: Snapshot | null,
  state: SelectionState,
): string[] {
  if (!snapshot) {
    return ["Choose an inventory snapshot before checking."];
  }
  const selected = selectedRecords(snapshot, state);
  if (selected.length === 0) {
    return ["Select at least one part from the snapshot."];
  }
  const errors: string[] = [];
  const perSku = new Map<string, { quantity: number; available: number }>();
  for (const { record, quantity } of selected) {
    if (quantity < 1) {
      errors.push(`Quantity for ${record.sku} must be a whole number of one or more.`);
      continue;
    }
    const entry = perSku.get(record.sku) ?? { quantity: 0, available: 0 };
    entry.quantity += quantity;
    entry.available += record.quantity;
    perSku.set(record.sku, entry);
  }
  for (const [sku, entry] of perSku) {
    if (entry.quantity > entry.available) {
      errors.push(
        `${sku} needs ${entry.quantity} unit(s) but this snapshot has ${entry.available}.`,
      );
    }
  }
  return errors;
}

export function buildSelection(
  snapshot: Snapshot,
  state: SelectionState,
): SelectionItem[] {
  const bySku = new Map<string, number>();
  for (const { record, quantity } of selectedRecords(snapshot, state)) {
    bySku.set(record.sku, (bySku.get(record.sku) ?? 0) + quantity);
  }
  return [...bySku.entries()]
    .map(([sku, quantity]) => ({ sku, quantity }))
    .sort((a, b) => a.sku.localeCompare(b.sku));
}

export function selectionSignature(
  items: Array<{ sku: string; quantity: number }>,
): string {
  return items
    .map((item) => `${item.sku}:${item.quantity}`)
    .sort()
    .join("|");
}

export function reportIsStale(
  report: CheckReport | null,
  snapshotId: string,
  current: SelectionItem[],
): boolean {
  if (!report) {
    return false;
  }
  return (
    report.snapshotId !== snapshotId ||
    selectionSignature(report.selection) !== selectionSignature(current)
  );
}

export interface VerdictMeta {
  symbol: string;
  label: string;
  className: string;
}

export const VERDICT_META: Record<Verdict, VerdictMeta> = {
  compatible: { symbol: "✓", label: "Compatible", className: "compatible" },
  incompatible: { symbol: "✕", label: "Incompatible", className: "incompatible" },
  unknown: { symbol: "?", label: "Unknown", className: "unknown" },
};

export function verdictMeta(status: Verdict): VerdictMeta {
  return VERDICT_META[status];
}

export function reportSummary(report: CheckReport): string {
  const parts = [
    `${report.summary.compatible} compatible`,
    `${report.summary.incompatible} conflict${report.summary.incompatible === 1 ? "" : "s"}`,
    `${report.summary.unknown} unknown`,
  ];
  if (report.completeness.status === "incomplete") {
    parts.push("build incomplete");
  }
  return parts.join(" · ");
}

export function completionLabel(report: CheckReport): string {
  return report.completeness.status === "complete"
    ? "Complete build"
    : `Incomplete build · ${report.completeness.missing.length} missing`;
}

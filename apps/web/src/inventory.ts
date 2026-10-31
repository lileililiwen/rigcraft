import type {
  DuplicateAction,
  FieldError,
  Preview,
  PreviewRow,
  RowOverrides,
  RowValues,
} from "./types";

export type DecisionMap = Record<number, "accept" | "exclude">;
export type ResolutionMap = Record<string, DuplicateAction | undefined>;
export type OverrideMap = Record<number, RowOverrides | undefined>;

export const LOGICAL_FIELDS = [
  { key: "sku", label: "SKU" },
  { key: "category", label: "Category" },
  { key: "model", label: "Model" },
  { key: "quantity", label: "Quantity" },
  { key: "price", label: "Sale price" },
  { key: "currency", label: "Currency" },
  { key: "specs", label: "Specifications" },
  { key: "cost", label: "Purchase cost (not used in P0)" },
] as const;

export const REQUIRED_FIELDS = ["sku", "category", "model", "quantity", "price"];

export const CATEGORIES = [
  "cpu",
  "motherboard",
  "ram",
  "gpu",
  "psu",
  "case",
  "cooler",
  "storage",
];

export function defaultDecisions(preview: Preview): DecisionMap {
  const decisions: DecisionMap = {};
  for (const row of preview.rows) {
    decisions[row.row] = row.status === "error" ? "exclude" : "accept";
  }
  return decisions;
}

export function defaultOverrides(): OverrideMap {
  return {};
}

export function defaultResolutions(): ResolutionMap {
  return {};
}

export function acceptedRows(preview: Preview, decisions: DecisionMap): PreviewRow[] {
  return preview.rows.filter((row) => decisions[row.row] === "accept");
}

export function rowsNeedingResolution(
  preview: Preview,
  decisions: DecisionMap,
  resolutions: ResolutionMap,
): string[] {
  return preview.duplicateGroups
    .filter((group) => {
      const accepted = group.rows.filter((row) => decisions[row] === "accept");
      return accepted.length > 1 && !resolutions[group.groupId];
    })
    .map((group) => group.groupId);
}

function validateOverride(override: RowOverrides | undefined): FieldError[] {
  if (!override) {
    return [];
  }
  const errors: FieldError[] = [];
  if (override.sku !== undefined && override.sku.trim() === "") {
    errors.push({ field: "sku", code: "required", message: "SKU is required." });
  }
  if (override.category !== undefined && override.category.trim() === "") {
    errors.push({
      field: "category",
      code: "required",
      message: "Category is required.",
    });
  }
  if (override.model !== undefined && override.model.trim() === "") {
    errors.push({ field: "model", code: "required", message: "Model is required." });
  }
  if (override.quantity !== undefined) {
    if (override.quantity.trim() === "") {
      errors.push({
        field: "quantity",
        code: "required",
        message: "Quantity is required.",
      });
    } else if (!/^\d+$/.test(override.quantity.trim())) {
      errors.push({
        field: "quantity",
        code: "quantity_format",
        message: "Quantity must be a whole number.",
        suggestion: "Enter a whole number such as 3.",
      });
    }
  }
  if (override.price !== undefined) {
    if (override.price.trim() === "") {
      errors.push({ field: "price", code: "required", message: "Sale price is required." });
    } else if (!/^\d+(\.\d{1,2})?$/.test(override.price.trim())) {
      errors.push({
        field: "price",
        code: "price_format",
        message: "Price must be a number with at most two decimals.",
        suggestion: "Write 1299 or 1299.50 without symbols or separators.",
      });
    }
  }
  return errors;
}

export function rowIssues(
  row: PreviewRow,
  override: RowOverrides | undefined,
  serverIssues: FieldError[] = [],
): FieldError[] {
  const overridden = new Set(Object.keys(override ?? {}));
  const remaining = row.errors.filter((error) => !overridden.has(error.field));
  const seen = new Set<string>();
  return [...remaining, ...serverIssues, ...validateOverride(override)].filter((error) => {
    const key = `${error.field}:${error.code}`;
    if (seen.has(key)) {
      return false;
    }
    seen.add(key);
    return true;
  });
}

export function overrideValues(row: PreviewRow, override: RowOverrides | undefined): RowValues {
  return {
    sku: override?.sku ?? row.values.sku,
    category: override?.category ?? row.values.category,
    model: override?.model ?? row.values.model,
    quantity: override?.quantity ?? row.values.quantity,
    price: override?.price ?? row.values.price,
    currency: row.values.currency,
    specs: override?.specs ?? row.values.specs,
  };
}

export function canCommit(
  preview: Preview,
  decisions: DecisionMap,
  resolutions: ResolutionMap,
  overrides: OverrideMap,
  serverIssues: Record<number, FieldError[]>,
): { ready: boolean; reason: string } {
  const accepted = acceptedRows(preview, decisions);
  if (accepted.length === 0) {
    return { ready: false, reason: "Include at least one valid row to create a snapshot." };
  }
  const blocked = accepted.find(
    (row) => rowIssues(row, overrides[row.row], serverIssues[row.row]).length > 0,
  );
  if (blocked) {
    return {
      ready: false,
      reason: `Row ${blocked.row} still has an error. Correct it or choose Exclude.`,
    };
  }
  const unresolved = rowsNeedingResolution(preview, decisions, resolutions);
  if (unresolved.length > 0) {
    return {
      ready: false,
      reason: "Choose merge, replace, or keep separate for every duplicated SKU.",
    };
  }
  return { ready: true, reason: "" };
}

export function priceToMinor(value: string): number | null {
  const trimmed = value.trim();
  if (!/^\d+(\.\d{1,2})?$/.test(trimmed)) {
    return null;
  }
  const [whole, fraction = ""] = trimmed.split(".");
  const minor = Number(whole) * 100 + Number(fraction.padEnd(2, "0"));
  return Number.isSafeInteger(minor) ? minor : null;
}

export function formatMoney(minor: number, currency: string): string {
  try {
    return new Intl.NumberFormat(undefined, {
      style: "currency",
      currency,
    }).format(minor / 100);
  } catch {
    return `${(minor / 100).toFixed(2)} ${currency}`;
  }
}

/**
 * One timestamp format for the whole UI: ISO 8601 in UTC. It never consults
 * `navigator.language`, so a zh-locale browser and an en-locale browser render
 * the same string.
 */
export function formatTimestamp(iso: string): string {
  const value = new Date(iso);
  if (Number.isNaN(value.getTime())) {
    return iso;
  }
  return value.toISOString().replace(/\.000Z$/, "Z");
}

export function shortId(id: string): string {
  return id.length > 12 ? `${id.slice(0, 8)}…${id.slice(-4)}` : id;
}

export function sourceLabel(source: string): string {
  if (source.startsWith("catalog:")) {
    return `Catalog · ${source.slice("catalog:".length)}`;
  }
  switch (source) {
    case "user":
      return "User-provided";
    case "rigcraft-normalizer":
      return "RigCraft normalization";
    case "licensed-source":
      return "Licensed source";
    default:
      return source;
  }
}

/**
 * The one-line distinction the inventory page needs: were this record's part
 * facts supplied by a catalog, or asserted by the operator? Stock values are
 * always operator facts.
 */
export function factsLabel(source: string | undefined): string {
  if (!source || source === "user") {
    return "Operator-entered facts";
  }
  if (source.startsWith("catalog:")) {
    return `Catalog facts · ${source.slice("catalog:".length)}`;
  }
  return `Facts · ${source}`;
}

export function continuationSummary(rows: PreviewRow[]): string {
  const total = rows.length;
  const errors = rows.filter((row) => row.status === "error").length;
  const duplicates = rows.filter((row) => row.status === "duplicate").length;
  return `${total} row${total === 1 ? "" : "s"} · ${errors} error${
    errors === 1 ? "" : "s"
  } · ${duplicates} duplicate${duplicates === 1 ? "" : "s"}`;
}

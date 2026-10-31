export type RowStatus = "valid" | "error" | "duplicate";
export type DecisionAction = "accept" | "exclude";
export type DuplicateAction = "merge" | "replace" | "keep_separate";

export interface FieldError {
  field: string;
  code: string;
  message: string;
  suggestion?: string;
}

export interface Provenance {
  source: string;
  reference: string;
  observedAt: string;
}

export interface Spec {
  key: string;
  raw: string;
  number?: number;
  text?: string;
  unit?: string;
  status: "confirmed" | "unknown";
  provenance: Provenance;
}

export interface MatchInfo {
  basis: string;
  confidence: string;
}

export interface RowValues {
  sku: string;
  category: string;
  model: string;
  quantity: string;
  price: string;
  currency: string;
  specs: string;
}

export interface NormalizedRow {
  sku: string | null;
  category: string | null;
  modelNormalized: string | null;
  quantity: number | null;
  priceMinor: number | null;
  currency: string | null;
}

export interface PreviewRow {
  row: number;
  status: RowStatus;
  values: RowValues;
  normalized: NormalizedRow;
  specs: Spec[];
  errors: FieldError[];
  duplicateGroupId: string | null;
  provenance: Record<string, Provenance>;
  match: MatchInfo;
}

export interface DuplicateGroup {
  groupId: string;
  sku: string;
  rows: number[];
}

export interface PreviewSummary {
  total: number;
  valid: number;
  errors: number;
  duplicateRows: number;
}

export interface Preview {
  previewId: string;
  createdAt: string;
  expiresAt: string;
  currency: string;
  filename: string;
  columns: string[];
  mapping: Record<string, string>;
  rows: PreviewRow[];
  duplicateGroups: DuplicateGroup[];
  summary: PreviewSummary;
}

export interface RowOverrides {
  sku?: string;
  category?: string;
  model?: string;
  quantity?: string;
  price?: string;
  specs?: string;
}

export interface RowDecision {
  row: number;
  action: DecisionAction;
  overrides?: RowOverrides;
}

export interface DuplicateResolution {
  groupId: string;
  action: DuplicateAction;
}

export interface CommitRequest {
  decisions: RowDecision[];
  duplicateResolutions: DuplicateResolution[];
}

export interface RecordItem {
  id: string;
  lot: number;
  sku: string;
  category: string;
  modelRaw: string;
  modelNormalized: string;
  quantity: number;
  priceMinor: number;
  currency: string;
  specs: Spec[];
  provenance: Record<string, Provenance>;
  match: MatchInfo;
}

export interface SnapshotTotals {
  recordCount: number;
  unitCount: number;
  valueMinor: number;
}

export interface Snapshot {
  id: string;
  createdAt: string;
  revision: number;
  parentSnapshotId: string | null;
  currency: string;
  source: { filename: string; sha256: string };
  note: string | null;
  records: RecordItem[];
  totals: SnapshotTotals;
}

export interface CommitResponse {
  snapshotId: string;
  snapshot: Snapshot;
}

export interface SnapshotSummary {
  id: string;
  createdAt: string;
  revision: number;
  parentSnapshotId: string | null;
  currency: string;
  sourceFilename: string;
  recordCount: number;
  unitCount: number;
  valueMinor: number;
  note: string | null;
}

export interface RecordChange {
  recordId: string;
  quantity?: number;
  priceMinor?: number;
  sku?: string;
  category?: string;
  model?: string;
  specs?: string;
}

export interface CorrectionRequest {
  note?: string;
  changes: RecordChange[];
}

export interface ApiErrorBody {
  code: string;
  message: string;
  field?: string;
  row?: number;
  details?: {
    rows?: Array<{ row: number; errors: FieldError[] }>;
    groups?: string[];
    sku?: string;
  };
}

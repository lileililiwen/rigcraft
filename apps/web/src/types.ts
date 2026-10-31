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

export type Verdict = "compatible" | "incompatible" | "unknown";
export type CompletenessStatus = "complete" | "incomplete";

export interface SelectionItem {
  sku: string;
  quantity: number;
}

export interface CheckRequest {
  snapshotId: string;
  selection: SelectionItem[];
}

export interface PartRef {
  recordId: string;
  sku: string;
  category: string;
  model: string;
}

export interface RuleEvidence {
  part: PartRef;
  field: string;
  value: string | null;
  provenance: Provenance | null;
}

export interface RuleResult {
  ruleId: string;
  ruleVersion: string;
  title: string;
  category: string;
  status: Verdict;
  parts: PartRef[];
  evidence: RuleEvidence[];
  explanation: string;
  followUp: string | null;
  method: string;
}

export interface MissingPart {
  category: string;
  reason: string;
  followUp: string;
}

export interface Completeness {
  status: CompletenessStatus;
  missing: MissingPart[];
}

export interface SelectedPart {
  recordId: string;
  sku: string;
  category: string;
  model: string;
  quantity: number;
  lots: number;
  available: number;
}

export interface CheckSummary {
  compatible: number;
  incompatible: number;
  unknown: number;
}

export interface CheckReport {
  id: string;
  snapshotId: string;
  snapshotRevision: number;
  ruleSetVersion: string;
  checkedAt: string;
  selection: SelectedPart[];
  completeness: Completeness;
  overallStatus: Verdict;
  rules: RuleResult[];
  summary: CheckSummary;
  stale: boolean;
  currentSnapshotRevision?: number | null;
  staleReason?: string | null;
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
    category?: string;
    skus?: string[];
    requested?: number;
    available?: number;
    lots?: number;
  };
}

export type RecommendStatus = "ok" | "no_solution" | "search_limited";

export interface RecommendRequest {
  snapshotId: string;
  budgetMinor: number;
  currency: string;
  useCase: string;
  inventoryOnly: boolean;
  lockedSkus: string[];
  excludedSkus: string[];
}

export interface RecommendationRequestEcho {
  budgetMinor: number;
  currency: string;
  useCase: string;
  inventoryOnly: boolean;
  lockedSkus: string[];
  excludedSkus: string[];
}

export interface CandidatePart {
  recordId: string;
  sku: string;
  category: string;
  model: string;
  quantity: number;
  owned: boolean;
  unitPriceMinor: number;
  lineTotalMinor: number;
  available: number;
}

export interface StockLine {
  sku: string;
  category: string;
  used: number;
  available: number;
  remaining: number;
  owned: boolean;
}

export interface ScoreDimension {
  key: string;
  label: string;
  available: boolean;
  value: number | null;
  weight: number;
  basis: string;
}

export interface ScoreBreakdown {
  total: number;
  confidence: string;
  dimensions: ScoreDimension[];
}

export interface AlternativePart {
  category: string;
  sku: string;
  model: string;
  lineTotalMinor: number;
  reason: string;
  detail: string;
}

export interface Candidate {
  rank: number;
  objective: string;
  label: string;
  parts: CandidatePart[];
  totalMinor: number;
  budgetDeltaMinor: number;
  stock: StockLine[];
  unitsFromStock: number;
  ownedUnits: number;
  overallStatus: Verdict;
  completeness: CompletenessStatus;
  checkReportId: string;
  checkSummary: CheckSummary;
  unknownFollowUps: string[];
  scores: ScoreBreakdown;
  reasons: string[];
  alternatives: AlternativePart[];
  check: CheckReport;
}

export interface Exclusion {
  sku: string;
  category: string;
  model: string;
  reason: string;
  detail: string;
}

export interface Blocker {
  code: string;
  category: string | null;
  sku: string | null;
  detail: string;
}

export interface NoSolution {
  binding: string;
  headline: string;
  explanation: string;
  blockers: Blocker[];
  adjustable: string[];
  minimumBuildMinor: number | null;
  budgetShortfallMinor: number | null;
}

export interface SearchStats {
  limit: number;
  nodesExamined: number;
  leavesEvaluated: number;
  validBuilds: number;
  limited: boolean;
  method: string;
}

export interface RecommendationReport {
  id: string;
  createdAt: string;
  snapshotId: string;
  snapshotRevision: number;
  ruleSetVersion: string;
  scoringVersion: string;
  status: RecommendStatus;
  request: RecommendationRequestEcho;
  currency: string;
  budgetMinor: number;
  candidates: Candidate[];
  noSolution: NoSolution | null;
  search: SearchStats;
  exclusions: Exclusion[];
  stale: boolean;
  currentSnapshotRevision?: number | null;
  staleReason?: string | null;
}

export type AiFieldSource = "sentence" | "snapshot" | "default" | "unresolved";

export interface ProposedField<T> {
  value: T | null;
  source: AiFieldSource;
  excerpt?: string;
  note?: string;
}

export interface AiProposal {
  budgetMinor: ProposedField<number>;
  currency: ProposedField<string>;
  useCase: ProposedField<string>;
  inventoryOnly: ProposedField<boolean>;
  lockedSkus: ProposedField<string[]>;
  excludedSkus: ProposedField<string[]>;
}

export interface AiAmbiguity {
  field: string;
  code: string;
  message: string;
}

export interface AiPromptScope {
  sentence: boolean;
  currency: boolean;
  useCases: string[];
  inventoryFields: string[];
  inventoryRecords: number;
}

export interface AiProviderIdentity {
  kind: string;
  baseUrl: string;
  model: string;
}

export interface AiFieldChange {
  field: string;
  from: unknown;
  to: unknown;
}

export interface AiProposalResponse {
  mode: string;
  promptVersion: string;
  provider: AiProviderIdentity;
  proposal: AiProposal;
  ambiguities: AiAmbiguity[];
  ignoredFields: string[];
  sent: AiPromptScope;
  confirmRequired: boolean;
  request?: RecommendRequest | null;
  diff?: AiFieldChange[] | null;
}

export interface AiParseRequest {
  text: string;
  snapshotId: string;
  apiKey: string;
  baseUrl: string;
  model: string;
  includeInventorySummary: boolean;
}

export interface AiCurrentRequest {
  budgetMinor: number | null;
  currency: string | null;
  useCase: string | null;
  lockedSkus: string[];
  excludedSkus: string[];
}

export interface AiReviseRequest extends AiParseRequest {
  current: AiCurrentRequest;
}

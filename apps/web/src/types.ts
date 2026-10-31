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
  source?: string;
  sourceUrl?: string;
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
  catalog?: { partId: string };
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
    rows?: DetailRow[];
    specKeys?: string[];
    groups?: string[];
    sku?: string;
    category?: string;
    skus?: string[];
    requested?: number;
    available?: number;
    lots?: number;
    limit?: number;
    limitBytes?: number;
    specCode?: string;
    snapshotCurrency?: string;
    requestCurrency?: string;
    preRestorePath?: string;
    reason?: string;
    schemaVersion?: number;
    supportedSchemaVersion?: number;
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
  materialSignature: string;
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

export interface UseCaseRule {
  useCase: string;
  requiresDiscreteGpu: boolean;
  acceptsIgpu: boolean;
  explanation: string;
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
  useCaseRule: UseCaseRule;
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

export type MatchStatus = "matched" | "ambiguous" | "unmatched" | "invalid";

export interface LineIssue {
  code: string;
  message: string;
}

export interface MatchCandidate {
  sku: string;
  category: string;
  model: string;
  priceMinor: number;
  currency: string;
  available: number;
  coverage: number;
  exact: boolean;
  sharedTokens: string[];
  extraTokens: string[];
  missingTokens: string[];
  provenance: Provenance | null;
}

export interface ParsedLine {
  line: number;
  raw: string;
  status: MatchStatus;
  category: string | null;
  model: string;
  quantity: number;
  priceMinor: number | null;
  issues: LineIssue[];
  candidates: MatchCandidate[];
  disambiguation?: string | null;
  duplicateOf?: number | null;
}

export interface ParsedList {
  matchVersion: string;
  snapshotId: string;
  currency: string;
  lineCount: number;
  lines: ParsedLine[];
  limits: { maxLines: number; maxBytes: number; maxCandidates: number };
}

export interface ParseListRequest {
  snapshotId: string;
  text: string;
  currency: string;
}

export type PartSource = "owned" | "snapshot";

export interface BuildPartInput {
  source: PartSource;
  sku?: string;
  category?: string;
  model?: string;
  quantity?: number;
  specs?: string;
  priceMinor?: number;
}

export interface DiagnoseRequest {
  snapshotId: string;
  currency: string;
  build: BuildPartInput[];
}

export type AdviceAction = "must_replace" | "should_upgrade" | "keep" | "unverified";

export interface AdviceCost {
  currency: string;
  keepMinor: number;
  replacementSku: string | null;
  replacementModel: string | null;
  replacementMinor: number | null;
  additionalMinor: number | null;
  note: string;
}

export interface Advice {
  row: number;
  sku: string;
  category: string;
  model: string;
  action: AdviceAction;
  conditional: boolean;
  reason: string;
  ruleIds: string[];
  evidence: RuleEvidence[];
  verification?: string | null;
  cost: AdviceCost;
}

export interface AdviceGroups {
  mustReplace: number[];
  shouldUpgrade: number[];
  keep: number[];
  unverified: number[];
}

export interface PlanOption {
  row: number;
  category: string;
  sku: string;
  model: string;
  priceMinor: number;
  status: Verdict;
  resolves: string[];
  ruleIds: string[];
  followUps: string[];
}

export interface RejectedOption {
  row: number;
  category: string;
  sku: string;
  model: string;
  priceMinor: number;
  ruleIds: string[];
  detail: string;
}

export interface PlanStep {
  row: number;
  category: string;
  fromSku: string;
  fromModel: string;
  toSku: string;
  toModel: string;
  priceMinor: number;
  ruleIds: string[];
  reason: string;
}

export type PlanStatus =
  | "not_needed"
  | "validated"
  | "needs_evidence"
  | "no_option"
  | "no_valid_plan";

export interface UpgradePlan {
  status: PlanStatus;
  summary: string;
  steps: PlanStep[];
  options: PlanOption[];
  rejected: RejectedOption[];
  additionalCostMinor: number;
  currency: string;
  validation?: CheckReport | null;
  followUps: string[];
  evaluated: number;
  method: string;
}

export interface DescribedPart {
  row: number;
  source: "description" | "snapshot";
  sku: string;
  category: string;
  model: string;
  quantity: number;
  owned: boolean;
  reportedPriceMinor: number | null;
  specs: Spec[];
  provenance: Record<string, Provenance>;
  match: MatchInfo;
}

export interface DiagnosisReport {
  id: string;
  createdAt: string;
  snapshotId: string;
  snapshotRevision: number;
  ruleSetVersion: string;
  matchVersion: string;
  adviceVersion: string;
  currency: string;
  build: DescribedPart[];
  check: CheckReport;
  advice: Advice[];
  groups: AdviceGroups;
  plan: UpgradePlan;
  stale: boolean;
  currentSnapshotRevision?: number | null;
  staleReason?: string | null;
}

export interface DatabaseCounts {
  schemaVersion: number;
  snapshotCount: number;
  latestRevision: number | null;
  createdAt: string | null;
  recordCount: number;
  lotCount: number;
}

export interface BackupInspection {
  fileName: string;
  sizeBytes: number;
  integrity: string;
  counts: DatabaseCounts;
}

export interface RestoreSummary {
  restored: boolean;
  sourceSchemaVersion: number;
  preRestorePath: string;
  counts: DatabaseCounts;
}

export interface DownloadedFile {
  filename: string;
  note: string | null;
}

export type DetailRow =
  | { row: number; errors: FieldError[] }
  | { row: number; field: string; code: string; message: string; suggestion?: string };

export interface CatalogDataset {
  id: string;
  name: string;
  sourceUrl: string | null;
  commit: string | null;
  license: string;
  retrievedAt: string | null;
  importedAt: string;
  version: string;
  rowCount: number;
  categoryCounts: Record<string, number>;
  active: boolean;
}

export interface CatalogPart {
  id: string;
  datasetId: string;
  externalId: string | null;
  sku: string;
  category: string;
  model: string;
  modelNormalized: string;
  manufacturer: string | null;
  specs: Spec[];
  sourceUrl: string | null;
  verifiedAt: string | null;
}

export interface StoredCatalogPart {
  part: CatalogPart;
  dataset: CatalogDataset;
}

export interface CatalogPartDetail extends StoredCatalogPart {
  provenance: Record<string, Provenance>;
}

export interface CatalogReport {
  accepted: boolean;
  replaced: boolean;
  dataset: CatalogDataset;
  partCount: number;
  categoryCounts: Record<string, number>;
  specKeys: string[];
}

export interface CatalogSearchResult {
  query: string;
  category: string | null;
  limit: number;
  offset: number;
  total: number;
  hits: StoredCatalogPart[];
}

export interface CatalogProposals {
  model?: string;
  specs: Record<string, string>;
}

export interface CatalogConflict {
  field: string;
  operatorValue: string;
  catalogValue: string;
}

export interface CatalogCandidate {
  part: CatalogPart;
  dataset: CatalogDataset;
  provenance: Provenance;
}

export interface CatalogMatch {
  row: number;
  status: "matched" | "ambiguous" | "unmatched";
  basis: "sku" | "mpn" | "model" | "none";
  candidates: CatalogCandidate[];
  proposals: CatalogProposals;
  conflicts: CatalogConflict[];
}

export interface CatalogMatchRow {
  row: number;
  sku: string;
  category: string;
  model: string;
  specs: string;
}

export interface CatalogMatchResponse {
  matches: CatalogMatch[];
}

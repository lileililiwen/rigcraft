import type {
  AiParseRequest,
  AiProposalResponse,
  AiReviseRequest,
  ApiErrorBody,
  BackupInspection,
  CheckReport,
  CheckRequest,
  CommitRequest,
  CatalogDataset,
  CatalogMatchResponse,
  CatalogMatchRow,
  CatalogPartDetail,
  CatalogReport,
  CatalogSearchResult,
  CommitResponse,
  CorrectionRequest,
  DiagnoseRequest,
  DiagnosisReport,
  DownloadedFile,
  ParsedList,
  ParseListRequest,
  Preview,
  RecommendationReport,
  RecommendRequest,
  RestoreSummary,
  Snapshot,
  SnapshotSummary,
} from "./types";

export class ApiFailure extends Error {
  status: number;
  code: string;
  field?: string;
  row?: number;
  details?: ApiErrorBody["details"];

  constructor(status: number, body: ApiErrorBody) {
    super(body.message);
    this.name = "ApiFailure";
    this.status = status;
    this.code = body.code;
    this.field = body.field;
    this.row = body.row;
    this.details = body.details;
  }
}

async function readResponse<T>(response: Response): Promise<T> {
  const text = await response.text();
  let body: unknown = null;
  if (text) {
    try {
      body = JSON.parse(text);
    } catch {
      body = null;
    }
  }
  if (!response.ok) {
    const error = (body as { error?: ApiErrorBody } | null)?.error;
    throw new ApiFailure(
      response.status,
      error ?? {
        code: "network_error",
        message: `The service returned status ${response.status}.`,
      },
    );
  }
  return body as T;
}

export async function previewImport(
  file: File,
  currency: string,
  mapping: Record<string, string>,
): Promise<Preview> {
  const form = new FormData();
  form.append("file", file, file.name);
  form.append("currency", currency);
  if (Object.keys(mapping).length > 0) {
    form.append("mapping", JSON.stringify(mapping));
  }
  const response = await fetch("/api/imports", { method: "POST", body: form });
  return readResponse<Preview>(response);
}

export async function commitImport(
  previewId: string,
  request: CommitRequest,
): Promise<CommitResponse> {
  const response = await fetch(`/api/imports/${encodeURIComponent(previewId)}/commit`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(request),
  });
  return readResponse<CommitResponse>(response);
}

export async function getSnapshot(snapshotId: string): Promise<Snapshot> {
  const response = await fetch(
    `/api/inventory/snapshots/${encodeURIComponent(snapshotId)}`,
  );
  return readResponse<Snapshot>(response);
}

export async function listSnapshots(): Promise<SnapshotSummary[]> {
  const response = await fetch("/api/inventory/snapshots");
  return readResponse<SnapshotSummary[]>(response);
}

export async function correctSnapshot(
  snapshotId: string,
  request: CorrectionRequest,
): Promise<Snapshot> {
  const response = await fetch(
    `/api/inventory/snapshots/${encodeURIComponent(snapshotId)}/corrections`,
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(request),
    },
  );
  return readResponse<Snapshot>(response);
}

export async function checkBuild(request: CheckRequest): Promise<CheckReport> {
  const response = await fetch("/api/builds/check", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(request),
  });
  return readResponse<CheckReport>(response);
}

export async function getCheckReport(checkId: string): Promise<CheckReport> {
  const response = await fetch(`/api/builds/checks/${encodeURIComponent(checkId)}`);
  return readResponse<CheckReport>(response);
}

export const sampleCsvUrl = "/api/imports/sample";

function dispositionFilename(response: Response, fallback: string): string {
  const disposition = response.headers.get("Content-Disposition") ?? "";
  return /filename="([^"]+)"/.exec(disposition)?.[1] ?? fallback;
}

function saveBlob(blob: Blob, filename: string): void {
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  document.body.append(anchor);
  anchor.click();
  anchor.remove();
  URL.revokeObjectURL(url);
}

async function downloadFailure(response: Response): Promise<never> {
  const text = await response.text();
  let body: { error?: ApiErrorBody } | null = null;
  try {
    body = JSON.parse(text) as { error?: ApiErrorBody };
  } catch {
    body = null;
  }
  throw new ApiFailure(
    response.status,
    body?.error ?? {
      code: "download_failed",
      message: `The download returned status ${response.status}.`,
    },
  );
}

/** Download one snapshot as the importer's CSV or the authoritative JSON. */
export async function downloadSnapshotExport(
  snapshotId: string,
  format: "csv" | "json",
): Promise<DownloadedFile> {
  const response = await fetch(
    `/api/inventory/snapshots/${encodeURIComponent(snapshotId)}/export?format=${format}`,
  );
  if (!response.ok) {
    return downloadFailure(response);
  }
  const text = await response.text();
  const filename = dispositionFilename(
    response,
    `rigcraft-inventory-snapshot.${format}`,
  );
  saveBlob(
    new Blob([text], {
      type: format === "csv" ? "text/csv;charset=utf-8" : "application/json",
    }),
    filename,
  );
  return {
    filename,
    note: response.headers.get("X-Rigcraft-Export-Note"),
  };
}

/** Download the whole live database as a standalone SQLite file. */
export async function downloadBackup(): Promise<string> {
  const response = await fetch("/api/backup");
  if (!response.ok) {
    return downloadFailure(response);
  }
  const blob = await response.blob();
  const filename = dispositionFilename(response, "rigcraft-backup.sqlite3");
  saveBlob(blob, filename);
  return filename;
}

export async function inspectBackup(file: File): Promise<BackupInspection> {
  const form = new FormData();
  form.append("file", file, file.name);
  const response = await fetch("/api/backup/inspect", { method: "POST", body: form });
  return readResponse<BackupInspection>(response);
}

export async function restoreBackup(file: File): Promise<RestoreSummary> {
  const form = new FormData();
  form.append("file", file, file.name);
  form.append("confirm", "restore");
  const response = await fetch("/api/backup/restore", { method: "POST", body: form });
  return readResponse<RestoreSummary>(response);
}

export async function recommendBuild(
  request: RecommendRequest,
): Promise<RecommendationReport> {
  const response = await fetch("/api/recommendations", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(request),
  });
  return readResponse<RecommendationReport>(response);
}

export async function getRecommendation(id: string): Promise<RecommendationReport> {
  const response = await fetch(`/api/recommendations/${encodeURIComponent(id)}`);
  return readResponse<RecommendationReport>(response);
}

/** Fetch an export and hand it to the browser as a download. Returns the file name. */
export async function downloadRecommendationExport(
  id: string,
  format: "json" | "markdown",
): Promise<string> {
  const response = await fetch(
    `/api/recommendations/${encodeURIComponent(id)}/export?format=${format}`,
  );
  const text = await response.text();
  if (!response.ok) {
    let body: { error?: ApiErrorBody } | null = null;
    try {
      body = JSON.parse(text) as { error?: ApiErrorBody };
    } catch {
      body = null;
    }
    throw new ApiFailure(
      response.status,
      body?.error ?? {
        code: "export_failed",
        message: `The export returned status ${response.status}.`,
      },
    );
  }
  const disposition = response.headers.get("Content-Disposition") ?? "";
  const match = /filename="([^"]+)"/.exec(disposition);
  const fallback = `rigcraft-recommendation.${format === "markdown" ? "md" : "json"}`;
  const filename = match?.[1] ?? fallback;
  const type =
    format === "markdown" ? "text/markdown;charset=utf-8" : "application/json";
  const url = URL.createObjectURL(new Blob([text], { type }));
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  document.body.append(anchor);
  anchor.click();
  anchor.remove();
  URL.revokeObjectURL(url);
  return filename;
}

export async function parseDiagnosisList(
  request: ParseListRequest,
): Promise<ParsedList> {
  const response = await fetch("/api/diagnoses/parse", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(request),
  });
  return readResponse<ParsedList>(response);
}

export async function diagnoseBuild(
  request: DiagnoseRequest,
): Promise<DiagnosisReport> {
  const response = await fetch("/api/diagnoses", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(request),
  });
  return readResponse<DiagnosisReport>(response);
}

export async function getDiagnosis(id: string): Promise<DiagnosisReport> {
  const response = await fetch(`/api/diagnoses/${encodeURIComponent(id)}`);
  return readResponse<DiagnosisReport>(response);
}

export async function parseWithAssistant(
  request: AiParseRequest,
): Promise<AiProposalResponse> {
  // The key travels in this body only. It is never stored, logged or reused.
  const response = await fetch("/api/ai/parse", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(request),
  });
  return readResponse<AiProposalResponse>(response);
}

export async function reviseWithAssistant(
  request: AiReviseRequest,
): Promise<AiProposalResponse> {
  const response = await fetch("/api/ai/revise", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(request),
  });
  return readResponse<AiProposalResponse>(response);
}

export const catalogCategories = [
  "cpu",
  "motherboard",
  "ram",
  "gpu",
  "psu",
  "case",
  "cooler",
  "storage",
] as const;

/** Upload the operator's own catalog document. Nothing is fetched or bundled. */
export async function uploadCatalog(
  document: unknown,
  acknowledged: boolean,
): Promise<CatalogReport> {
  const response = await fetch("/api/catalog/datasets", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ acknowledged, catalog: document }),
  });
  return readResponse<CatalogReport>(response);
}

export async function listCatalogDatasets(): Promise<CatalogDataset[]> {
  const response = await fetch("/api/catalog/datasets");
  return readResponse<CatalogDataset[]>(response);
}

export async function setCatalogDatasetActive(
  id: string,
  active: boolean,
): Promise<CatalogDataset> {
  const response = await fetch(
    `/api/catalog/datasets/${encodeURIComponent(id)}`,
    {
      method: "PATCH",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ active }),
    },
  );
  return readResponse<CatalogDataset>(response);
}

export async function removeCatalogDataset(id: string): Promise<void> {
  const response = await fetch(
    `/api/catalog/datasets/${encodeURIComponent(id)}?confirm=remove`,
    { method: "DELETE" },
  );
  await readResponse<{ removed: boolean }>(response);
}

export async function searchCatalog(
  query: string,
  category: string,
  limit = 20,
  offset = 0,
): Promise<CatalogSearchResult> {
  const params = new URLSearchParams({
    q: query,
    limit: String(limit),
    offset: String(offset),
  });
  if (category) {
    params.set("category", category);
  }
  const response = await fetch(`/api/catalog/search?${params.toString()}`);
  return readResponse<CatalogSearchResult>(response);
}

export async function getCatalogPart(id: string): Promise<CatalogPartDetail> {
  const response = await fetch(`/api/catalog/parts/${encodeURIComponent(id)}`);
  return readResponse<CatalogPartDetail>(response);
}

export async function matchCatalogRows(
  rows: CatalogMatchRow[],
): Promise<CatalogMatchResponse> {
  const response = await fetch("/api/catalog/match", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ rows }),
  });
  return readResponse<CatalogMatchResponse>(response);
}

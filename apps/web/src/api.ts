import type {
  AiParseRequest,
  AiProposalResponse,
  AiReviseRequest,
  ApiErrorBody,
  CheckReport,
  CheckRequest,
  CommitRequest,
  CommitResponse,
  CorrectionRequest,
  Preview,
  RecommendationReport,
  RecommendRequest,
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

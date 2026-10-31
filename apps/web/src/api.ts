import type {
  ApiErrorBody,
  CommitRequest,
  CommitResponse,
  CorrectionRequest,
  Preview,
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

export const sampleCsvUrl = "/api/imports/sample";

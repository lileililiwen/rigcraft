import { useEffect, useRef, useState, type FormEvent } from "react";
import type {
  BackupInspection,
  RestoreSummary,
  Snapshot,
  SnapshotSummary,
} from "../types";
import {
  ApiFailure,
  downloadBackup,
  downloadSnapshotExport,
  inspectBackup,
  restoreBackup,
} from "../api";
import { formatTimestamp } from "../inventory";
import { plural } from "../plural";

interface DataPortabilityProps {
  snapshot: Snapshot | null;
  history: SnapshotSummary[];
  onRestored: (summary: RestoreSummary) => void;
}

interface Problem {
  title: string;
  message: string;
  code?: string;
  preRestorePath?: string;
}

type Busy = "export" | "backup" | "inspect" | "restore" | null;

function describeFailure(error: unknown, action: string): Problem {
  if (error instanceof ApiFailure) {
    return {
      title: action === "inspect" ? "This backup cannot be restored" : "Restore failed",
      message: error.message,
      code: error.code,
      preRestorePath: error.details?.preRestorePath,
    };
  }
  return {
    title: action === "inspect" ? "This backup cannot be inspected" : "Restore failed",
    message:
      error instanceof Error
        ? error.message
        : "The local service could not be reached. Check that it is running.",
    code: "network_error",
  };
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) {
    return `${bytes} bytes`;
  }
  if (bytes < 1024 * 1024) {
    return `${(bytes / 1024).toFixed(1)} KiB`;
  }
  return `${(bytes / (1024 * 1024)).toFixed(1)} MiB`;
}

export function DataPortability({
  snapshot,
  history,
  onRestored,
}: DataPortabilityProps) {
  const [busy, setBusy] = useState<Busy>(null);
  const [notice, setNotice] = useState(
    "Exports and backups are created by this local service only.",
  );
  const [problem, setProblem] = useState<Problem | null>(null);
  const [file, setFile] = useState<File | null>(null);
  const [inspection, setInspection] = useState<BackupInspection | null>(null);
  const [restored, setRestored] = useState<RestoreSummary | null>(null);
  const [confirmation, setConfirmation] = useState("");
  const previewRef = useRef<HTMLHeadingElement>(null);
  const successRef = useRef<HTMLHeadingElement>(null);
  const errorRef = useRef<HTMLDivElement>(null);
  const fileInput = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (problem) {
      errorRef.current?.focus();
    }
  }, [problem]);

  useEffect(() => {
    if (restored) {
      successRef.current?.focus();
    } else if (inspection) {
      previewRef.current?.focus();
    }
  }, [inspection, restored]);

  const disabled = busy !== null;
  const confirmed = confirmation.trim().toUpperCase() === "RESTORE";
  const newest = history[0] ?? null;

  async function exportSnapshot(format: "csv" | "json") {
    if (!snapshot) {
      return;
    }
    setBusy("export");
    setProblem(null);
    try {
      const { filename, note } = await downloadSnapshotExport(snapshot.id, format);
      setNotice(
        note
          ? `Downloaded ${filename}. ${note}`
          : `Downloaded ${filename}. This file contains the complete snapshot.`,
      );
    } catch (error) {
      setProblem(describeFailure(error, "export"));
    } finally {
      setBusy(null);
    }
  }

  async function backup() {
    setBusy("backup");
    setProblem(null);
    try {
      const filename = await downloadBackup();
      setNotice(
        `Downloaded ${filename}. It is a standard SQLite file containing the entire private inventory.`,
      );
    } catch (error) {
      setProblem(describeFailure(error, "backup"));
    } finally {
      setBusy(null);
    }
  }

  async function inspect(event: FormEvent) {
    event.preventDefault();
    if (!file) {
      setProblem({
        title: "Choose a file",
        message: "Select the SQLite backup you want to inspect before replacing anything.",
      });
      return;
    }
    setBusy("inspect");
    setProblem(null);
    setInspection(null);
    setRestored(null);
    setConfirmation("");
    try {
      const result = await inspectBackup(file);
      setInspection(result);
      setNotice(
        `Inspected ${result.fileName}: integrity ${result.integrity}, schema ${result.counts.schemaVersion}, ${plural(result.counts.recordCount, "record")}. Nothing has been replaced.`,
      );
    } catch (error) {
      setProblem(describeFailure(error, "inspect"));
    } finally {
      setBusy(null);
    }
  }

  async function restore(event: FormEvent) {
    event.preventDefault();
    if (!file || !inspection) {
      return;
    }
    if (!confirmed) {
      setProblem({
        title: "Confirmation required",
        message: "Type RESTORE to confirm replacing the live inventory.",
      });
      return;
    }
    setBusy("restore");
    setProblem(null);
    setNotice("Restoring from the inspected backup. A safety copy is written first.");
    try {
      const summary = await restoreBackup(file);
      setRestored(summary);
      setInspection(null);
      setConfirmation("");
      setFile(null);
      if (fileInput.current) {
        fileInput.current.value = "";
      }
      setNotice(
        `Restored ${plural(summary.counts.snapshotCount, "snapshot")} and ${plural(
          summary.counts.recordCount,
          "record",
        )}. The previous catalog is in the pre-restore copy.`,
      );
      onRestored(summary);
    } catch (error) {
      setProblem(describeFailure(error, "restore"));
      setNotice("The restore did not complete. Review the error before trying again.");
    } finally {
      setBusy(null);
    }
  }

  return (
    <section className="panel" aria-labelledby="portability-title">
      <div className="panel__head">
        <p className="caps-label" aria-hidden="true">
          Portability
        </p>
        <h2 className="panel__title" id="portability-title">
          Export and backup
        </h2>
      </div>
      <p className="lede">
        Take the catalog with you, or put it back. JSON is the authoritative export;
        CSV uses the import columns and is lossy. A backup is a standard SQLite file
        and contains the shop&apos;s entire private inventory.
      </p>
      <div
        className="status-line"
        role="status"
        aria-live="polite"
        aria-busy={disabled}
        data-busy={disabled}
      >
        <span>{notice}</span>
      </div>

      {problem ? (
        <div className="alert" role="alert" tabIndex={-1} ref={errorRef}>
          <h2>{problem.title}</h2>
          <p>{problem.message}</p>
          {problem.code ? (
            <p className="small muted">
              Code: <span className="mono">{problem.code}</span>
            </p>
          ) : null}
          {problem.preRestorePath ? (
            <p className="small">
              Previous inventory: <span className="mono">{problem.preRestorePath}</span>
            </p>
          ) : null}
          <div className="button-row">
            <button
              type="button"
              className="button--secondary"
              onClick={() => setProblem(null)}
            >
              Dismiss
            </button>
          </div>
        </div>
      ) : null}

      <div className="field-grid portability-grid">
        <div className="field">
          <h3>Export this snapshot</h3>
          <p className="field__hint" id="export-hint">
            CSV carries the import columns only: no lot identity, snapshot metadata or
            per-field provenance reference. JSON keeps everything.
          </p>
          <div className="button-row">
            <button
              type="button"
              className="button--secondary"
              disabled={!snapshot || disabled}
              aria-describedby="export-hint"
              onClick={() => void exportSnapshot("csv")}
            >
              Download CSV
            </button>
            <button
              type="button"
              className="button--secondary"
              disabled={!snapshot || disabled}
              aria-describedby="export-hint"
              onClick={() => void exportSnapshot("json")}
            >
              Download JSON
            </button>
          </div>
          <p className="small muted">
            {snapshot
              ? `Ready: revision ${snapshot.revision}, ${plural(
                  snapshot.records.length,
                  "record",
                )}.`
              : "No snapshot is open. Choose View on a revision in Snapshot history, or commit an import."}
          </p>
        </div>

        <div className="field">
          <h3>Back up this database</h3>
          <p className="field__hint" id="backup-hint">
            One consistent copy of every snapshot, correction and derived report, as a
            plain SQLite file any SQLite tool can inspect.
          </p>
          <div className="button-row">
            <button
              type="button"
              className="button--secondary"
              disabled={disabled}
              aria-describedby="backup-hint"
              onClick={() => void backup()}
            >
              Download backup
            </button>
          </div>
        </div>
      </div>

      <form className="restore-panel" onSubmit={(event) => void inspect(event)}>
        <h3>Restore from a backup</h3>
        <p className="small muted">
          Inspecting reads the file without changing anything. Replacing is a separate,
          explicitly confirmed step.
        </p>
        <div className="field">
          <label htmlFor="backup-file">SQLite backup file</label>
          <input
            ref={fileInput}
            id="backup-file"
            name="backup"
            type="file"
            accept=".sqlite3,.sqlite,.db,application/vnd.sqlite3,application/x-sqlite3"
            disabled={disabled}
            aria-describedby="backup-file-hint"
            onChange={(event) => {
              setFile(event.target.files?.[0] ?? null);
              setInspection(null);
              setRestored(null);
              setConfirmation("");
              setProblem(null);
            }}
          />
          <p className="field__hint" id="backup-file-hint">
            {file
              ? `Selected ${file.name} (${formatBytes(file.size)}).`
              : "Choose a RigCraft backup or another RigCraft SQLite catalog."}
          </p>
        </div>
        <div className="button-row">
          <button type="submit" disabled={!file || disabled}>
            {busy === "inspect" ? "Inspecting…" : "Inspect backup"}
          </button>
        </div>
      </form>

      {inspection ? (
        <div
          className="restore-preview"
          aria-labelledby="restore-preview-title"
          id="restore-preview"
        >
          <h3 id="restore-preview-title" tabIndex={-1} ref={previewRef}>
            Backup inspected — nothing replaced yet
          </h3>
          <dl className="summary-grid">
            <div>
              <dt>Integrity</dt>
              <dd>{inspection.integrity}</dd>
            </div>
            <div>
              <dt>Schema version</dt>
              <dd>{inspection.counts.schemaVersion}</dd>
            </div>
            <div>
              <dt>Created</dt>
              <dd>
                {inspection.counts.createdAt ? (
                  <time dateTime={inspection.counts.createdAt}>
                    {formatTimestamp(inspection.counts.createdAt)}
                  </time>
                ) : (
                  "No snapshots"
                )}
              </dd>
            </div>
            <div>
              <dt>Snapshots</dt>
              <dd>{inspection.counts.snapshotCount}</dd>
            </div>
            <div>
              <dt>Latest revision</dt>
              <dd>{inspection.counts.latestRevision ?? "None"}</dd>
            </div>
            <div>
              <dt>Records</dt>
              <dd>{inspection.counts.recordCount}</dd>
            </div>
            <div>
              <dt>Lots in newest snapshot</dt>
              <dd>{inspection.counts.lotCount}</dd>
            </div>
          </dl>

          <div className="alert" role="note">
            <h2>What will be replaced</h2>
            <p>
              The live catalog at this service
              {history.length === 0
                ? " currently holds no snapshots"
                : ` currently holds ${plural(history.length, "snapshot")}${
                    newest
                      ? `, newest revision ${newest.revision} with ${plural(
                          newest.recordCount,
                          "record",
                        )}`
                      : ""
                  }`}
              . Replacing cannot be undone from this page; a pre-restore safety copy is
              written first and is never deleted automatically.
            </p>
          </div>

          <form onSubmit={(event) => void restore(event)}>
            <div className="field">
              <label htmlFor="restore-confirmation">
                Type <span className="mono">RESTORE</span> to confirm
              </label>
              <input
                id="restore-confirmation"
                autoComplete="off"
                spellCheck={false}
                value={confirmation}
                disabled={disabled}
                aria-describedby="restore-confirmation-hint"
                onChange={(event) => setConfirmation(event.target.value)}
              />
              <p className="field__hint" id="restore-confirmation-hint">
                The replace button stays disabled until the word matches. One click
                cannot replace the inventory.
              </p>
            </div>
            <div className="button-row">
              <button
                type="submit"
                className="button--danger"
                disabled={!confirmed || disabled}
              >
                {busy === "restore" ? "Replacing inventory…" : "Replace this inventory"}
              </button>
              <button
                type="button"
                className="button--secondary"
                disabled={disabled}
                onClick={() => {
                  setInspection(null);
                  setConfirmation("");
                  setFile(null);
                  if (fileInput.current) {
                    fileInput.current.value = "";
                  }
                  setNotice("Restore cancelled. Nothing was replaced.");
                  fileInput.current?.focus();
                }}
              >
                Cancel
              </button>
            </div>
          </form>
        </div>
      ) : null}

      {restored ? (
        <div className="restore-success" aria-labelledby="restore-success-title">
          <h3 id="restore-success-title" tabIndex={-1} ref={successRef}>
            Inventory restored
          </h3>
          <p>
            Schema {restored.sourceSchemaVersion} is now at{" "}
            {restored.counts.schemaVersion}.{" "}
            {plural(restored.counts.snapshotCount, "snapshot")},{" "}
            {restored.counts.latestRevision === null
              ? "no revisions"
              : `latest revision ${restored.counts.latestRevision}`}
            , {plural(restored.counts.recordCount, "record")} and{" "}
            {plural(restored.counts.lotCount, "lot")} in the newest snapshot.
          </p>
          <p className="small">
            Pre-restore safety copy:{" "}
            <span className="mono" id="pre-restore-path">
              {restored.preRestorePath}
            </span>
          </p>
        </div>
      ) : null}
    </section>
  );
}

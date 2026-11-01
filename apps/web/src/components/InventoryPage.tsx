import { useEffect, useRef, useState } from "react";
import type {
  CatalogMatch,
  CommitRequest,
  DuplicateAction,
  FieldError,
  Preview,
  RestoreSummary,
  Snapshot,
  SnapshotSummary,
} from "../types";
import {
  ApiFailure,
  commitImport,
  getSnapshot,
  listSnapshots,
  matchCatalogRows,
  previewImport,
} from "../api";
import {
  canCommit,
  continuationSummary,
  defaultDecisions,
  formatTimestamp,
  formatMoney,
  isValidCurrencyCode,
  shortId,
} from "../inventory";
import type { DecisionMap, OverrideMap, ResolutionMap } from "../inventory";
import { plural } from "../plural";
import { ImportForm } from "./ImportForm";
import { PreviewReview } from "./PreviewReview";
import { SiteHeader } from "./SiteHeader";
import { SnapshotPanel } from "./SnapshotPanel";
import { DataPortability } from "./DataPortability";

type Phase = "empty" | "parsing" | "review" | "committing" | "committed" | "failed";

interface Problem {
  title: string;
  message: string;
  code?: string;
  rows?: Array<{ row: number; errors: FieldError[] }>;
}

function describeFailure(error: unknown): Problem {
  if (error instanceof ApiFailure) {
    return {
      title: error.status === 422 ? "Input needs correction" : "Request failed",
      message: error.message,
      code: error.code,
      rows: (error.details?.rows ?? []).filter(
        (row): row is { row: number; errors: FieldError[] } => "errors" in row,
      ),
    };
  }
  return {
    title: "Request failed",
    message:
      error instanceof Error
        ? error.message
        : "The local service could not be reached. Check that it is running.",
    code: "network_error",
  };
}

function issueMap(problem: Problem): Record<number, FieldError[]> {
  const map: Record<number, FieldError[]> = {};
  for (const entry of problem.rows ?? []) {
    if ("errors" in entry) {
      map[entry.row] = entry.errors;
    }
  }
  return map;
}

export function InventoryPage() {
  const [file, setFile] = useState<File | null>(null);
  const [currency, setCurrency] = useState("USD");
  const [mapping, setMapping] = useState<Record<string, string>>({});
  const [preview, setPreview] = useState<Preview | null>(null);
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [history, setHistory] = useState<SnapshotSummary[]>([]);
  const [historyError, setHistoryError] = useState<string | null>(null);
  const [phase, setPhase] = useState<Phase>("empty");
  const [problem, setProblem] = useState<Problem | null>(null);
  const [retry, setRetry] = useState<(() => void) | null>(null);
  const [decisions, setDecisions] = useState<DecisionMap>({});
  const [resolutions, setResolutions] = useState<ResolutionMap>({});
  const [overrides, setOverrides] = useState<OverrideMap>({});
  const [serverIssues, setServerIssues] = useState<Record<number, FieldError[]>>({});
  const [catalogMatches, setCatalogMatches] = useState<Record<number, CatalogMatch>>({});
  const [catalogOptIn, setCatalogOptIn] = useState<Record<number, string>>({});
  const [catalogNotice, setCatalogNotice] = useState<string | null>(null);
  const [live, setLive] = useState("No CSV selected.");
  const [initialising, setInitialising] = useState(true);
  const errorRef = useRef<HTMLDivElement>(null);
  const statusRef = useRef<HTMLDivElement>(null);
  // The newest revision opens once, on first load. `suppressFocus` keeps that
  // open from stealing focus: the first Tab stop on the page stays the skip link.
  const openedOnLoad = useRef(false);
  const suppressFocus = useRef(false);

  async function refreshHistory() {
    try {
      const summaries = await listSnapshots();
      setHistory(summaries);
      setHistoryError(null);
      return summaries;
    } catch (error) {
      setHistoryError(
        error instanceof ApiFailure ? error.message : "Snapshot history is unavailable.",
      );
      return null;
    }
  }

  useEffect(() => {
    void (async () => {
      const summaries = await refreshHistory();
      if (!openedOnLoad.current && summaries && summaries.length > 0) {
        openedOnLoad.current = true;
        await viewSnapshot(summaries[0].id, true);
      }
      // Everything below the status line waits for the first snapshot state, so
      // the automatic open appends its bar and records instead of pushing the
      // import panel down after paint (a real layout shift).
      setInitialising(false);
    })();
  }, []);

  useEffect(() => {
    if (phase === "review") {
      document.getElementById("review-title")?.focus();
    } else if (phase === "committed") {
      if (suppressFocus.current) {
        suppressFocus.current = false;
      } else {
        document.getElementById("snapshot-title")?.focus();
      }
    } else if (phase === "parsing" || phase === "committing") {
      statusRef.current?.focus();
    }
  }, [phase, snapshot?.id]);

  useEffect(() => {
    if (problem) {
      errorRef.current?.focus();
    }
  }, [problem]);

  async function loadCatalogMatches(next: Preview) {
    const rows = next.rows
      .filter((row) => (row.normalized.category ?? row.values.category).trim() !== "")
      .map((row) => ({
        row: row.row,
        sku: row.values.sku,
        category: row.normalized.category ?? row.values.category,
        model: row.values.model,
        specs: row.values.specs,
      }));
    if (rows.length === 0) {
      return;
    }
    try {
      const { matches } = await matchCatalogRows(rows);
      const map: Record<number, CatalogMatch> = {};
      for (const match of matches) {
        map[match.row] = match;
      }
      setCatalogMatches(map);
      setCatalogNotice(null);
    } catch (error) {
      setCatalogMatches({});
      setCatalogNotice(
        error instanceof ApiFailure
          ? `Catalog matching is unavailable: ${error.message}`
          : "Catalog matching is unavailable. Your rows and values are unaffected.",
      );
    }
  }

  async function runPreview(nextMapping: Record<string, string> = mapping) {
    if (!file) {
      setProblem({
        title: "Choose a file",
        message: "Select a UTF-8 CSV before previewing.",
      });
      return;
    }
    if (!isValidCurrencyCode(currency)) {
      setProblem({
        title: "Check the currency",
        message: "Use a three-letter ISO 4217 code such as USD, EUR, CNY or JPY.",
      });
      return;
    }
    setPhase("parsing");
    setProblem(null);
    setRetry(null);
    setServerIssues({});
    setCatalogMatches({});
    setCatalogOptIn({});
    setCatalogNotice(null);
    setLive("Parsing CSV and validating every row…");
    try {
      const result = await previewImport(file, currency.trim(), nextMapping);
      setPreview(result);
      setDecisions(defaultDecisions(result));
      setResolutions({});
      setOverrides({});
      setSnapshot(null);
      setMapping(result.mapping);
      setPhase("review");
      setLive(`Preview ready: ${continuationSummary(result.rows)}.`);
      void loadCatalogMatches(result);
    } catch (error) {
      const next = describeFailure(error);
      setProblem(next);
      setServerIssues(issueMap(next));
      setPhase("failed");
      setLive("Preview failed. Fix the problem or retry.");
      setRetry(() => () => void runPreview(nextMapping));
    }
  }

  async function runCommit() {
    if (!preview) {
      return;
    }
    const readiness = canCommit(
      preview,
      decisions,
      resolutions,
      overrides,
      serverIssues,
    );
    if (!readiness.ready) {
      setProblem({
        title: "The snapshot is not ready",
        message: readiness.reason,
        code: "commit_blocked",
      });
      setRetry(null);
      return;
    }
    const request: CommitRequest = {
      decisions: preview.rows.map((row) => ({
        row: row.row,
        action: decisions[row.row] ?? "exclude",
        overrides: overrides[row.row],
        catalog: catalogOptIn[row.row]
          ? { partId: catalogOptIn[row.row] }
          : undefined,
      })),
      duplicateResolutions: Object.entries(resolutions)
        .filter((entry): entry is [string, DuplicateAction] => Boolean(entry[1]))
        .map(([groupId, action]) => ({ groupId, action })),
    };
    setPhase("committing");
    setProblem(null);
    setRetry(null);
    setLive("Committing the snapshot. Nothing is written until every row passes.");
    try {
      const result = await commitImport(preview.previewId, request);
      setSnapshot(result.snapshot);
      setPhase("committed");
      setLive(
        `Snapshot committed. Revision ${result.snapshot.revision}, ${plural(
          result.snapshot.totals.recordCount,
          "record",
        )}, ${formatMoney(
          result.snapshot.totals.valueMinor,
          result.snapshot.currency,
        )} stock value.`,
      );
      // The committed file belongs to the snapshot; the import panel is ready
      // for the next batch instead of offering to preview the same file again.
      setFile(null);
      setProblem(null);
      await refreshHistory();
    } catch (error) {
      const next = describeFailure(error);
      if (error instanceof ApiFailure && error.code === "invalid_row") {
        const rows = issueMap(next);
        setServerIssues(rows);
        setPhase("review");
        setProblem({
          ...next,
          title: "A row marked for commit is still invalid",
          message:
            "Correct the highlighted row or choose Exclude, then commit again. Nothing was written.",
        });
      } else {
        setPhase("failed");
        setProblem(next);
        setLive("Commit failed. Nothing was written; you can retry.");
        setRetry(() => () => void runCommit());
      }
    }
  }

  async function viewSnapshot(id: string, initial = false) {
    try {
      const next = await getSnapshot(id);
      suppressFocus.current = initial;
      setSnapshot(next);
      setPhase("committed");
      setLive(
        initial
          ? `Opened the newest snapshot, revision ${next.revision}: ${plural(
              next.totals.recordCount,
              "record",
            )}, ${formatMoney(next.totals.valueMinor, next.currency)} stock value. Export and backup are ready below.`
          : `Viewing snapshot revision ${next.revision}.`,
      );
      setProblem(null);
      setRetry(null);
    } catch (error) {
      const next = describeFailure(error);
      setProblem(next);
      setLive("The snapshot could not be opened.");
    }
  }

  function startOver() {
    setFile(null);
    setMapping({});
    setPreview(null);
    setSnapshot(null);
    setPhase("empty");
    setProblem(null);
    setRetry(null);
    setDecisions({});
    setResolutions({});
    setOverrides({});
    setServerIssues({});
    setCatalogMatches({});
    setCatalogOptIn({});
    setCatalogNotice(null);
    setLive("Ready for a new CSV import.");
    window.setTimeout(() => document.getElementById("inventory-file")?.focus(), 0);
  }

  async function handleRestored(summary: RestoreSummary) {
    setProblem(null);
    setRetry(null);
    const summaries = await refreshHistory();
    if (summaries && summaries.length > 0) {
      try {
        const next = await getSnapshot(summaries[0].id);
        setSnapshot(next);
        setPreview(null);
        setPhase("committed");
        setLive(
          `Restored ${plural(summary.counts.recordCount, "record")}. Viewing revision ${next.revision}.`,
        );
        return;
      } catch (error) {
        setProblem(describeFailure(error));
      }
    }
    setSnapshot(null);
    setPreview(null);
    setPhase("empty");
    setLive("The restored database has no snapshots. Import a CSV to start.");
  }

  const latestRevision =
    history.length > 0 ? Math.max(...history.map((entry) => entry.revision)) : snapshot?.revision ?? 1;
  const busy = phase === "parsing" || phase === "committing";
  const showPreview = preview !== null && phase !== "committed";
  const openSnapshot = phase === "committed" ? snapshot : null;
  const snapshotIsLatest = openSnapshot ? openSnapshot.revision === latestRevision : false;

  return (
    <>
      <SiteHeader current="inventory" />
      <main className="page" id="main" tabIndex={-1} aria-labelledby="inventory-page-title">
        <h1 className="page-title" id="inventory-page-title">
          Inventory
        </h1>
        <p className="lede">
          Import owned hardware from a UTF-8 CSV, correct every bad row, and commit
          a versioned snapshot. Each value keeps its original input, normalized
          value, and provenance.
        </p>
        <p className="privacy-note">
          Inventory is private to this local service. It is not uploaded to a
          catalog, shared with an AI provider, or logged.
        </p>

        <div
          className="status-line"
          role="status"
          aria-live="polite"
          aria-busy={busy}
          data-busy={busy}
          tabIndex={-1}
          ref={statusRef}
        >
          <span>{live}</span>
        </div>

        {initialising ? (
          <p className="small muted">Loading the snapshot state…</p>
        ) : (
          <>
        {openSnapshot ? (
          <section className="console-bar" aria-labelledby="console-bar-title">
            <div className="console-bar__head">
              <p className="caps-label" aria-hidden="true">
                Open snapshot
              </p>
              <span
                className={`badge badge--${snapshotIsLatest ? "valid" : "unknown"}`}
              >
                <span className="badge__symbol" aria-hidden="true">
                  {snapshotIsLatest ? "✓" : "?"}
                </span>
                {snapshotIsLatest ? "Latest revision" : "Earlier revision"}
              </span>
            </div>
            <h2 className="console-bar__title" id="console-bar-title">
              Snapshot state
            </h2>
            <dl className="console-stats">
              <div>
                <dt>Revision</dt>
                <dd>{openSnapshot.revision}</dd>
              </div>
              <div>
                <dt>Records</dt>
                <dd>{openSnapshot.totals.recordCount}</dd>
              </div>
              <div>
                <dt>Units in stock</dt>
                <dd>{openSnapshot.totals.unitCount}</dd>
              </div>
              <div>
                <dt>Stock value</dt>
                <dd>{formatMoney(openSnapshot.totals.valueMinor, openSnapshot.currency)}</dd>
              </div>
            </dl>
            <p className="small muted console-bar__meta">
              Source <span className="mono">{openSnapshot.source.filename}</span> ·
              committed{" "}
              <time dateTime={openSnapshot.createdAt}>
                {formatTimestamp(openSnapshot.createdAt)}
              </time>
            </p>
          </section>
        ) : null}

        {problem ? (
          <div className="alert" role="alert" tabIndex={-1} ref={errorRef}>
            <h2>{problem.title}</h2>
            <p>{problem.message}</p>
            {problem.code ? (
              <p className="small muted">
                Code: <span className="mono">{problem.code}</span>
              </p>
            ) : null}
            {problem.rows && problem.rows.length > 0 ? (
              <ul>
                {problem.rows.map((entry) => (
                  <li key={entry.row}>
                    Row {entry.row}:{" "}
                    {entry.errors
                      .map((error) => `${error.field} — ${error.message}`)
                      .join("; ")}
                  </li>
                ))}
              </ul>
            ) : null}
            <div className="button-row">
              {retry ? (
                <button type="button" onClick={retry} disabled={busy}>
                  Retry
                </button>
              ) : null}
              <button
                type="button"
                className="button--secondary"
                onClick={() => {
                  setProblem(null);
                  setRetry(null);
                }}
              >
                Dismiss
              </button>
            </div>
          </div>
        ) : null}

        {showPreview && preview ? (
          <PreviewReview
            preview={preview}
            catalogMatches={catalogMatches}
            catalogOptIn={catalogOptIn}
            onCatalogOptIn={(row, partId) =>
              setCatalogOptIn((current) => {
                const next = { ...current };
                if (partId) {
                  next[row] = partId;
                } else {
                  delete next[row];
                }
                return next;
              })
            }
            decisions={decisions}
            resolutions={resolutions}
            overrides={overrides}
            serverIssues={serverIssues}
            busy={busy}
            onDecision={(row, action) => {
              setDecisions((current) => ({ ...current, [row]: action }));
              setServerIssues((current) => {
                const next = { ...current };
                delete next[row];
                return next;
              });
            }}
            onResolution={(groupId, action) =>
              setResolutions((current) => ({ ...current, [groupId]: action }))
            }
            onOverride={(row, next) =>
              setOverrides((current) => ({ ...current, [row]: next }))
            }
            onCommit={() => void runCommit()}
            onReparse={(nextMapping) => {
              setMapping(nextMapping);
              void runPreview(nextMapping);
            }}
            onStartOver={startOver}
          />
        ) : null}

        {openSnapshot ? (
          <>
            <SnapshotPanel
              snapshot={openSnapshot}
              isLatest={snapshotIsLatest}
              onView={(id) => void viewSnapshot(id)}
              onCorrected={(corrected) => {
                setSnapshot(corrected);
                setLive(
                  `Revision ${corrected.revision} saved from revision ${
                    corrected.revision - 1
                  }. The earlier snapshot is unchanged.`,
                );
                void refreshHistory();
              }}
            />
            <div className="button-row">
              <button type="button" className="button--secondary" onClick={startOver}>
                Start a new import
              </button>
            </div>
          </>
        ) : null}

        {!preview ? (
          <ImportForm
            file={file}
            currency={currency}
            busy={busy}
            onFile={(next) => {
              setFile(next);
              setProblem(null);
            }}
            onCurrency={setCurrency}
            onSubmit={() => void runPreview()}
          />
        ) : null}

        {catalogNotice ? <p className="small muted">{catalogNotice}</p> : null}

        {history.length > 0 || historyError ? (
          <section className="panel" aria-labelledby="history-title">
            <div className="panel__head">
              <p className="caps-label" aria-hidden="true">
                Revisions
              </p>
              <h2 className="panel__title" id="history-title">
                Snapshot history
              </h2>
            </div>
            {historyError ? (
              <p className="field__error">{historyError}</p>
            ) : null}
            <p className="small muted">
              Every revision stays readable. Opening an older one shows its exact
              records and provenance.
            </p>
            <ul className="history-list">
              {history.map((entry) => {
                const current = snapshot?.id === entry.id && phase === "committed";
                return (
                  <li
                    key={entry.id}
                    className={`history-card${current ? " history-card--current" : ""}`}
                  >
                    <div className="history-card__head">
                      <h3 className="history-card__title">
                        Revision {entry.revision}
                      </h3>
                      {current ? (
                        <span className="badge badge--valid">
                          <span className="badge__symbol" aria-hidden="true">
                            ✓
                          </span>
                          Viewing now
                        </span>
                      ) : null}
                    </div>
                    <dl className="history-card__rows">
                      <div>
                        <dt>Committed</dt>
                        <dd>
                          <time dateTime={entry.createdAt}>
                            {formatTimestamp(entry.createdAt)}
                          </time>
                        </dd>
                      </div>
                      <div>
                        <dt>Records</dt>
                        <dd>{entry.recordCount}</dd>
                      </div>
                      <div>
                        <dt>Stock value</dt>
                        <dd>{formatMoney(entry.valueMinor, entry.currency)}</dd>
                      </div>
                    </dl>
                    {entry.note ? (
                      <p className="small muted">{entry.note}</p>
                    ) : null}
                    <div className="button-row">
                      <button
                        type="button"
                        className="button--secondary"
                        aria-current={current ? "true" : undefined}
                        onClick={() => void viewSnapshot(entry.id)}
                      >
                        View {shortId(entry.id)}
                      </button>
                    </div>
                  </li>
                );
              })}
            </ul>
          </section>
        ) : null}

        <DataPortability
          snapshot={snapshot}
          history={history}
          onRestored={(summary) => void handleRestored(summary)}
        />
          </>
        )}
      </main>
    </>
  );
}

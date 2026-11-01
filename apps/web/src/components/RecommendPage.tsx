import { useEffect, useRef, useState } from "react";
import type { RecommendationReport, RecordItem, Snapshot, SnapshotSummary } from "../types";
import {
  ApiFailure,
  downloadRecommendationExport,
  getSnapshot,
  listSnapshots,
  recommendBuild,
} from "../api";
import { formatTimestamp, formatMoney, shortId } from "../inventory";
import {
  USE_CASES,
  parseBudgetMinor,
  plural,
  requestSignature,
  selectedRequest,
} from "../recommend";
import { AssistantPanel } from "./AssistantPanel";
import type { AppliedProposal } from "./AssistantPanel";
import { RecommendOptions } from "./RecommendOptions";
import type { ExportNotice } from "./RecommendOptions";
import { RecommendVerdict } from "./RecommendVerdict";
import { SiteHeader } from "./SiteHeader";

type Phase = "idle" | "generating" | "done" | "failed";

interface Problem {
  title: string;
  message: string;
  code?: string;
}

function describeFailure(error: unknown): Problem {
  if (error instanceof ApiFailure) {
    return {
      title: error.status === 422 ? "The recommendation could not run" : "Request failed",
      message: error.message,
      code: error.code,
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

export function RecommendPage() {
  const [snapshots, setSnapshots] = useState<SnapshotSummary[] | null>(null);
  const [historyProblem, setHistoryProblem] = useState<string | null>(null);
  const [snapshotId, setSnapshotId] = useState("");
  const [snapshotToken, setSnapshotToken] = useState(0);
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [snapshotBusy, setSnapshotBusy] = useState(false);
  const [snapshotProblem, setSnapshotProblem] = useState<string | null>(null);
  const [budget, setBudget] = useState("");
  const [useCase, setUseCase] = useState("gaming");
  const [locked, setLocked] = useState<Record<string, boolean>>({});
  const [excluded, setExcluded] = useState<Record<string, boolean>>({});
  const [phase, setPhase] = useState<Phase>("idle");
  const [report, setReport] = useState<RecommendationReport | null>(null);
  const [problem, setProblem] = useState<Problem | null>(null);
  const [retry, setRetry] = useState<(() => void) | null>(null);
  const [live, setLive] = useState("Choose a snapshot, enter a budget and generate.");
  const [exportNotice, setExportNotice] = useState<ExportNotice | null>(null);
  const [exporting, setExporting] = useState<"json" | "markdown" | null>(null);
  const [generatedSignature, setGeneratedSignature] = useState<string | null>(null);
  const statusRef = useRef<HTMLDivElement>(null);
  const errorRef = useRef<HTMLDivElement>(null);
  const resultsRef = useRef<HTMLHeadingElement>(null);
  const budgetRef = useRef<HTMLInputElement>(null);
  const generateRef = useRef<HTMLButtonElement>(null);

  // The checkboxes are keyed by record id, but the API takes SKUs. Mapping here
  // keeps the lock/exclude lists valid for the recommendation contract.
  const skuFor = (recordId: string) =>
    snapshot?.records.find((record) => record.id === recordId)?.sku;
  const lockedList = Object.keys(locked)
    .filter((key) => locked[key])
    .map(skuFor)
    .filter((sku): sku is string => Boolean(sku));
  const excludedList = Object.keys(excluded)
    .filter((key) => excluded[key])
    .map(skuFor)
    .filter((sku): sku is string => Boolean(sku));
  const currentSignature = requestSignature(
    snapshotId,
    budget,
    useCase,
    lockedList,
    excludedList,
  );
  const staleReport = Boolean(report && generatedSignature !== currentSignature);

  useEffect(() => {
    let cancelled = false;
    listSnapshots()
      .then((list) => {
        if (cancelled) {
          return;
        }
        setSnapshots(list);
        setHistoryProblem(null);
        if (list.length > 0) {
          setSnapshotId(list[0].id);
          setLive(
            `Ready. ${plural(list.length, "snapshot")} available; the newest is revision ${list[0].revision}.`,
          );
        } else {
          setLive("No inventory snapshot is available. Import a CSV first.");
        }
      })
      .catch((error) => {
        if (cancelled) {
          return;
        }
        setSnapshots([]);
        setHistoryProblem(
          error instanceof ApiFailure ? error.message : "Snapshot history is unavailable.",
        );
        setLive("Snapshot history could not be loaded.");
      });
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (!snapshotId) {
      setSnapshot(null);
      return;
    }
    let cancelled = false;
    setSnapshotBusy(true);
    setSnapshotProblem(null);
    setLive("Loading the inventory snapshot…");
    getSnapshot(snapshotId)
      .then((next) => {
        if (cancelled) {
          return;
        }
        setSnapshot(next);
        setLocked({});
        setExcluded({});
        setSnapshotBusy(false);
        setLive(
          `Snapshot revision ${next.revision} loaded: ${plural(
            next.records.length,
            "record",
          )}, currency ${next.currency}.`,
        );
      })
      .catch((error) => {
        if (cancelled) {
          return;
        }
        setSnapshot(null);
        setSnapshotBusy(false);
        setSnapshotProblem(
          error instanceof ApiFailure
            ? `${error.message} (${error.code})`
            : "The snapshot could not be loaded. Check the service and retry.",
        );
        setLive("The snapshot could not be loaded.");
      });
    return () => {
      cancelled = true;
    };
  }, [snapshotId, snapshotToken]);

  useEffect(() => {
    if (phase === "generating") {
      statusRef.current?.focus();
    }
  }, [phase]);

  useEffect(() => {
    if (phase === "done" && report) {
      resultsRef.current?.focus();
    }
  }, [phase, report]);

  useEffect(() => {
    if (problem) {
      errorRef.current?.focus();
    }
  }, [problem]);

  function toggleLocked(record: RecordItem, checked: boolean) {
    setLocked((current) => ({ ...current, [record.id]: checked }));
    if (checked) {
      setExcluded((current) => ({ ...current, [record.id]: false }));
    }
  }

  function toggleExcluded(record: RecordItem, checked: boolean) {
    setExcluded((current) => ({ ...current, [record.id]: checked }));
    if (checked) {
      setLocked((current) => ({ ...current, [record.id]: false }));
    }
  }

  async function generate() {
    if (!snapshot) {
      return;
    }
    const budgetMinor = parseBudgetMinor(budget);
    if (budgetMinor === null) {
      setProblem({
        title: "Enter a budget",
        message:
          "Use a positive amount with at most two decimals, for example 1200 or 1299.50.",
        code: "budget_invalid",
      });
      setRetry(null);
      return;
    }
    setPhase("generating");
    setProblem(null);
    setRetry(null);
    setExportNotice(null);
    setLive(
      "Searching the inventory for complete, compatible builds within the budget…",
    );
    try {
      const result = await recommendBuild(
        selectedRequest(
          snapshot.id,
          budgetMinor,
          snapshot.currency,
          useCase,
          lockedList,
          excludedList,
        ),
      );
      setReport(result);
      setGeneratedSignature(currentSignature);
      setPhase("done");
      if (result.status === "no_solution") {
        setLive(
          `No solution. ${result.noSolution?.headline ?? "The constraints cannot be satisfied."}`,
        );
      } else if (result.status === "search_limited") {
        setLive(
          `Search limit reached. ${plural(
            result.candidates.length,
            "best-effort option",
          )} shown; no global optimum is claimed.`,
        );
      } else {
        setLive(
          `Recommendation complete: ${plural(
            result.candidates.length,
            "materially different option",
          )} within ${formatMoney(result.budgetMinor, result.currency)}.`,
        );
      }
    } catch (error) {
      const next = describeFailure(error);
      setProblem(next);
      setPhase("failed");
      setLive("The recommendation failed. Your request is kept; retry when the service is available.");
      setRetry(() => () => void generate());
    }
  }

  /**
   * Apply a confirmed proposal to the deterministic form. The AI layer only
   * fills these fields; the operator still generates the search explicitly.
   */
  function applyAssistant(next: AppliedProposal) {
    setBudget(next.budgetText);
    setUseCase(next.useCase);
    const lockedSkus = new Set(next.lockedSkus);
    const excludedSkus = new Set(next.excludedSkus);
    const nextLocked: Record<string, boolean> = {};
    const nextExcluded: Record<string, boolean> = {};
    for (const record of snapshot?.records ?? []) {
      nextLocked[record.id] = lockedSkus.has(record.sku);
      nextExcluded[record.id] = excludedSkus.has(record.sku);
    }
    setLocked(nextLocked);
    setExcluded(nextExcluded);
    setProblem(null);
    setLive(
      `Request applied: budget ${next.budgetText} ${snapshot?.currency ?? ""}, ${next.useCase}, ${plural(next.lockedSkus.length, "locked part")} and ${plural(next.excludedSkus.length, "excluded part")}. Press Generate recommendations to run the deterministic search.`,
    );
    generateRef.current?.focus();
  }

  function skipAssistant() {
    setLive("Continuing without AI. The deterministic form needs no key.");
    budgetRef.current?.focus();
  }

  async function runExport(format: "json" | "markdown") {
    if (!report) {
      return;
    }
    setExporting(format);
    setExportNotice(null);
    try {
      const filename = await downloadRecommendationExport(report.id, format);
      setExportNotice({ kind: "ok", message: `Export downloaded: ${filename}` });
      setLive(`Export downloaded: ${filename}`);
    } catch (error) {
      const detail =
        error instanceof ApiFailure
          ? `${error.message} (${error.code})`
          : "The export could not be downloaded.";
      setExportNotice({ kind: "error", message: detail });
      setLive(`Export failed: ${detail}`);
    } finally {
      setExporting(null);
    }
  }

  const busy = phase === "generating";
  const budgetInvalid = parseBudgetMinor(budget) === null;
  const budgetErrorVisible = budgetInvalid && budget.trim() !== "";

  return (
    <>
      <SiteHeader current="recommend" wide />
      <main
        className="page page--wide workbench-page"
        id="main"
        tabIndex={-1}
        aria-labelledby="recommend-page-title"
      >
        <h1 className="page-title" id="recommend-page-title">
          Build recommendation
        </h1>
        <p className="lede">
          Enter a total budget and what the machine is for. RigCraft searches only
          your inventory, keeps every part within available stock, and checks every
          candidate with the compatibility rules before it is shown. No conflict is
          recommended; an unknown result is labelled as unverified.
        </p>
        <p className="privacy-note">
          The search runs locally with no hardware catalog. The assistant panel is
          optional, needs your own provider key, and only proposes the request you
          confirm. Searches are deterministic and bounded, and generating a plan
          never reserves stock.
        </p>

        <div
          className="status-line"
          role="status"
          aria-live="polite"
          aria-busy={busy || snapshotBusy}
          data-busy={busy || snapshotBusy}
          tabIndex={-1}
          ref={statusRef}
        >
          <span>{live}</span>
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
            <div className="button-row">
              {retry ? (
                <button type="button" onClick={retry} disabled={busy}>
                  Retry recommendation
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

        <section className="panel" aria-labelledby="request-snapshot-title">
          <h2 className="panel__title" id="request-snapshot-title">
            1. Choose an inventory snapshot
          </h2>
          {historyProblem ? (
            <p className="field__error" role="alert">
              {historyProblem}{" "}
              <button
                type="button"
                className="link-button"
                onClick={() => {
                  setHistoryProblem(null);
                  setSnapshots(null);
                  listSnapshots()
                    .then((list) => {
                      setSnapshots(list);
                      if (list.length > 0) {
                        setSnapshotId(list[0].id);
                      }
                    })
                    .catch(() => {
                      setSnapshots([]);
                      setHistoryProblem("Snapshot history is still unavailable.");
                    });
                }}
              >
                Retry history
              </button>
            </p>
          ) : null}
          {snapshots === null ? (
            <p className="small muted">Loading snapshot history…</p>
          ) : snapshots.length === 0 ? (
            <p>
              No inventory snapshot exists yet.{" "}
              <a href="/inventory">Import a CSV in Inventory</a> before asking for a
              recommendation.
            </p>
          ) : (
            <>
              <div className="field field--inline">
                <label htmlFor="recommend-snapshot">Snapshot</label>
                <select
                  id="recommend-snapshot"
                  value={snapshotId}
                  disabled={busy}
                  onChange={(event) => setSnapshotId(event.target.value)}
                >
                  <option value="">Choose a snapshot</option>
                  {snapshots.map((entry) => (
                    <option key={entry.id} value={entry.id}>
                      Revision {entry.revision} · {entry.sourceFilename} ·{" "}
                      {formatTimestamp(entry.createdAt)} · {entry.recordCount} records ·{" "}
                      {shortId(entry.id)}
                    </option>
                  ))}
                </select>
              </div>
              {snapshot ? (
                <p className="small muted">
                  Snapshot {shortId(snapshot.id)} · revision {snapshot.revision} ·{" "}
                  {snapshot.currency} · source{" "}
                  <span className="mono">{snapshot.source.filename}</span>
                </p>
              ) : null}
              {snapshotProblem ? (
                <div className="alert" role="alert">
                  <h2>Snapshot could not be loaded</h2>
                  <p>{snapshotProblem}</p>
                  <div className="button-row">
                    <button
                      type="button"
                      onClick={() => setSnapshotToken((value) => value + 1)}
                      disabled={snapshotBusy}
                    >
                      Retry snapshot
                    </button>
                  </div>
                </div>
              ) : null}
            </>
          )}
        </section>

        {snapshot ? (
          <div className="workbench">
            <div className="workbench__constraints">
              <AssistantPanel
                snapshot={snapshot}
                disabled={busy}
                onApply={applyAssistant}
                onSkip={skipAssistant}
                announce={setLive}
              />
              <form
                aria-labelledby="request-form-title"
                onSubmit={(event) => {
                  event.preventDefault();
                  void generate();
                }}
              >
                <section className="panel workbench-panel" aria-labelledby="request-form-title">
                  <p className="caps-label" aria-hidden="true">
                    Constraints
                  </p>
                  <h2 className="panel__title" id="request-form-title">
                    3. Budget and constraints
                  </h2>
                  <div className="field-grid">
                    <div className="field">
                      <label htmlFor="recommend-budget">
                        Total budget ({snapshot.currency})
                      </label>
                      <input
                        id="recommend-budget"
                        ref={budgetRef}
                        inputMode="decimal"
                        value={budget}
                        placeholder="1200.00"
                        aria-invalid={budgetErrorVisible || undefined}
                        aria-describedby={
                          budgetErrorVisible
                            ? "recommend-budget-hint recommend-budget-error"
                            : "recommend-budget-hint"
                        }
                        disabled={busy}
                        onChange={(event) => setBudget(event.target.value)}
                      />
                      <p className="field__hint" id="recommend-budget-hint">
                        One currency per batch: {snapshot.currency}. A positive amount
                        with at most two decimals; a comma or a space may group
                        thousands, as in 1,800 or 1,800.50.
                      </p>
                      {budgetErrorVisible ? (
                        <p className="field__error" id="recommend-budget-error">
                          <span className="field__error__title">Enter a budget</span> Use
                          a positive amount with at most two decimals, for example 1200
                          or 1,800.50. Code: <span className="mono">budget_invalid</span>
                        </p>
                      ) : null}
                    </div>
                    <div className="field">
                      <label htmlFor="recommend-use-case">Primary use</label>
                      <select
                        id="recommend-use-case"
                        value={useCase}
                        disabled={busy}
                        onChange={(event) => setUseCase(event.target.value)}
                      >
                        {USE_CASES.map((entry) => (
                          <option key={entry.value} value={entry.value}>
                            {entry.label}
                          </option>
                        ))}
                      </select>
                      <p className="field__hint">
                        Gaming requires a discrete graphics card in every option.
                        Office accepts a CPU with sourced integrated graphics.
                        Creation does not require a discrete card.
                      </p>
                    </div>
                  </div>
                  <p className="small muted">
                    Inventory-only mode is on. A part is never added from outside this
                    snapshot, and a locked part is treated as already owned: zero
                    incremental price, no stock consumption.
                  </p>

                  <fieldset className="part-group">
                    <legend>Already owned parts (locked)</legend>
                    <p className="small muted">
                      Optional. A locked part must exist in this snapshot; it appears in
                      every build at zero cost.
                    </p>
                    <details className="details-block">
                      <summary>
                        Choose owned parts ({plural(lockedList.length, "part")} selected)
                      </summary>
                      <ul className="part-list">
                      {snapshot.records.map((record) => (
                        <li className="part-option part-option--none" key={`lock-${record.id}`}>
                          <input
                            type="checkbox"
                            id={`lock-${record.id}`}
                            checked={locked[record.id] === true}
                            disabled={busy}
                            onChange={(event) => toggleLocked(record, event.target.checked)}
                          />
                          <label htmlFor={`lock-${record.id}`}>
                            Lock <strong>{record.modelRaw}</strong>{" "}
                            <span className="small muted">
                              <span className="mono">{record.sku}</span> ·{" "}
                              {record.category} · {record.quantity} in stock ·{" "}
                              {formatMoney(record.priceMinor, record.currency)}
                            </span>
                          </label>
                        </li>
                      ))}
                      </ul>
                    </details>
                  </fieldset>

                  <fieldset className="part-group">
                    <legend>Parts to exclude</legend>
                    <p className="small muted">
                      Optional. Excluded parts are never chosen and are listed in the
                      report with the reason.
                    </p>
                    <details className="details-block">
                      <summary>
                        Choose excluded parts ({plural(excludedList.length, "part")} selected)
                      </summary>
                      <ul className="part-list">
                      {snapshot.records.map((record) => (
                        <li className="part-option part-option--none" key={`exclude-${record.id}`}>
                          <input
                            type="checkbox"
                            id={`exclude-${record.id}`}
                            checked={excluded[record.id] === true}
                            disabled={busy}
                            onChange={(event) => toggleExcluded(record, event.target.checked)}
                          />
                          <label htmlFor={`exclude-${record.id}`}>
                            Exclude <strong>{record.modelRaw}</strong>{" "}
                            <span className="small muted">
                              <span className="mono">{record.sku}</span> · {record.category}
                            </span>
                          </label>
                        </li>
                      ))}
                      </ul>
                    </details>
                  </fieldset>

                  <div className="button-row">
                    <button
                      type="submit"
                      ref={generateRef}
                      disabled={busy || snapshotBusy || budgetInvalid}
                      aria-describedby="recommend-generate-hint"
                    >
                      {busy ? "Searching…" : "Generate recommendations"}
                    </button>
                    <span className="small muted">
                      {lockedList.length} locked · {excludedList.length} excluded
                    </span>
                  </div>
                  <p className="field__hint" id="recommend-generate-hint">
                    Generate stays disabled until the budget is a positive amount with
                    at most two decimals.
                  </p>
                </section>
              </form>
            </div>
            <div className="workbench__options">
              <RecommendOptions
                report={report}
                staleReport={staleReport}
                headingRef={resultsRef}
                exporting={exporting}
                exportNotice={exportNotice}
                onExport={(format) => void runExport(format)}
              />
            </div>
            <div className="workbench__verdict">
              <RecommendVerdict report={report} />
            </div>
          </div>
        ) : null}
      </main>
    </>
  );
}

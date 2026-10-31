import { useEffect, useRef, useState } from "react";
import type {
  Candidate,
  RecommendationReport,
  RecordItem,
  Snapshot,
  SnapshotSummary,
} from "../types";
import {
  ApiFailure,
  downloadRecommendationExport,
  getSnapshot,
  listSnapshots,
  recommendBuild,
} from "../api";
import { PART_CATEGORIES, verdictMeta } from "../compatibility";
import { formatDate, formatMoney, shortId } from "../inventory";
import {
  USE_CASES,
  bindingLabel,
  candidatePart,
  formatScore,
  parseBudgetMinor,
  requestSignature,
  searchSummary,
  selectedRequest,
} from "../recommend";
import { AssistantPanel } from "./AssistantPanel";
import type { AppliedProposal } from "./AssistantPanel";
import { CandidateCard } from "./CandidateCard";
import { SiteHeader } from "./SiteHeader";

type Phase = "idle" | "generating" | "done" | "failed";

interface Problem {
  title: string;
  message: string;
  code?: string;
}

interface ExportNotice {
  kind: "ok" | "error";
  message: string;
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
            `Ready. ${list.length} snapshot${list.length === 1 ? "" : "s"} available; the newest is revision ${list[0].revision}.`,
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
          `Snapshot revision ${next.revision} loaded: ${next.records.length} record${
            next.records.length === 1 ? "" : "s"
          }, currency ${next.currency}.`,
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
          `Search limit reached. ${result.candidates.length} best-effort option(s) shown; no global optimum is claimed.`,
        );
      } else {
        setLive(
          `Recommendation complete: ${result.candidates.length} option${
            result.candidates.length === 1 ? "" : "s"
          } within ${formatMoney(result.budgetMinor, result.currency)}.`,
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
      `Request applied: budget ${next.budgetText} ${snapshot?.currency ?? ""}, ${next.useCase}, ${next.lockedSkus.length} locked and ${next.excludedSkus.length} excluded. Press Generate recommendations to run the deterministic search.`,
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
  const current = report?.candidates ?? [];

  return (
    <>
      <SiteHeader current="recommend" />
      <main className="page" id="main" tabIndex={-1}>
        <h1 className="page-title">Build recommendation</h1>
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
                      {formatDate(entry.createdAt)} · {entry.recordCount} records ·{" "}
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
          <>
            <AssistantPanel
              snapshot={snapshot}
              disabled={busy}
              onApply={applyAssistant}
              onSkip={skipAssistant}
              announce={setLive}
            />
            <form
              onSubmit={(event) => {
                event.preventDefault();
                void generate();
              }}
            >
              <section className="panel" aria-labelledby="request-form-title">
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
                      aria-invalid={problem?.code === "budget_invalid" || undefined}
                      aria-describedby="recommend-budget-hint"
                      disabled={busy}
                      onChange={(event) => setBudget(event.target.value)}
                    />
                    <p className="field__hint" id="recommend-budget-hint">
                      One currency per batch: {snapshot.currency}. At most two decimals.
                    </p>
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
                      Weights: gaming favours performance, office favours cost, creation
                      balances performance and upgrade room.
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
                      Choose owned parts ({lockedList.length} selected)
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
                      Choose excluded parts ({excludedList.length} selected)
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
                  <button type="submit" ref={generateRef} disabled={busy || snapshotBusy}>
                    {busy ? "Searching…" : "Generate recommendations"}
                  </button>
                  <span className="small muted">
                    {lockedList.length} locked · {excludedList.length} excluded
                  </span>
                </div>
              </section>
            </form>
          </>
        ) : null}

        {report ? (
          <section
            className={`panel result-panel${staleReport ? " result-panel--stale" : ""}`}
            aria-labelledby="recommend-result-title"
          >
            <h2 className="panel__title" id="recommend-result-title" tabIndex={-1} ref={resultsRef}>
              Recommended builds
            </h2>
            {staleReport ? (
              <div className="stale-note" role="status">
                <strong>These results are stale.</strong> The budget, use case or
                constraints changed after this report was generated. Generate again
                before treating them as current.
              </div>
            ) : null}
            {report.stale ? (
              <div className="stale-note" role="status">
                <strong>The inventory moved.</strong>{" "}
                {report.staleReason ??
                  "The snapshot revision changed after this report was created."}{" "}
                Generate again before treating it as current.
              </div>
            ) : null}
            <dl className="summary-grid result-summary">
              <div>
                <dt>Options</dt>
                <dd>{report.candidates.length}</dd>
              </div>
              <div>
                <dt>Budget</dt>
                <dd>{formatMoney(report.budgetMinor, report.currency)}</dd>
              </div>
              <div>
                <dt>Valid builds</dt>
                <dd>{report.search.validBuilds}</dd>
              </div>
              <div>
                <dt>Status</dt>
                <dd>
                  {report.status === "ok"
                    ? "Solved"
                    : report.status === "search_limited"
                      ? "Limit reached"
                      : "No solution"}
                </dd>
              </div>
            </dl>
            <p className="small muted">
              Snapshot revision {report.snapshotRevision} · scoring{" "}
              {report.scoringVersion} · rules {report.ruleSetVersion} ·{" "}
              {searchSummary(report.search)}
            </p>

            {report.search.limited ? (
              <div className="diagnostic-block diagnostic-block--unknown" role="status">
                <h3>Search limit reached</h3>
                <p>
                  The search stopped at {report.search.limit} nodes after checking{" "}
                  {report.search.leavesEvaluated} combinations. These options are the
                  best found so far; no global optimum is claimed. Narrow the request
                  to let the search finish.
                </p>
              </div>
            ) : null}

            {report.noSolution ? (
              <div className="diagnostic-block diagnostic-block--conflict" role="group" aria-labelledby="no-solution-title">
                <h3 id="no-solution-title">{bindingLabel(report.noSolution)}</h3>
                <p>
                  <strong>{report.noSolution.headline}</strong>
                </p>
                <p>{report.noSolution.explanation}</p>
                {report.noSolution.blockers.length > 0 ? (
                  <>
                    <h4>What blocked it</h4>
                    <ul className="missing-list">
                      {report.noSolution.blockers.map((blocker, index) => (
                        <li key={`${blocker.code}-${index}`}>{blocker.detail}</li>
                      ))}
                    </ul>
                  </>
                ) : null}
                <h4>What can be adjusted</h4>
                <ul className="missing-list">
                  {report.noSolution.adjustable.map((item) => (
                    <li key={item}>{item}</li>
                  ))}
                </ul>
              </div>
            ) : null}

            {current.map((candidate) => (
              <CandidateCard
                candidate={candidate}
                currency={report.currency}
                key={candidate.checkReportId}
              />
            ))}

            {current.length > 0 ? (
              <div className="comparison-block">
                <h3 className="evidence-title">Compare options</h3>
                <div className="table-wrap">
                  <table className="responsive-table compare-table">
                    <caption>
                      Side-by-side differences across the recommended options
                    </caption>
                    <thead>
                      <tr>
                        <th scope="col">Attribute</th>
                        {current.map((candidate) => (
                          <th scope="col" key={candidate.rank}>
                            {candidate.rank}. {candidate.label}
                          </th>
                        ))}
                      </tr>
                    </thead>
                    <tbody>
                      {[
                        {
                          label: "Total",
                          value: (candidate: Candidate) =>
                            formatMoney(candidate.totalMinor, report.currency),
                        },
                        {
                          label: "Budget delta",
                          value: (candidate: Candidate) =>
                            formatMoney(candidate.budgetDeltaMinor, report.currency),
                        },
                        {
                          label: "Compatibility",
                          value: (candidate: Candidate) =>
                            `${verdictMeta(candidate.overallStatus).symbol} ${
                              verdictMeta(candidate.overallStatus).label
                            }`,
                        },
                        {
                          label: "Completeness",
                          value: (candidate: Candidate) =>
                            candidate.completeness === "complete" ? "Complete" : "Incomplete",
                        },
                        {
                          label: "Stock units",
                          value: (candidate: Candidate) => String(candidate.unitsFromStock),
                        },
                        {
                          label: "Owned units",
                          value: (candidate: Candidate) => String(candidate.ownedUnits),
                        },
                        {
                          label: "Score",
                          value: (candidate: Candidate) =>
                            `${formatScore(candidate.scores.total)} (${candidate.scores.confidence})`,
                        },
                        ...PART_CATEGORIES.map((category) => ({
                          label: category.label,
                          value: (candidate: Candidate) => {
                            const part = candidatePart(candidate, category.key);
                            return part
                              ? `${part.model} · ${formatMoney(part.lineTotalMinor, report.currency)}`
                              : "—";
                          },
                        })),
                        {
                          label: "Check report",
                          value: (candidate: Candidate) => candidate.checkReportId,
                        },
                      ].map((row) => (
                        <tr key={row.label}>
                          <th scope="row" data-label="Attribute">
                            {row.label}
                          </th>
                          {current.map((candidate) => (
                            <td data-label={`${candidate.rank}. ${candidate.label}`} key={candidate.rank}>
                              {row.value(candidate)}
                            </td>
                          ))}
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </div>
            ) : null}

            {current.length > 0 ? (
              <div className="button-row">
                <button
                  type="button"
                  onClick={() => void runExport("json")}
                  disabled={exporting !== null}
                >
                  {exporting === "json" ? "Preparing JSON…" : "Download JSON export"}
                </button>
                <button
                  type="button"
                  className="button--secondary"
                  onClick={() => void runExport("markdown")}
                  disabled={exporting !== null}
                >
                  {exporting === "markdown"
                    ? "Preparing Markdown…"
                    : "Download Markdown plan"}
                </button>
              </div>
            ) : null}
            {exportNotice ? (
              <p
                className={exportNotice.kind === "error" ? "field__error" : "small muted"}
                role={exportNotice.kind === "error" ? "alert" : "status"}
              >
                {exportNotice.message}
              </p>
            ) : null}

            {report.exclusions.length > 0 ? (
              <details className="details-block">
                <summary>Parts excluded from consideration</summary>
                <ul className="diagnostic-list">
                  {report.exclusions.map((exclusion) => (
                    <li key={`${exclusion.sku}-${exclusion.reason}`}>
                      <strong>
                        {exclusion.sku} · {exclusion.category}
                      </strong>{" "}
                      <span className="mono small">{exclusion.reason}</span>
                      <p>{exclusion.detail}</p>
                    </li>
                  ))}
                </ul>
              </details>
            ) : null}
            <p className="privacy-note">
              Prices, stock and verdicts come from snapshot revision{" "}
              {report.snapshotRevision}. Re-generate after an inventory or rule change;
              an export keeps the versions it was produced from.
            </p>
          </section>
        ) : snapshot ? (
          <section className="panel" aria-labelledby="no-report-title">
            <h2 className="panel__title" id="no-report-title">
              4. Read and compare options
            </h2>
            <p>
              Enter a budget and generate. Each option states why it was chosen, its
              cost and stock use, every rule verdict, what was excluded, and exactly
              which fact to confirm when evidence is missing.
            </p>
          </section>
        ) : null}
      </main>
    </>
  );
}

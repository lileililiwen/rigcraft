import { useEffect, useRef, useState } from "react";
import type {
  CheckReport,
  RecordItem,
  RuleResult,
  Snapshot,
  SnapshotSummary,
} from "../types";
import { ApiFailure, checkBuild, getSnapshot, listSnapshots } from "../api";
import {
  PART_CATEGORIES,
  buildSelection,
  categoryLabel,
  completionLabel,
  defaultQuantity,
  emptySelection,
  isRecordSelected,
  reportIsStale,
  reportSummary,
  selectionErrors,
  verdictMeta,
} from "../compatibility";
import type { SelectionState } from "../compatibility";
import { formatTimestamp, formatMoney, shortId, sourceLabel } from "../inventory";
import { ProvenanceList, SpecList } from "./Badges";
import { SiteHeader } from "./SiteHeader";

type Phase = "idle" | "checking" | "checked" | "failed";

interface Problem {
  title: string;
  message: string;
  code?: string;
}

function describeFailure(error: unknown): Problem {
  if (error instanceof ApiFailure) {
    return {
      title: error.status === 422 ? "The check could not run" : "Request failed",
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

function VerdictBadge({ report }: { report: CheckReport }) {
  const meta = verdictMeta(report.overallStatus);
  return (
    <span className={`badge badge--${meta.className} badge--verdict`}>
      <span className="badge__symbol" aria-hidden="true">
        {meta.symbol}
      </span>
      {meta.label}
    </span>
  );
}

function RuleBadge({ rule }: { rule: RuleResult }) {
  const meta = verdictMeta(rule.status);
  return (
    <span className={`badge badge--${meta.className}`}>
      <span className="badge__symbol" aria-hidden="true">
        {meta.symbol}
      </span>
      {meta.label}
    </span>
  );
}

function RuleEvidence({ rule }: { rule: RuleResult }) {
  return (
    <details className="details-block rule-evidence">
      <summary>Rule, inputs and provenance</summary>
      <p className="small muted">{rule.method}</p>
      <p className="small mono">
        {rule.ruleId} · {rule.ruleVersion}
      </p>
      <dl className="evidence-list">
        {rule.evidence.map((entry, index) => (
          <div key={`${entry.part.recordId}-${entry.field}-${index}`}>
            <dt>
              <span className="mono">{entry.part.sku}</span> · {entry.field}
            </dt>
            <dd>
              {entry.value !== null ? (
                <span className="mono">{entry.value}</span>
              ) : (
                <span className="badge badge--unknown">
                  <span className="badge__symbol" aria-hidden="true">
                    ?
                  </span>
                  Not provided
                </span>
              )}
              {entry.provenance ? (
                <span className="small muted">
                  {" "}
                  · {sourceLabel(entry.provenance.source)} ·{" "}
                  <span className="tag">{entry.provenance.reference}</span> ·{" "}
                  <time dateTime={entry.provenance.observedAt}>
                    {formatTimestamp(entry.provenance.observedAt)}
                  </time>
                </span>
              ) : (
                <span className="small muted"> · No source recorded</span>
              )}
            </dd>
          </div>
        ))}
      </dl>
    </details>
  );
}

export function CheckPage() {
  const [snapshots, setSnapshots] = useState<SnapshotSummary[] | null>(null);
  const [historyProblem, setHistoryProblem] = useState<string | null>(null);
  const [snapshotId, setSnapshotId] = useState("");
  const [snapshotToken, setSnapshotToken] = useState(0);
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [snapshotBusy, setSnapshotBusy] = useState(false);
  const [snapshotProblem, setSnapshotProblem] = useState<string | null>(null);
  const [selection, setSelection] = useState<SelectionState>(emptySelection);
  const [phase, setPhase] = useState<Phase>("idle");
  const [report, setReport] = useState<CheckReport | null>(null);
  const [problem, setProblem] = useState<Problem | null>(null);
  const [retry, setRetry] = useState<(() => void) | null>(null);
  const [live, setLive] = useState("Choose a snapshot and the parts to check.");
  const statusRef = useRef<HTMLDivElement>(null);
  const errorRef = useRef<HTMLDivElement>(null);

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
          error instanceof ApiFailure
            ? error.message
            : "Snapshot history is unavailable.",
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
        setSelection(emptySelection());
        setSnapshotBusy(false);
        setLive(
          `Snapshot revision ${next.revision} loaded: ${next.records.length} record${next.records.length === 1 ? "" : "s"}.`,
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
    if (phase === "checking") {
      statusRef.current?.focus();
    }
  }, [phase]);

  useEffect(() => {
    if (phase === "checked" && report) {
      document.getElementById("result-title")?.focus();
    }
  }, [phase, report]);

  useEffect(() => {
    if (problem) {
      errorRef.current?.focus();
    }
  }, [problem]);

  function noteEdit() {
    if (report) {
      setLive(
        "The selected parts changed. The previous verdict is stale until the check runs again.",
      );
    }
  }

  function selectSingle(category: string, recordId: string) {
    noteEdit();
    setSelection((current) => {
      const nextSingle = { ...current.single, [category]: recordId };
      const nextQuantities = { ...current.quantities };
      if (recordId && !nextQuantities[recordId] && snapshot) {
        const record = snapshot.records.find((entry) => entry.id === recordId);
        if (record) {
          nextQuantities[recordId] = defaultQuantity();
        }
      }
      return { ...current, single: nextSingle, quantities: nextQuantities };
    });
  }

  function toggleMulti(record: RecordItem, included: boolean) {
    noteEdit();
    setSelection((current) => ({
      ...current,
      multi: { ...current.multi, [record.id]: included },
      quantities: {
        ...current.quantities,
        [record.id]:
          current.quantities[record.id] ?? defaultQuantity(),
      },
    }));
  }

  function changeQuantity(recordId: string, value: string) {
    noteEdit();
    setSelection((current) => ({
      ...current,
      quantities: { ...current.quantities, [recordId]: value },
    }));
  }

  async function runCheck() {
    if (!snapshot) {
      return;
    }
    const errors = selectionErrors(snapshot, selection);
    if (errors.length > 0) {
      setProblem({
        title: "The check needs a different selection",
        message: errors.join(" "),
        code: "selection_incomplete",
      });
      setRetry(null);
      return;
    }
    setPhase("checking");
    setProblem(null);
    setRetry(null);
    setLive("Checking the selected build against the snapshot facts…");
    try {
      const result = await checkBuild({
        snapshotId: snapshot.id,
        selection: buildSelection(snapshot, selection),
      });
      setReport(result);
      setPhase("checked");
      setLive(
        `Check complete: ${verdictMeta(result.overallStatus).label}. ${reportSummary(result)}.`,
      );
    } catch (error) {
      const next = describeFailure(error);
      setProblem(next);
      setPhase("failed");
      setLive("The check failed. Your selection is kept; retry when the service is available.");
      setRetry(() => () => void runCheck());
    }
  }

  const currentSelection = snapshot ? buildSelection(snapshot, selection) : [];
  const localStale = reportIsStale(report, snapshotId, currentSelection);
  const stale = Boolean(report && (report.stale || localStale));
  const staleReason = localStale
    ? report && report.snapshotId !== snapshotId
      ? "The selected inventory snapshot changed."
      : "The selected parts or quantities changed."
    : report?.staleReason ?? null;
  const busy = phase === "checking";
  const selectedCount = snapshot
    ? currentSelection.reduce((total, item) => total + item.quantity, 0)
    : 0;

  return (
    <>
      <SiteHeader current="compatibility" />
      <main className="page" id="main" tabIndex={-1}>
        <h1 className="page-title">Compatibility check</h1>
        <p className="lede">
          Check a selected build against the facts in one inventory snapshot. Every
          rule reports compatible, incompatible or unknown with the values and
          provenance it used. A missing fact stays unknown; it is never guessed.
        </p>
        <p className="privacy-note">
          This check runs only against your local inventory. No hardware catalog or
          AI provider is consulted, and the selection is not saved as stock.
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
                  Retry check
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

        <section className="panel" aria-labelledby="snapshot-choice-title">
          <h2 className="panel__title" id="snapshot-choice-title">
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
              <a href="/inventory">Import a CSV in Inventory</a> before checking a
              build.
            </p>
          ) : (
            <>
              <div className="field field--inline">
                <label htmlFor="check-snapshot">Snapshot</label>
                <select
                  id="check-snapshot"
                  value={snapshotId}
                  disabled={busy}
                  onChange={(event) => {
                    noteEdit();
                    setSnapshotId(event.target.value);
                  }}
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
          <form
            onSubmit={(event) => {
              event.preventDefault();
              void runCheck();
            }}
          >
            <section className="panel" aria-labelledby="parts-title">
              <h2 className="panel__title" id="parts-title">
                2. Select parts
              </h2>
              <p className="small muted">
                Choose at most one CPU, motherboard, graphics card, power supply,
                case and cooler. Memory and storage can include several lines.
                Quantity defaults to one and is visible next to each part.
              </p>
              {PART_CATEGORIES.map((category) => {
                const records = snapshot.records.filter(
                  (record) => record.category === category.key,
                );
                const singleValue = selection.single[category.key] ?? "";
                return (
                  <fieldset className="part-group" key={category.key}>
                    <legend>{category.label}</legend>
                    {records.length === 0 ? (
                      <p className="small muted">
                        No {category.label.toLowerCase()} is recorded in this
                        snapshot.
                      </p>
                    ) : (
                      <ul className="part-list">
                        {!category.multi ? (
                          <li className="part-option part-option--none">
                            <input
                              type="radio"
                              id={`part-${category.key}-none`}
                              name={`part-${category.key}`}
                              value=""
                              checked={singleValue === ""}
                              disabled={busy}
                              onChange={() => selectSingle(category.key, "")}
                            />
                            <label htmlFor={`part-${category.key}-none`}>
                              No {category.label.toLowerCase()} selected
                            </label>
                          </li>
                        ) : null}
                        {records.map((record) => {
                          const selected = isRecordSelected(selection, record);
                          const inputId = `part-${record.id}`;
                          const inputType = category.multi ? "checkbox" : "radio";
                          const unavailable = record.quantity <= 0;
                          const quantity = selection.quantities[record.id] ?? "1";
                          return (
                            <li className="part-option" key={record.id}>
                              <div className="part-option__head">
                                <input
                                  type={inputType}
                                  id={inputId}
                                  name={`part-${category.key}`}
                                  value={record.id}
                                  checked={selected}
                                  disabled={busy || unavailable}
                                  aria-describedby={`${inputId}-meta`}
                                  onChange={(event) => {
                                    if (category.multi) {
                                      toggleMulti(record, event.target.checked);
                                    } else {
                                      selectSingle(category.key, record.id);
                                    }
                                  }}
                                />
                                <label htmlFor={inputId} className="part-option__label">
                                  <span className="part-option__model">
                                    {record.modelRaw}
                                  </span>
                                  <span className="small muted" id={`${inputId}-meta`}>
                                    <span className="mono">{record.sku}</span> ·{" "}
                                    {record.quantity} in stock ·{" "}
                                    {formatMoney(record.priceMinor, record.currency)}
                                    {unavailable ? " · unavailable" : ""}
                                  </span>
                                </label>
                              </div>
                              <div className="part-option__qty">
                                <label htmlFor={`qty-${record.id}`}>
                                  Quantity for {record.sku}
                                </label>
                                <input
                                  id={`qty-${record.id}`}
                                  inputMode="numeric"
                                  value={quantity}
                                  disabled={!selected || busy || unavailable}
                                  aria-describedby={`qty-${record.id}-hint`}
                                  onChange={(event) =>
                                    changeQuantity(record.id, event.target.value)
                                  }
                                />
                                <span className="field__hint" id={`qty-${record.id}-hint`}>
                                  Whole units, at most {record.quantity}.
                                </span>
                              </div>
                              <details className="details-block">
                                <summary>Facts and provenance for {record.sku}</summary>
                                <SpecList specs={record.specs} />
                                <ProvenanceList
                                  provenance={record.provenance}
                                  match={record.match}
                                />
                              </details>
                            </li>
                          );
                        })}
                      </ul>
                    )}
                  </fieldset>
                );
              })}
              <div className="button-row">
                <button type="submit" disabled={busy || snapshotBusy}>
                  {busy ? "Checking…" : "Check compatibility"}
                </button>
                <span className="small muted" aria-live="off">
                  {selectedCount} unit{selectedCount === 1 ? "" : "s"} selected
                </span>
              </div>
            </section>
          </form>
        ) : null}

        {report ? (
          <section
            className={`panel result-panel${stale ? " result-panel--stale" : ""}`}
            aria-labelledby="result-title"
          >
            <h2 className="panel__title" id="result-title" tabIndex={-1}>
              Compatibility result
            </h2>
            {stale ? (
              <div className="stale-note" role="status">
                <strong>This verdict is stale.</strong>{" "}
                {staleReason ??
                  "The inventory or selection changed after the check."}{" "}
                Run the check again before treating it as current.
              </div>
            ) : null}
            <div className="verdict-line">
              <VerdictBadge report={report} />
              <span className="verdict-completeness">
                {completionLabel(report)}
              </span>
              <span className="small muted">
                Checked{" "}
                <time dateTime={report.checkedAt}>
                  {formatTimestamp(report.checkedAt)}
                </time>{" "}
                · snapshot revision {report.snapshotRevision}
                {stale &&
                report.currentSnapshotRevision != null &&
                report.currentSnapshotRevision !== report.snapshotRevision
                  ? ` (now revision ${report.currentSnapshotRevision})`
                  : ""}{" "}
                · rule set {report.ruleSetVersion}
              </span>
            </div>
            <dl className="summary-grid result-summary">
              <div>
                <dt>Rules passed</dt>
                <dd>{report.summary.compatible}</dd>
              </div>
              <div>
                <dt>Conflicts</dt>
                <dd>{report.summary.incompatible}</dd>
              </div>
              <div>
                <dt>Unknown</dt>
                <dd>{report.summary.unknown}</dd>
              </div>
            </dl>

            {report.completeness.status === "incomplete" ? (
              <div className="diagnostic-block diagnostic-block--unknown">
                <h3>Incomplete build</h3>
                <ul className="missing-list">
                  {report.completeness.missing.map((entry) => (
                    <li key={entry.category}>
                      <strong>{categoryLabel(entry.category)}</strong>: {entry.reason}.{" "}
                      <span>{entry.followUp}</span>
                    </li>
                  ))}
                </ul>
              </div>
            ) : null}

            <div className="diagnostic-block diagnostic-block--conflict">
              <h3>Conflict diagnostics</h3>
              {report.summary.incompatible === 0 ? (
                <p>No definite conflict was found among the rules that could run.</p>
              ) : (
                <ul className="diagnostic-list">
                  {report.rules
                    .filter((rule) => rule.status === "incompatible")
                    .map((rule) => (
                      <li key={rule.ruleId}>
                        <strong>{rule.title}</strong>{" "}
                        <span className="mono small">{rule.ruleId}</span>
                        <p>{rule.explanation}</p>
                      </li>
                    ))}
                </ul>
              )}
            </div>

            <div className="diagnostic-block diagnostic-block--unknown">
              <h3>Facts to confirm</h3>
              {report.rules.filter((rule) => rule.status === "unknown").length ===
              0 ? (
                <p>No rule is blocked by missing evidence.</p>
              ) : (
                <ul className="diagnostic-list">
                  {report.rules
                    .filter((rule) => rule.status === "unknown")
                    .map((rule) => (
                      <li key={rule.ruleId}>
                        <strong>{rule.title}</strong>{" "}
                        <span className="mono small">{rule.ruleId}</span>
                        <p>{rule.followUp ?? rule.explanation}</p>
                      </li>
                    ))}
                </ul>
              )}
            </div>

            <h3 className="evidence-title">Rule evidence</h3>
            <div className="table-wrap">
              <table className="responsive-table">
                <caption>
                  Every rule, its verdict, explanation and the inputs with provenance
                </caption>
                <thead>
                  <tr>
                    <th scope="col">Rule</th>
                    <th scope="col">Verdict</th>
                    <th scope="col">Explanation</th>
                    <th scope="col">Evidence</th>
                  </tr>
                </thead>
                <tbody>
                  {report.rules.map((rule) => (
                    <tr key={rule.ruleId}>
                      <td data-label="Rule">
                        {rule.title}
                        <span className="small muted mono"> · {rule.ruleId}</span>
                      </td>
                      <td data-label="Verdict">
                        <RuleBadge rule={rule} />
                      </td>
                      <td data-label="Explanation">
                        {rule.explanation}
                        {rule.followUp ? (
                          <span className="follow-up"> Confirm: {rule.followUp}</span>
                        ) : null}
                      </td>
                      <td data-label="Evidence">
                        <RuleEvidence rule={rule} />
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            <p className="privacy-note">
              Verdicts cite each fact's recorded provenance. A declared catalog source
              is shown as catalog-declared; RigCraft does not query or verify it, and a
              missing fact stays unknown.
            </p>
            <div className="button-row">
              <button
                type="button"
                onClick={() => void runCheck()}
                disabled={busy || snapshotBusy}
              >
                {busy ? "Checking…" : "Re-check this selection"}
              </button>
            </div>
          </section>
        ) : snapshot ? (
          <section className="panel" aria-labelledby="no-check-title">
            <h2 className="panel__title" id="no-check-title">
              3. Read the verdict
            </h2>
            <p>
              Select the parts above and run the check. The result names every rule,
              the parts and fields it compared, the source of each value, and the exact
              fact to confirm when evidence is missing.
            </p>
          </section>
        ) : null}
      </main>
    </>
  );
}

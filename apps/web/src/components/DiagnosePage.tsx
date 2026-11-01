import { useEffect, useRef, useState } from "react";
import type {
  BuildPartInput,
  ParsedLine,
  ParsedList,
  DiagnosisReport,
  Snapshot,
  SnapshotSummary,
} from "../types";
import { ApiFailure, diagnoseBuild, getSnapshot, listSnapshots, parseDiagnosisList } from "../api";
import { PART_CATEGORIES, categoryLabel } from "../compatibility";
import {
  MATCH_META,
  SKIP,
  initialChoices,
  lineDecisionLabel,
  reviewProblems,
  reviewedParts,
} from "../diagnosis";
import { formatTimestamp, formatMoney, shortId } from "../inventory";
import { plural } from "../plural";
import { DiagnosisReportPanel } from "./DiagnosisReportPanel";
import { SiteHeader } from "./SiteHeader";

type Phase = "idle" | "review" | "diagnosing" | "done" | "failed";

interface Problem {
  title: string;
  message: string;
  code?: string;
}

interface ManualPart {
  key: string;
  part: BuildPartInput;
  label: string;
}

function describeFailure(error: unknown): Problem {
  if (error instanceof ApiFailure) {
    return {
      title: error.status === 422 ? "The diagnosis could not run" : "Request failed",
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

export function DiagnosePage() {
  const [snapshots, setSnapshots] = useState<SnapshotSummary[] | null>(null);
  const [historyProblem, setHistoryProblem] = useState<string | null>(null);
  const [snapshotId, setSnapshotId] = useState("");
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [snapshotBusy, setSnapshotBusy] = useState(false);
  const [snapshotProblem, setSnapshotProblem] = useState<string | null>(null);
  const [currency, setCurrency] = useState("");
  const [text, setText] = useState("");
  const [list, setList] = useState<ParsedList | null>(null);
  const [choices, setChoices] = useState<Record<number, string>>({});
  const [matching, setMatching] = useState(false);
  const [manual, setManual] = useState<ManualPart[]>([]);
  const [manualCategory, setManualCategory] = useState("cpu");
  const [manualModel, setManualModel] = useState("");
  const [manualQuantity, setManualQuantity] = useState("1");
  const [manualSpecs, setManualSpecs] = useState("");
  const [manualError, setManualError] = useState<string | null>(null);
  const [phase, setPhase] = useState<Phase>("idle");
  const [report, setReport] = useState<DiagnosisReport | null>(null);
  const [problem, setProblem] = useState<Problem | null>(null);
  const [live, setLive] = useState("Choose a snapshot, then paste your parts list or add parts by hand.");
  const [manualKey, setManualKey] = useState(0);
  const statusRef = useRef<HTMLDivElement>(null);
  const reviewRef = useRef<HTMLHeadingElement>(null);
  const errorRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let cancelled = false;
    listSnapshots()
      .then((entries) => {
        if (cancelled) {
          return;
        }
        setSnapshots(entries);
        setHistoryProblem(null);
        if (entries.length > 0) {
          setSnapshotId(entries[0].id);
          setLive(
            `Ready. ${plural(entries.length, "snapshot")} available; the newest is revision ${entries[0].revision}.`,
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
    setList(null);
    setChoices({});
    setReport(null);
    setPhase("idle");
    setLive("Loading the inventory snapshot…");
    getSnapshot(snapshotId)
      .then((next) => {
        if (cancelled) {
          return;
        }
        setSnapshot(next);
        setCurrency(next.currency);
        setSnapshotBusy(false);
        setLive(
          `Snapshot revision ${next.revision} loaded: ${plural(
            next.records.length,
            "record",
          )}. Matching compares pasted models with these records only.`,
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
  }, [snapshotId]);

  useEffect(() => {
    if (list) {
      reviewRef.current?.focus();
    }
  }, [list]);

  useEffect(() => {
    if (phase === "done" && report) {
      document.getElementById("diagnosis-result-title")?.focus();
    }
  }, [phase, report]);

  useEffect(() => {
    if (problem) {
      errorRef.current?.focus();
    }
  }, [problem]);

  async function runMatch() {
    if (text.trim().length === 0) {
      setProblem({
        title: "Nothing to match",
        message: "Paste at least one part, one per line, or add parts by hand below.",
        code: "text_required",
      });
      return;
    }
    setMatching(true);
    setProblem(null);
    setReport(null);
    setPhase("review");
    setLive("Matching the pasted lines against this snapshot…");
    try {
      const parsed = await parseDiagnosisList({
        snapshotId,
        currency,
        text,
      });
      const counts = parsed.lines.reduce<Record<string, number>>((totals, line) => {
        totals[line.status] = (totals[line.status] ?? 0) + 1;
        return totals;
      }, {});
      setList(parsed);
      setChoices(initialChoices(parsed));
      setLive(
        `Match review ready: ${plural(parsed.lineCount, "line")}. ${
          counts.matched ?? 0
        } matched, ${counts.ambiguous ?? 0} ambiguous, ${counts.unmatched ?? 0} without a match, ${
          counts.invalid ?? 0
        } needing a fix. Resolve every line before diagnosing.`,
      );
    } catch (error) {
      setProblem(describeFailure(error));
      setList(null);
      setPhase("idle");
      setLive("Matching failed. Your pasted list is kept; retry when the service is available.");
    } finally {
      setMatching(false);
    }
  }

  function addManualPart() {
    const model = manualModel.trim();
    if (!model) {
      setManualError("Enter the model of the part you already have.");
      return;
    }
    const quantity = Number(manualQuantity.trim());
    if (!Number.isSafeInteger(quantity) || quantity < 1 || quantity > 1000) {
      setManualError("Quantity must be a whole number from 1 to 1000.");
      return;
    }
    const part: BuildPartInput = {
      source: "owned",
      category: manualCategory,
      model,
      quantity,
      specs: manualSpecs.trim() ? manualSpecs.trim() : undefined,
    };
    setManualError(null);
    setManualKey((value) => value + 1);
    setManual((current) => [
      ...current,
      { key: `manual-${manualKey}`, part, label: `${categoryLabel(manualCategory)} · ${model}` },
    ]);
    setManualModel("");
    setManualQuantity("1");
    setManualSpecs("");
    setLive(`Added ${categoryLabel(manualCategory)} ${model} to the described build.`);
  }

  function describeLineByHand(line: ParsedLine) {
    setManualCategory(line.category ?? "cpu");
    setManualModel(line.model);
    setManualQuantity(String(line.quantity));
    setManualSpecs("");
    setManualError(null);
    setChoices((current) => ({ ...current, [line.line]: SKIP }));
    setLive(
      `Line ${line.line} is set to be described by hand. Fill in its category, model and any specifications you have, then add it.`,
    );
    document.getElementById("manual-category")?.focus();
  }

  function resolve(line: ParsedLine, value: string) {
    setChoices((current) => ({ ...current, [line.line]: value }));
    const decision =
      value === SKIP
        ? `Line ${line.line} will be left out of the diagnosis.`
        : `Line ${line.line} will use ${value}.`;
    setLive(
      report
        ? `${decision} The described build changed, so the previous diagnosis is stale until it runs again.`
        : decision,
    );
  }

  async function runDiagnosis() {
    const reviewed = reviewedParts(list, choices);
    const build = [...reviewed.map((entry) => entry.part), ...manual.map((entry) => entry.part)];
    if (build.length === 0) {
      setProblem({
        title: "The build is empty",
        message: "Choose at least one matched line or add a part by hand.",
        code: "build_empty",
      });
      return;
    }
    const problems = reviewProblems(list, choices);
    if (problems.length > 0) {
      setProblem({
        title: "Resolve the match review first",
        message: problems.map((entry) => entry.message).join(" "),
        code: "review_incomplete",
      });
      return;
    }
    setPhase("diagnosing");
    setProblem(null);
    setLive("Diagnosing the described build with the compatibility engine…");
    try {
      const result = await diagnoseBuild({ snapshotId, currency, build });
      setReport(result);
      setPhase("done");
      setLive(
        `Diagnosis complete: ${result.check.overallStatus}. ${plural(
          result.groups.mustReplace.length,
          "part",
        )} must be replaced, ${plural(
          result.groups.shouldUpgrade.length,
          "part",
        )} should be verified, ${plural(
          result.groups.keep.length,
          "part",
        )} can be kept, and ${plural(
          result.groups.unverified.length,
          "part",
        )} not evaluated by any rule. Upgrade plan: ${result.plan.status.replace(/_/g, " ")}.`,
      );
    } catch (error) {
      setProblem(describeFailure(error));
      setPhase("failed");
      setLive("The diagnosis failed. Your parts and specifications are kept; retry when the service is available.");
    }
  }

  const reviewed = reviewedParts(list, choices);
  const partCount = reviewed.length + manual.length;
  const busy = phase === "diagnosing" || matching;

  return (
    <>
      <SiteHeader current="diagnose" />
      <main className="page" id="main" tabIndex={-1} aria-labelledby="diagnose-page-title">
        <h1 className="page-title" id="diagnose-page-title">
          Diagnose an existing build
        </h1>
        <p className="lede">
          Describe the PC you already have — paste a parts list, or add parts by hand with
          whatever specifications you can confirm. The diagnosis reuses the compatibility
          engine, then separates what must be replaced, what should be verified first,
          what is fine as is and what no rule could evaluate. A replacement plan is only
          shown as validated when the checker accepts the whole upgraded build.
        </p>
        <p className="privacy-note">
          Matching compares your lines with the selected local snapshot only. No hardware
          catalog, purchase order or AI provider is consulted, and nothing is reserved.
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
                    .then((entries) => {
                      setSnapshots(entries);
                      if (entries.length > 0) {
                        setSnapshotId(entries[0].id);
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
              <a href="/inventory">Import a CSV in Inventory</a> before matching a parts
              list. Parts you own can still be described by hand, but no price or
              replacement candidate is available without a snapshot.
            </p>
          ) : (
            <>
              <div className="field field--inline">
                <label htmlFor="diagnose-snapshot">Snapshot</label>
                <select
                  id="diagnose-snapshot"
                  value={snapshotId}
                  disabled={busy}
                  onChange={(event) => setSnapshotId(event.target.value)}
                >
                  <option value="">Choose a snapshot</option>
                  {snapshots.map((entry) => (
                    <option key={entry.id} value={entry.id}>
                      Revision {entry.revision} · {entry.sourceFilename} ·{" "}
                      {formatTimestamp(entry.createdAt)} ·{" "}
                      {plural(entry.recordCount, "record")} · {shortId(entry.id)}
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
                </div>
              ) : null}
            </>
          )}
        </section>

        {snapshot ? (
          <section className="panel" aria-labelledby="paste-title">
            <h2 className="panel__title" id="paste-title">
              2. Paste the parts list
            </h2>
            <p className="small muted">
              One part per line, at most 40 lines and 8 KiB. Start with the category
              where you know it, add <span className="mono">x2</span> for two units and{" "}
              <span className="mono">@549.00</span> for a price, for example{" "}
              <span className="mono">
                gpu: NVIDIA GeForce RTX 4070 Founders Edition x1 @549.00
              </span>
              .
            </p>
            <div className="field-grid">
              <div className="field">
                <label htmlFor="diagnose-text">Parts list</label>
                <textarea
                  id="diagnose-text"
                  rows={7}
                  value={text}
                  disabled={busy}
                  aria-describedby="diagnose-text-hint"
                  placeholder={
                    "cpu: AMD Ryzen 7 7800X3D\ngpu: NVIDIA GeForce RTX 4070 Founders Edition\npsu: Corsair RM750e"
                  }
                  onChange={(event) => {
                    setText(event.target.value);
                    // A match belongs to the text it was made from.
                    setList(null);
                    setChoices({});
                    setReport(null);
                    setPhase("idle");
                  }}
                />
                <span className="field__hint" id="diagnose-text-hint">
                  Pasted text is matched against this snapshot and never stored on its own.
                </span>
              </div>
              <div className="field">
                <label htmlFor="diagnose-currency">Currency for pasted prices</label>
                <input
                  id="diagnose-currency"
                  value={currency}
                  maxLength={3}
                  disabled={busy}
                  onChange={(event) => setCurrency(event.target.value.toUpperCase())}
                />
                <span className="field__hint">
                  Must match the snapshot currency ({snapshot.currency}). One currency per
                  build.
                </span>
              </div>
            </div>
            <div className="button-row">
              <button
                type="button"
                onClick={() => void runMatch()}
                disabled={busy || text.trim() === ""}
                aria-describedby="diagnose-match-hint"
              >
                {matching ? "Matching…" : "Match this list"}
              </button>
              <span className="small muted" id="diagnose-match-hint">
                {list
                  ? `${plural(list.lineCount, "line")} reviewed`
                  : text.trim() === ""
                    ? "Paste at least one part above to enable matching."
                    : "Matching never guesses a SKU for an ambiguous line."}
              </span>
            </div>
          </section>
        ) : null}

        {snapshot ? (
          <section className="panel" aria-labelledby="review-title">
            <h2 className="panel__title" id="review-title" tabIndex={-1} ref={reviewRef}>
              3. Review the matches
            </h2>
            {list ? (
              <>
                <p className="small muted">
                  A matched line is only a suggestion: the diagnosis uses the SKU you choose
                  here. Ambiguous lines start with no choice, and nothing is coerced to the
                  nearest part.
                </p>
                <ol className="review-list">
                  {list.lines.map((line) => {
                    const meta = MATCH_META[line.status];
                    const choice = choices[line.line] ?? "";
                    const unusable = line.duplicateOf != null || line.status === "invalid";
                    return (
                      <li className="review-line" key={line.line}>
                        <fieldset>
                          <legend>
                            Line {line.line} — {line.raw}
                          </legend>
                          <div className="review-line__meta">
                            <span className={`badge badge--${meta.className}`}>
                              <span className="badge__symbol" aria-hidden="true">
                                {meta.symbol}
                              </span>
                              {meta.label}
                            </span>
                            <span className="small muted">
                              {line.category ? categoryLabel(line.category) : "Category not stated"}{" "}
                              · quantity {line.quantity}
                              {line.priceMinor !== null
                                ? ` · ${formatMoney(line.priceMinor, list.currency)}`
                                : ""}
                              {" · "}
                              {lineDecisionLabel(line, choice)}
                            </span>
                          </div>
                          {line.issues.map((issue) => (
                            <p className="field__error" key={issue.code}>
                              {issue.message}
                            </p>
                          ))}
                          {line.duplicateOf != null ? (
                            <p className="small muted">
                              Line {line.duplicateOf} already describes this part, so this
                              duplicate is not added.
                            </p>
                          ) : null}
                          {unusable ? null : (
                            <div className="review-choices">
                              {line.candidates.map((candidate) => (
                                <div className="choice" key={candidate.sku}>
                                  <input
                                    type="radio"
                                    id={`line-${line.line}-${candidate.sku}`}
                                    name={`line-${line.line}`}
                                    value={candidate.sku}
                                    checked={choice === candidate.sku}
                                    disabled={busy}
                                    onChange={() => resolve(line, candidate.sku)}
                                  />
                                  <label htmlFor={`line-${line.line}-${candidate.sku}`}>
                                    <span>
                                      {candidate.model}{" "}
                                      <span className="mono small">{candidate.sku}</span>
                                      {candidate.exact ? " · exact tokens" : " · partial tokens"}
                                    </span>
                                    <span className="small muted">
                                      {formatMoney(candidate.priceMinor, candidate.currency)} ·{" "}
                                      {candidate.available} in stock
                                      {!candidate.exact &&
                                      candidate.extraTokens.length > 0
                                        ? ` · adds ${candidate.extraTokens.join(", ")}`
                                        : ""}
                                      {!candidate.exact &&
                                      candidate.missingTokens.length > 0
                                        ? ` · missing ${candidate.missingTokens.join(", ")}`
                                        : ""}
                                    </span>
                                  </label>
                                </div>
                              ))}
                              <div className="choice">
                                <input
                                  type="radio"
                                  id={`line-${line.line}-skip`}
                                  name={`line-${line.line}`}
                                  value={SKIP}
                                  checked={choice === SKIP}
                                  disabled={busy}
                                  onChange={() => resolve(line, SKIP)}
                                />
                                <label htmlFor={`line-${line.line}-skip`}>
                                  Leave this line out of the diagnosis
                                </label>
                              </div>
                            </div>
                          )}
                          {line.status === "unmatched" || line.status === "ambiguous" ? (
                            <p className="follow-up">
                              {line.disambiguation}{" "}
                              <button
                                type="button"
                                className="link-button"
                                onClick={() => describeLineByHand(line)}
                              >
                                Add it by hand instead
                              </button>
                            </p>
                          ) : null}
                          {line.disambiguation && line.status === "invalid" ? (
                            <p className="follow-up">{line.disambiguation}</p>
                          ) : null}
                        </fieldset>
                      </li>
                    );
                  })}
                </ol>
              </>
            ) : (
              <p className="small muted">
                Match a pasted list in step 2 to review its lines here. Parts you add by
                hand appear in step 4, and the diagnosis runs from step 4.
              </p>
            )}
          </section>
        ) : null}

        {snapshot ? (
          <section className="panel" aria-labelledby="manual-title">
            <h2 className="panel__title" id="manual-title">
              4. Add a part by hand
            </h2>
            <p className="small muted">
              Use this for parts the snapshot does not contain, or when you know a
              specification the match could not confirm. Every part you add is already
              owned, so it adds no purchase cost. Specification keys the system does not
              know stay unknown; nothing is inferred.
            </p>
            <div className="field-grid">
              <div className="field">
                <label htmlFor="manual-category">Category</label>
                <select
                  id="manual-category"
                  value={manualCategory}
                  disabled={busy}
                  onChange={(event) => setManualCategory(event.target.value)}
                >
                  {PART_CATEGORIES.map((category) => (
                    <option key={category.key} value={category.key}>
                      {category.label}
                    </option>
                  ))}
                </select>
              </div>
              <div className="field">
                <label htmlFor="manual-model">Model</label>
                <input
                  id="manual-model"
                  value={manualModel}
                  maxLength={200}
                  disabled={busy}
                  aria-describedby={manualError ? "manual-error" : undefined}
                  onChange={(event) => setManualModel(event.target.value)}
                />
              </div>
              <div className="field">
                <label htmlFor="manual-quantity">Quantity</label>
                <input
                  id="manual-quantity"
                  inputMode="numeric"
                  value={manualQuantity}
                  disabled={busy}
                  onChange={(event) => setManualQuantity(event.target.value)}
                />
              </div>
            </div>
            <div className="field">
              <label htmlFor="manual-specs">Specifications you have confirmed</label>
              <textarea
                id="manual-specs"
                rows={3}
                value={manualSpecs}
                disabled={busy}
                placeholder="socket=AM5;tdp=120W;igpu=yes"
                aria-describedby="manual-specs-hint"
                onChange={(event) => setManualSpecs(event.target.value)}
              />
              <span className="field__hint" id="manual-specs-hint">
                Semicolon-separated key=value pairs. A key the system does not recognise
                stays unknown rather than being guessed.
              </span>
            </div>
            {manualError ? (
              <p className="field__error" id="manual-error" role="alert">
                {manualError}
              </p>
            ) : null}
            <div className="button-row">
              <button type="button" onClick={addManualPart} disabled={busy}>
                Add this part
              </button>
              <span className="small muted">
                {plural(manual.length, "part")} added by hand
              </span>
            </div>
            {manual.length > 0 ? (
              <ul className="manual-list">
                {manual.map((entry) => (
                  <li key={entry.key}>
                    <span>{entry.label}</span>{" "}
                    <span className="small muted">
                      {plural(entry.part.quantity ?? 1, "unit")}
                      {entry.part.specs ? " · specifications recorded" : " · no specifications"}
                    </span>{" "}
                    <button
                      type="button"
                      className="link-button"
                      onClick={() => {
                        setManual((current) =>
                          current.filter((candidate) => candidate.key !== entry.key),
                        );
                        setLive(`Removed ${entry.label} from the described build.`);
                      }}
                    >
                      Remove
                    </button>
                  </li>
                ))}
              </ul>
            ) : null}
            <div className="button-row">
              <button
                type="button"
                onClick={() => void runDiagnosis()}
                disabled={busy || partCount === 0}
                aria-describedby="diagnose-run-hint"
              >
                {phase === "diagnosing" ? "Diagnosing…" : "Diagnose this build"}
              </button>
              <span className="small muted" id="diagnose-run-hint">
                {partCount === 0
                  ? "Add a matched line or a part by hand to enable the diagnosis."
                  : `${plural(partCount, "part")} will be diagnosed${
                      list
                        ? ` · ${reviewed.length} from the list, ${manual.length} by hand`
                        : ` · ${manual.length} by hand`
                    }`}
              </span>
            </div>
          </section>
        ) : null}

        {report && phase === "done" ? <DiagnosisReportPanel report={report} /> : null}

        {!report && snapshot ? (
          <section className="panel" aria-labelledby="no-diagnosis-title">
            <h2 className="panel__title" id="no-diagnosis-title">
              5. Read the diagnosis
            </h2>
            <p>
              Resolve the match review and run the diagnosis. The result groups every part
              into must replace, should upgrade, fine as is or not verified, cites the rule
              evidence, and validates any replacement plan against the same compatibility
              engine.
            </p>
          </section>
        ) : null}
      </main>
    </>
  );
}

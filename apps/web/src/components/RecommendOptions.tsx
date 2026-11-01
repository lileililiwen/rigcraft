import type { RefObject } from "react";
import type { Candidate, RecommendationReport } from "../types";
import { PART_CATEGORIES, verdictMeta } from "../compatibility";
import { formatMoney } from "../inventory";
import { bindingLabel, candidatePart, formatScore } from "../recommend";
import { CandidateCard } from "./CandidateCard";

export interface ExportNotice {
  kind: "ok" | "error";
  message: string;
}

interface Props {
  report: RecommendationReport | null;
  staleReport: boolean;
  headingRef: RefObject<HTMLHeadingElement>;
  exporting: "json" | "markdown" | null;
  exportNotice: ExportNotice | null;
  onExport: (format: "json" | "markdown") => void;
}

/**
 * The options column: every fact and every sentence the engine emits for the
 * report, from the binding constraint to the comparison and the exports.
 */
export function RecommendOptions({
  report,
  staleReport,
  headingRef,
  exporting,
  exportNotice,
  onExport,
}: Props) {
  if (!report) {
    return (
      <section className="panel workbench-panel" aria-labelledby="no-report-title">
        <p className="caps-label" aria-hidden="true">
          Options
        </p>
        <h2 className="panel__title" id="no-report-title">
          4. Read and compare options
        </h2>
        <p>
          Enter a budget and generate. Each option states why it was chosen, its
          cost and stock use, every rule verdict, what was excluded, and exactly
          which fact to confirm when evidence is missing.
        </p>
      </section>
    );
  }

  const current = report.candidates;

  return (
    <section
      className={`panel workbench-panel result-panel${staleReport ? " result-panel--stale" : ""}`}
      aria-labelledby="recommend-result-title"
    >
      <p className="caps-label" aria-hidden="true">
        Options
      </p>
      <h2
        className="panel__title"
        id="recommend-result-title"
        tabIndex={-1}
        ref={headingRef}
      >
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

      {report.noSolution ? (
        <div
          className="diagnostic-block diagnostic-block--conflict"
          role="group"
          aria-labelledby="no-solution-title"
        >
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
                      <td
                        data-label={`${candidate.rank}. ${candidate.label}`}
                        key={candidate.rank}
                      >
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
            onClick={() => onExport("json")}
            disabled={exporting !== null}
          >
            {exporting === "json" ? "Preparing JSON…" : "Download JSON export"}
          </button>
          <button
            type="button"
            className="button--secondary"
            onClick={() => onExport("markdown")}
            disabled={exporting !== null}
          >
            {exporting === "markdown" ? "Preparing Markdown…" : "Download Markdown plan"}
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
  );
}

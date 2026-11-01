import type { Candidate, RecommendationReport } from "../types";
import { verdictMeta } from "../compatibility";
import { formatMoney } from "../inventory";
import { bindingLabel, formatScore, plural, searchSummary } from "../recommend";

function StatusChip({ candidate }: { candidate: Candidate }) {
  const meta = verdictMeta(candidate.overallStatus);
  return (
    <span className={`badge badge--${meta.className} badge--verdict`}>
      <span className="badge__symbol" aria-hidden="true">
        {meta.symbol}
      </span>
      {meta.label}
    </span>
  );
}

function RuleSummary({ candidate }: { candidate: Candidate }) {
  const counts = [
    { status: "compatible" as const, count: candidate.checkSummary.compatible, noun: "compatible rule" },
    {
      status: "incompatible" as const,
      count: candidate.checkSummary.incompatible,
      noun: "conflict",
    },
    { status: "unknown" as const, count: candidate.checkSummary.unknown, noun: "unknown rule" },
  ];
  return (
    <ul className="rule-summary">
      {counts.map((entry) => {
        const meta = verdictMeta(entry.status);
        return (
          <li key={entry.status}>
            <span className={`badge badge--${meta.className}`}>
              <span className="badge__symbol" aria-hidden="true">
                {meta.symbol}
              </span>
              {plural(entry.count, entry.noun)}
            </span>
          </li>
        );
      })}
    </ul>
  );
}

/**
 * The decision, kept beside the options rather than repeated inside every card.
 * Every number here is read from the report; the rail computes nothing.
 */
export function RecommendVerdict({ report }: { report: RecommendationReport | null }) {
  const top = report?.candidates[0];
  const statusLabel = report
    ? report.status === "ok"
      ? "Solved"
      : report.status === "search_limited"
        ? "Limit reached"
        : "No solution"
    : "Not generated";

  return (
    <aside className="panel verdict-rail" aria-labelledby="verdict-title">
      <p className="caps-label" aria-hidden="true">
        Verdict · rule matrix
      </p>
      <h2 className="panel__title" id="verdict-title">
        Verdict and score
      </h2>

      {report ? (
        <>
          <dl className="summary-grid result-summary">
            <div>
              <dt>Distinct builds</dt>
              <dd>{report.candidates.length}</dd>
            </div>
            <div>
              <dt>Budget</dt>
              <dd>{formatMoney(report.budgetMinor, report.currency)}</dd>
            </div>
            <div>
              <dt>Valid candidates</dt>
              <dd>{report.search.validBuilds}</dd>
            </div>
            <div>
              <dt>Status</dt>
              <dd>{statusLabel}</dd>
            </div>
          </dl>
          <p className="counter-line">
            <span>{plural(report.search.validBuilds, "valid candidate")}</span>
            <span aria-hidden="true"> → </span>
            <span>{plural(report.candidates.length, "distinct build")}</span>
          </p>

          {top ? (
            <>
              <div className="verdict-rail__top">
                <p className="verdict-rail__top-label">
                  Top option: <strong>{top.rank}. {top.label}</strong>
                </p>
                <StatusChip candidate={top} />
              </div>
              <dl className="spec-rows verdict-rows">
                <div>
                  <dt>Budget delta</dt>
                  <dd>{formatMoney(top.budgetDeltaMinor, report.currency)}</dd>
                </div>
                <div>
                  <dt>Stock units</dt>
                  <dd>{top.unitsFromStock}</dd>
                </div>
                <div>
                  <dt>Score</dt>
                  <dd>
                    {formatScore(top.scores.total)}{" "}
                    <span className="small muted">{top.scores.confidence}</span>
                  </dd>
                </div>
              </dl>
              <h3 className="evidence-title">Rule matrix</h3>
              <RuleSummary candidate={top} />
            </>
          ) : (
            <p className="small muted">
              No option to score.{" "}
              {report.noSolution ? (
                <>
                  <strong>{bindingLabel(report.noSolution)}</strong> is binding; the
                  blockers are listed with the options.
                </>
              ) : null}
            </p>
          )}

          <div
            className={
              report.useCaseRule.requiresDiscreteGpu
                ? "diagnostic-block diagnostic-block--unknown"
                : "diagnostic-block"
            }
            role="note"
            data-use-case-rule={report.useCaseRule.useCase}
          >
            <h3>
              {report.useCaseRule.requiresDiscreteGpu
                ? "Gaming requires a discrete graphics card"
                : "Use-case completeness rule"}
            </h3>
            <p>{report.useCaseRule.explanation}</p>
          </div>

          {report.candidates.length > 0 ? (
            <p className="small muted" data-option-count={report.candidates.length}>
              {report.candidates.length < 3
                ? `${plural(report.candidates.length, "materially different build")} found; no option was padded to reach three.`
                : "Three materially different builds; each option has a different objective."}
            </p>
          ) : null}

          {report.search.limited ? (
            <div className="diagnostic-block diagnostic-block--unknown" role="status">
              <h3>Search limit reached</h3>
              <p>
                The search stopped at {report.search.limit} nodes after checking{" "}
                {plural(report.search.leavesEvaluated, "combination")}. These options
                are the best found so far; no global optimum is claimed. Narrow the
                request to let the search finish.
              </p>
            </div>
          ) : null}

          <p className="small muted">
            Snapshot revision {report.snapshotRevision} · scoring{" "}
            {report.scoringVersion} · rules {report.ruleSetVersion} ·{" "}
            {searchSummary(report.search)}
          </p>
        </>
      ) : (
        <p className="small muted">
          Generate a recommendation to read its status, budget delta, stock use,
          score and rule matrix here.
        </p>
      )}
    </aside>
  );
}

import type { Candidate, RuleResult } from "../types";
import { verdictMeta } from "../compatibility";
import { formatMoney } from "../inventory";
import { formatScore, formatWeight } from "../recommend";

function StatusBadge({ candidate }: { candidate: Candidate }) {
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

function RuleTable({ rules }: { rules: RuleResult[] }) {
  return (
    <div className="table-wrap">
      <table className="responsive-table">
        <caption>Every rule, its verdict and the fact to confirm</caption>
        <thead>
          <tr>
            <th scope="col">Rule</th>
            <th scope="col">Verdict</th>
            <th scope="col">Explanation</th>
            <th scope="col">Evidence</th>
          </tr>
        </thead>
        <tbody>
          {rules.map((rule) => {
            const meta = verdictMeta(rule.status);
            return (
              <tr key={rule.ruleId}>
                <td data-label="Rule">
                  {rule.title}
                  <span className="small muted mono"> · {rule.ruleId}</span>
                </td>
                <td data-label="Verdict">
                  <span className={`badge badge--${meta.className}`}>
                    <span className="badge__symbol" aria-hidden="true">
                      {meta.symbol}
                    </span>
                    {meta.label}
                  </span>
                </td>
                <td data-label="Explanation">
                  {rule.explanation}
                  {rule.followUp ? (
                    <span className="follow-up"> Confirm: {rule.followUp}</span>
                  ) : null}
                </td>
                <td data-label="Evidence">
                  <details className="details-block rule-evidence">
                    <summary>Compared values and provenance</summary>
                    <p className="small muted">{rule.method}</p>
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
                            <span className="small muted">
                              {" "}
                              · {entry.provenance?.reference ?? "No source recorded"}
                              {entry.provenance ? ` · ${entry.provenance.source}` : ""}
                            </span>
                          </dd>
                        </div>
                      ))}
                    </dl>
                  </details>
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}

export function CandidateCard({
  candidate,
  currency,
}: {
  candidate: Candidate;
  currency: string;
}) {
  const headingId = `candidate-${candidate.rank}-title`;
  return (
    <article className="candidate" aria-labelledby={headingId}>
      <div className="candidate__head">
        <h3 id={headingId}>
          <span className="candidate__rank">{candidate.rank}.</span> {candidate.label}
        </h3>
        <StatusBadge candidate={candidate} />
      </div>
      <dl className="summary-grid result-summary">
        <div>
          <dt>Total</dt>
          <dd>{formatMoney(candidate.totalMinor, currency)}</dd>
        </div>
        <div>
          <dt>Budget delta</dt>
          <dd>{formatMoney(candidate.budgetDeltaMinor, currency)}</dd>
        </div>
        <div>
          <dt>Stock units</dt>
          <dd>{candidate.unitsFromStock}</dd>
        </div>
        <div>
          <dt>Score</dt>
          <dd>
            {formatScore(candidate.scores.total)}{" "}
            <span className="small muted">{candidate.scores.confidence}</span>
          </dd>
        </div>
      </dl>
      <h4 className="evidence-title">Why this build</h4>
      <ul className="reason-list">
        {candidate.reasons.map((reason) => (
          <li key={reason}>{reason}</li>
        ))}
      </ul>
      <h4 className="evidence-title">Parts, price and stock</h4>
      <div className="table-wrap">
        <table className="responsive-table">
          <caption>Every part comes from the selected inventory snapshot</caption>
          <thead>
            <tr>
              <th scope="col">Part</th>
              <th scope="col">SKU</th>
              <th scope="col">Quantity</th>
              <th scope="col">Unit price</th>
              <th scope="col">Line total</th>
              <th scope="col">Stock</th>
            </tr>
          </thead>
          <tbody>
            {candidate.parts.map((part) => {
              const line = candidate.stock.find((entry) => entry.sku === part.sku);
              return (
                <tr key={part.sku}>
                  <td data-label="Part">
                    {part.model}
                    <span className="small muted"> · {part.category}</span>
                  </td>
                  <td data-label="SKU">
                    <span className="mono">{part.sku}</span>
                  </td>
                  <td data-label="Quantity">{part.quantity}</td>
                  <td data-label="Unit price">{formatMoney(part.unitPriceMinor, currency)}</td>
                  <td data-label="Line total">{formatMoney(part.lineTotalMinor, currency)}</td>
                  <td data-label="Stock">
                    {part.owned
                      ? `Owned · ${part.available} available`
                      : `${line?.used ?? part.quantity} used · ${line?.remaining ?? 0} left`}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
      <h4 className="evidence-title">Score breakdown</h4>
      <div className="table-wrap">
        <table className="responsive-table">
          <caption>Published weights and the evidence behind each dimension</caption>
          <thead>
            <tr>
              <th scope="col">Dimension</th>
              <th scope="col">Value</th>
              <th scope="col">Weight</th>
              <th scope="col">Basis</th>
            </tr>
          </thead>
          <tbody>
            {candidate.scores.dimensions.map((dimension) => (
              <tr key={dimension.key}>
                <td data-label="Dimension">{dimension.label}</td>
                <td data-label="Value">
                  {dimension.available ? formatScore(dimension.value) : "Not scored"}
                </td>
                <td data-label="Weight">
                  {dimension.available ? formatWeight(dimension.weight) : "—"}
                </td>
                <td data-label="Basis">{dimension.basis}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {candidate.alternatives.length > 0 ? (
        <details className="details-block">
          <summary>Alternatives not selected</summary>
          <ul className="diagnostic-list">
            {candidate.alternatives.map((alternative) => (
              <li key={`${alternative.category}-${alternative.sku}`}>
                <strong>
                  {alternative.category}: {alternative.model}
                </strong>{" "}
                <span className="mono small">{alternative.sku}</span>
                <p>{alternative.detail}</p>
              </li>
            ))}
          </ul>
        </details>
      ) : null}
      {candidate.unknownFollowUps.length > 0 ? (
        <div className="diagnostic-block diagnostic-block--unknown">
          <h4>Confirm before building</h4>
          <ul className="missing-list">
            {candidate.unknownFollowUps.map((followUp) => (
              <li key={followUp}>{followUp}</li>
            ))}
          </ul>
        </div>
      ) : null}
      <details className="details-block">
        <summary>Rule evidence for this build</summary>
        <RuleTable rules={candidate.check.rules} />
      </details>
      <p className="small muted">
        Check report <span className="mono">{candidate.checkReportId}</span> · rule set{" "}
        {candidate.check.ruleSetVersion} · snapshot revision{" "}
        {candidate.check.snapshotRevision}
      </p>
    </article>
  );
}

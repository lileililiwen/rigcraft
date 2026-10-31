import type { Advice, DiagnosisReport, RuleEvidence, RuleResult } from "../types";
import { categoryLabel } from "../compatibility";
import { ACTION_META, PLAN_META, groupRows, reportHeadline, verdictWord } from "../diagnosis";
import { formatDate, formatMoney, shortId, sourceLabel } from "../inventory";
import { VERDICT_META } from "../compatibility";

function EvidenceList({ evidence }: { evidence: RuleEvidence[] }) {
  if (evidence.length === 0) {
    return <p className="small muted">No compared field is recorded for this part.</p>;
  }
  return (
    <dl className="evidence-list">
      {evidence.map((entry, index) => (
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
                {entry.provenance.observedAt}
              </span>
            ) : (
              <span className="small muted"> · No source recorded</span>
            )}
          </dd>
        </div>
      ))}
    </dl>
  );
}

function AdviceCard({ advice }: { advice: Advice }) {
  const meta = ACTION_META[advice.action];
  return (
    <li className={`advice-card advice-card--${advice.action}`}>
      <div className="advice-card__head">
        <span className={`badge badge--${meta.className}`}>
          <span className="badge__symbol" aria-hidden="true">
            {meta.symbol}
          </span>
          {meta.label}
          {advice.conditional ? " (verify first)" : ""}
        </span>
        <span className="advice-card__part">
          <strong>{advice.model}</strong>{" "}
          <span className="small muted">
            {categoryLabel(advice.category)} · <span className="mono">{advice.sku}</span>
          </span>
        </span>
      </div>
      <p>{advice.reason}</p>
      {advice.verification ? (
        <p className="follow-up">Confirm first: {advice.verification}</p>
      ) : null}
      <p className="small">
        Rules:{" "}
        {advice.ruleIds.length === 0 ? (
          <span className="muted">none</span>
        ) : (
          advice.ruleIds.map((ruleId, index) => (
            <span key={ruleId}>
              {index > 0 ? ", " : ""}
              <a href={`#rule-${ruleId}`}>{ruleId}</a>
            </span>
          ))
        )}
      </p>
      <p className="small cost-line">
        Keep cost {formatMoney(advice.cost.keepMinor, advice.cost.currency)}
        {advice.cost.replacementSku ? (
          <>
            {" "}
            · Cheapest replacement{" "}
            <span className="mono">{advice.cost.replacementSku}</span>{" "}
            {advice.cost.replacementMinor !== null
              ? formatMoney(advice.cost.replacementMinor, advice.cost.currency)
              : ""}
          </>
        ) : null}
        {advice.cost.note ? <span className="muted"> — {advice.cost.note}</span> : null}
      </p>
      <details className="details-block">
        <summary>Compatibility evidence for {advice.sku}</summary>
        <EvidenceList evidence={advice.evidence} />
      </details>
    </li>
  );
}

function Group({
  title,
  description,
  entries,
  emptyText,
}: {
  title: string;
  description: string;
  entries: Advice[];
  emptyText: string;
}) {
  return (
    <section className="advice-group" aria-label={title}>
      <h3 className="advice-group__title">
        {title} <span className="muted">({entries.length})</span>
      </h3>
      <p className="small muted">{description}</p>
      {entries.length === 0 ? (
        <p className="small">{emptyText}</p>
      ) : (
        <ul className="advice-list">
          {entries.map((advice) => (
            <AdviceCard advice={advice} key={advice.row} />
          ))}
        </ul>
      )}
    </section>
  );
}

function RuleEvidence({ rule }: { rule: RuleResult }) {
  return (
    <details className="details-block rule-evidence" id={`rule-${rule.ruleId}`}>
      <summary>
        {rule.ruleId} · {rule.title}
      </summary>
      <p className="small muted">{rule.method}</p>
      <EvidenceList evidence={rule.evidence} />
    </details>
  );
}

export function DiagnosisReportPanel({ report }: { report: DiagnosisReport }) {
  const meta = VERDICT_META[report.check.overallStatus];
  const planMeta = PLAN_META[report.plan.status];
  const mustReplace = groupRows(report, "must_replace");
  const shouldUpgrade = groupRows(report, "should_upgrade");
  const keep = groupRows(report, "keep");
  return (
    <section
      className={`panel result-panel${report.stale ? " result-panel--stale" : ""}`}
      aria-labelledby="diagnosis-result-title"
    >
      <h2 className="panel__title" id="diagnosis-result-title" tabIndex={-1}>
        Build diagnosis
      </h2>
      {report.stale ? (
        <div className="stale-note" role="status">
          <strong>This diagnosis is stale.</strong>{" "}
          {report.staleReason ??
            "The inventory changed after this diagnosis ran."}{" "}
          Run it again before treating it as current.
        </div>
      ) : null}
      <div className="verdict-line">
        <span className={`badge badge--${meta.className} badge--verdict`}>
          <span className="badge__symbol" aria-hidden="true">
            {meta.symbol}
          </span>
          {meta.label}
        </span>
        <span className="verdict-completeness">
          {report.check.completeness.status === "complete"
            ? "Complete build"
            : `Incomplete build · ${report.check.completeness.missing.length} missing`}
        </span>
        <span className="small muted">
          Checked {formatDate(report.check.checkedAt)} · snapshot revision{" "}
          {report.snapshotRevision} · rule set {report.ruleSetVersion} ·{" "}
          {shortId(report.id)}
        </span>
      </div>
      <p className="lede">{reportHeadline(report)}</p>
      <dl className="summary-grid result-summary">
        <div>
          <dt>Rules passed</dt>
          <dd>{report.check.summary.compatible}</dd>
        </div>
        <div>
          <dt>Conflicts</dt>
          <dd>{report.check.summary.incompatible}</dd>
        </div>
        <div>
          <dt>Unknown</dt>
          <dd>{report.check.summary.unknown}</dd>
        </div>
      </dl>

      {report.check.completeness.status === "incomplete" ? (
        <div className="diagnostic-block diagnostic-block--unknown">
          <h3>Build gaps</h3>
          <ul className="missing-list">
            {report.check.completeness.missing.map((entry) => (
              <li key={entry.category}>
                <strong>{categoryLabel(entry.category)}</strong>: {entry.reason}.{" "}
                <span>{entry.followUp}</span>
              </li>
            ))}
          </ul>
        </div>
      ) : null}

      <Group
        title="Must replace"
        description="A definite conflict is recorded for these parts; keeping them contradicts the checker."
        entries={mustReplace}
        emptyText="No described part is in a definite conflict."
      />
      <Group
        title="Should upgrade — verify first"
        description="No conflict is recorded, but a required fact is missing. Gather the evidence first; replace only if it shows the part is inadequate."
        entries={shouldUpgrade}
        emptyText="No part is waiting on missing evidence."
      />
      <Group
        title="Fine as is"
        description="Every rule involving these parts passed, or the plan resolves their conflict elsewhere."
        entries={keep}
        emptyText="No part is kept by this diagnosis."
      />

      <section className="plan-block" aria-labelledby="plan-title">
        <h3 id="plan-title" className="evidence-title">
          Upgrade plan
        </h3>
        <div className="verdict-line">
          <span className={`badge badge--${planMeta.className}`}>
            <span className="badge__symbol" aria-hidden="true">
              {planMeta.symbol}
            </span>
            {planMeta.label}
          </span>
          <span className="small muted">
            Each replacement is re-checked against {report.check.ruleSetVersion} with the
            retained parts; {report.plan.evaluated} candidate substitution(s) evaluated.
          </span>
        </div>
        <p>{report.plan.summary}</p>
        {report.plan.steps.length > 0 ? (
          <div className="table-wrap">
            <table className="responsive-table">
              <caption>Steps the checker validated as one whole build</caption>
              <thead>
                <tr>
                  <th scope="col">Category</th>
                  <th scope="col">Replace</th>
                  <th scope="col">With</th>
                  <th scope="col">Price</th>
                  <th scope="col">Rules</th>
                </tr>
              </thead>
              <tbody>
                {report.plan.steps.map((step) => (
                  <tr key={`${step.row}-${step.toSku}`}>
                    <td data-label="Category">{categoryLabel(step.category)}</td>
                    <td data-label="Replace">
                      {step.fromModel} <span className="mono small">{step.fromSku}</span>
                    </td>
                    <td data-label="With">
                      {step.toModel} <span className="mono small">{step.toSku}</span>
                    </td>
                    <td data-label="Price">
                      {formatMoney(step.priceMinor, report.currency)}
                    </td>
                    <td data-label="Rules">{step.ruleIds.join(", ")}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : null}
        {report.plan.steps.length > 0 ? (
          <p className="small">
            Additional cost{" "}
            <strong>{formatMoney(report.plan.additionalCostMinor, report.currency)}</strong>{" "}
            · no trade-in or resale value is assumed ·{" "}
            {report.plan.validation
              ? `whole-plan check: ${verdictWord(report.plan.validation.overallStatus)}`
              : "whole-plan check: not run"}
          </p>
        ) : null}
        {report.plan.followUps.length > 0 ? (
          <div className="diagnostic-block diagnostic-block--unknown">
            <h4>Facts to confirm before relying on this plan</h4>
            <ul className="diagnostic-list">
              {report.plan.followUps.map((followUp, index) => (
                <li key={`${index}-${followUp}`}>{followUp}</li>
              ))}
            </ul>
          </div>
        ) : null}
        {report.plan.rejected.length > 0 ? (
          <details className="details-block">
            <summary>
              Rejected or unvalidated candidates ({report.plan.rejected.length})
            </summary>
            <ul className="diagnostic-list">
              {report.plan.rejected.map((entry) => (
                <li key={`${entry.row}-${entry.sku}`}>
                  <span className="mono">{entry.sku}</span> · {entry.model} ·{" "}
                  {formatMoney(entry.priceMinor, report.currency)} ·{" "}
                  {entry.ruleIds.join(", ") || "no rule"} — {entry.detail}
                </li>
              ))}
            </ul>
          </details>
        ) : null}
        {report.plan.options.length > 0 ? (
          <details className="details-block">
            <summary>Replacement options the checker evaluated</summary>
            <ul className="diagnostic-list">
              {report.plan.options.map((option) => (
                <li key={`${option.row}-${option.sku}`}>
                  <span className="mono">{option.sku}</span> · {option.model} ·{" "}
                  {formatMoney(option.priceMinor, report.currency)} ·{" "}
                  {verdictWord(option.status)}
                  {option.resolves.length > 0
                    ? ` · resolves ${option.resolves.join(", ")}`
                    : ""}
                </li>
              ))}
            </ul>
          </details>
        ) : null}
        <p className="small muted">{report.plan.method}</p>
      </section>

      <h3 className="evidence-title">Described build and rule evidence</h3>
      <div className="table-wrap">
        <table className="responsive-table">
          <caption>Every part as described, and every rule with its evidence</caption>
          <thead>
            <tr>
              <th scope="col">Part</th>
              <th scope="col">Facts from</th>
              <th scope="col">Quantity</th>
            </tr>
          </thead>
          <tbody>
            {report.build.map((part) => (
              <tr key={part.row}>
                <td data-label="Part">
                  {part.model} <span className="mono small">{part.sku}</span>
                  <span className="small muted"> · {categoryLabel(part.category)}</span>
                </td>
                <td data-label="Facts from">
                  {part.source === "snapshot" ? "Snapshot record" : "Your description"}
                </td>
                <td data-label="Quantity">{part.quantity}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <div className="rule-list">
        {report.check.rules.map((rule) => (
          <RuleEvidence rule={rule} key={rule.ruleId} />
        ))}
      </div>
      <p className="privacy-note">
        Every verdict cites the facts you described or the snapshot record with its
        recorded source. No hardware catalog is consulted, model match confidence stays
        unverified, and a missing fact is never filled in from a similar name.
      </p>
    </section>
  );
}

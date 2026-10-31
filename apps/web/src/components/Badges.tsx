import type { FieldError, Provenance, Spec } from "../types";
import { formatTimestamp, sourceLabel } from "../inventory";

const STATUS = {
  valid: { symbol: "✓", text: "Valid" },
  error: { symbol: "✕", text: "Error" },
  duplicate: { symbol: "⇄", text: "Duplicate SKU" },
  corrected: { symbol: "✓", text: "Corrected" },
} as const;

export function StatusBadge({ status }: { status: keyof typeof STATUS }) {
  const value = STATUS[status];
  return (
    <span className={`badge badge--${status}`}>
      <span className="badge__symbol" aria-hidden="true">
        {value.symbol}
      </span>
      {value.text}
    </span>
  );
}

export function SpecBadge({
  status,
  source,
}: {
  status: "confirmed" | "unknown";
  source: string;
}) {
  if (status === "unknown") {
    return (
      <span className="badge badge--unknown">
        <span className="badge__symbol" aria-hidden="true">
          ?
        </span>
        Unknown
      </span>
    );
  }
  if (source.startsWith("catalog:")) {
    return (
      <span className="badge badge--catalog" title={source}>
        <span className="badge__symbol" aria-hidden="true">
          ▣
        </span>
        Catalog
      </span>
    );
  }
  return (
    <span className="badge badge--confirmed">
      <span className="badge__symbol" aria-hidden="true">
        ✎
      </span>
      Operator-confirmed
    </span>
  );
}

export function FieldMessages({
  errors,
  idPrefix,
}: {
  errors: FieldError[];
  idPrefix: string;
}) {
  if (errors.length === 0) {
    return null;
  }
  return (
    <ul className="error-list" id={`${idPrefix}-errors`}>
      {errors.map((error) => (
        <li key={`${error.field}-${error.code}`}>
          <strong>{error.field}</strong>: {error.message}
          {error.suggestion ? <> {error.suggestion}</> : null}
        </li>
      ))}
    </ul>
  );
}

export function SpecList({ specs }: { specs: Spec[] }) {
  if (specs.length === 0) {
    return (
      <p className="small">
        <span className="badge badge--unknown">
          <span className="badge__symbol" aria-hidden="true">
            ?
          </span>
          Unknown
        </span>{" "}
        No specifications supplied. Confirm them before any compatibility check.
      </p>
    );
  }
  return (
    <ul className="spec-list">
      {specs.map((spec) => (
        <li key={spec.key}>
          <span className="tag">{spec.key}</span>
          <span>
            {spec.number !== undefined ? spec.number : spec.text}
            {spec.unit ? ` ${spec.unit}` : ""}
          </span>
          <SpecBadge status={spec.status} source={spec.provenance.source} />
        </li>
      ))}
    </ul>
  );
}

export function ProvenanceList({
  provenance,
  match,
}: {
  provenance: Record<string, Provenance>;
  match?: { basis: string; confidence: string };
}) {
  const entries = Object.entries(provenance);
  if (entries.length === 0 && !match) {
    return <p className="small muted">No provenance recorded.</p>;
  }
  return (
    <>
      {match ? (
        <p className="small">
          Match confidence: <strong>{match.confidence}</strong>. {match.basis}
        </p>
      ) : null}
      <ul className="provenance-list">
        {entries.map(([field, value]) => (
          <li key={field}>
            <strong>{field}</strong>: {sourceLabel(value.source)} ·{" "}
            <span className="tag">{value.reference}</span> ·{" "}
            <time dateTime={value.observedAt}>{formatTimestamp(value.observedAt)}</time>
          </li>
        ))}
      </ul>
    </>
  );
}

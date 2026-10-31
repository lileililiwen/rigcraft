import { useEffect, useState } from "react";
import type {
  DuplicateAction,
  FieldError,
  Preview,
  PreviewRow,
  RowOverrides,
  RowValues,
} from "../types";
import {
  CATEGORIES,
  LOGICAL_FIELDS,
  REQUIRED_FIELDS,
  acceptedRows,
  canCommit,
  continuationSummary,
  formatMoney,
  overrideValues,
  rowIssues,
  rowsNeedingResolution,
} from "../inventory";
import type { DecisionMap, OverrideMap, ResolutionMap } from "../inventory";
import { FieldMessages, StatusBadge } from "./Badges";

interface PreviewReviewProps {
  preview: Preview;
  decisions: DecisionMap;
  resolutions: ResolutionMap;
  overrides: OverrideMap;
  serverIssues: Record<number, FieldError[]>;
  busy: boolean;
  onDecision: (row: number, action: "accept" | "exclude") => void;
  onResolution: (groupId: string, action: DuplicateAction) => void;
  onOverride: (row: number, overrides: RowOverrides | undefined) => void;
  onCommit: () => void;
  onReparse: (mapping: Record<string, string>) => void;
  onStartOver: () => void;
}

const EDITABLE: Array<keyof RowOverrides> = [
  "sku",
  "category",
  "model",
  "quantity",
  "price",
  "specs",
];

export function PreviewReview({
  preview,
  decisions,
  resolutions,
  overrides,
  serverIssues,
  busy,
  onDecision,
  onResolution,
  onOverride,
  onCommit,
  onReparse,
  onStartOver,
}: PreviewReviewProps) {
  const [editingRow, setEditingRow] = useState<number | null>(null);
  const [draft, setDraft] = useState<RowValues | null>(null);
  const [mapping, setMapping] = useState<Record<string, string>>(preview.mapping);

  useEffect(() => {
    setMapping(preview.mapping);
    setEditingRow(null);
    setDraft(null);
  }, [preview.previewId, preview.mapping]);

  const readiness = canCommit(preview, decisions, resolutions, overrides, serverIssues);
  const unresolved = rowsNeedingResolution(preview, decisions, resolutions);
  const accepted = acceptedRows(preview, decisions);
  const missingMapped = REQUIRED_FIELDS.filter((field) => !mapping[field]);

  function openEditor(row: PreviewRow) {
    setEditingRow(row.row);
    setDraft(overrideValues(row, overrides[row.row]));
  }

  function applyEditor(row: PreviewRow) {
    if (!draft) {
      return;
    }
    const next: RowOverrides = {};
    for (const field of EDITABLE) {
      if (draft[field] !== row.values[field]) {
        next[field] = draft[field];
      }
    }
    onOverride(row.row, Object.keys(next).length > 0 ? next : undefined);
    setEditingRow(null);
    setDraft(null);
  }

  return (
    <>
      <section className="panel" aria-labelledby="review-title">
        <h2 className="panel__title" id="review-title" tabIndex={-1}>
          Review import
        </h2>
        <p className="lede">
          {preview.filename} · batch currency {preview.currency} ·{" "}
          {continuationSummary(preview.rows)}
        </p>
        <dl className="summary-grid">
          <div>
            <dt>Rows parsed</dt>
            <dd>{preview.summary.total}</dd>
          </div>
          <div>
            <dt>Valid</dt>
            <dd>{preview.summary.valid}</dd>
          </div>
          <div>
            <dt>Errors</dt>
            <dd>{preview.summary.errors}</dd>
          </div>
          <div>
            <dt>Duplicate rows</dt>
            <dd>{preview.summary.duplicateRows}</dd>
          </div>
          <div>
            <dt>Included now</dt>
            <dd>{accepted.length}</dd>
          </div>
        </dl>
        <p className="small muted">
          Preview expires {new Date(preview.expiresAt).toLocaleString()}. No row is
          written until you commit, and a failed commit writes nothing.
        </p>

        <div className="table-wrap">
          <table className="responsive-table">
            <caption>Parsed CSV rows with their validation result and decision</caption>
            <thead>
              <tr>
                <th scope="col">Row</th>
                <th scope="col">Status</th>
                <th scope="col">SKU</th>
                <th scope="col">Category</th>
                <th scope="col">Model</th>
                <th scope="col">Qty</th>
                <th scope="col">Price ({preview.currency})</th>
                <th scope="col">Specifications</th>
                <th scope="col">Decision</th>
                <th scope="col">Correct</th>
              </tr>
            </thead>
            <tbody>
              {preview.rows.map((row) => {
                const override = overrides[row.row];
                const issues = rowIssues(row, override, serverIssues[row.row]);
                const display = overrideValues(row, override);
                const effectiveStatus =
                  issues.length > 0
                    ? "error"
                    : override
                      ? "corrected"
                      : row.status;
                return (
                  <tr key={row.row}>
                    <td data-label="Row">{row.row}</td>
                    <td data-label="Status">
                      <StatusBadge status={effectiveStatus} />
                      <FieldMessages errors={issues} idPrefix={`row-${row.row}`} />
                      {row.duplicateGroupId ? (
                        <p className="small muted">
                          Same SKU as the other rows in duplicate group{" "}
                          {row.duplicateGroupId}.
                        </p>
                      ) : null}
                      {override ? (
                        <p className="small muted">
                          Corrected values are shown. Choose Include to add this row,
                          or Exclude to leave it out.
                        </p>
                      ) : null}
                    </td>
                    <td data-label="SKU" className="mono">
                      {display.sku || "—"}
                    </td>
                    <td data-label="Category">{display.category || "—"}</td>
                    <td data-label="Model">
                      <span>{display.model || "—"}</span>
                      {row.normalized.modelNormalized &&
                      row.normalized.modelNormalized !== row.values.model.trim() &&
                      !override?.model ? (
                        <span className="small muted">
                          {" "}
                          → {row.normalized.modelNormalized}
                        </span>
                      ) : null}
                      <span className="small muted"> · match unverified</span>
                    </td>
                    <td data-label="Quantity">{display.quantity || "—"}</td>
                    <td data-label="Price">
                      {override?.price ? (
                        <>
                          {display.price}
                          <span className="small muted"> (corrected)</span>
                        </>
                      ) : row.normalized.priceMinor !== null ? (
                        formatMoney(row.normalized.priceMinor, preview.currency)
                      ) : (
                        display.price || "—"
                      )}
                    </td>
                    <td data-label="Specifications">
                      {override?.specs ? (
                        <span className="small">
                          <span className="tag">{display.specs || "none"}</span>{" "}
                          (corrected; revalidated on commit)
                        </span>
                      ) : row.specs.length === 0 ? (
                        <span className="small">Unknown — no specs</span>
                      ) : (
                        <ul className="spec-list">
                          {row.specs.map((spec) => (
                            <li key={spec.key}>
                              <span className="tag">{spec.key}</span>{" "}
                              {spec.number !== undefined ? spec.number : spec.text}
                              {spec.unit ? ` ${spec.unit}` : ""}{" "}
                              {spec.status === "unknown" ? (
                                <span className="badge badge--unknown">
                                  <span className="badge__symbol" aria-hidden="true">
                                    ?
                                  </span>
                                  Unknown
                                </span>
                              ) : null}
                            </li>
                          ))}
                        </ul>
                      )}
                    </td>
                    <td data-label="Decision">
                      <fieldset>
                        <legend className="visually-hidden">
                          Include row {row.row} in the snapshot
                        </legend>
                        <div className="choice">
                          <label>
                            <input
                              type="radio"
                              name={`decision-${row.row}`}
                              value="accept"
                              checked={decisions[row.row] === "accept"}
                              onChange={() => onDecision(row.row, "accept")}
                            />
                            Include
                          </label>
                        </div>
                        <div className="choice">
                          <label>
                            <input
                              type="radio"
                              name={`decision-${row.row}`}
                              value="exclude"
                              checked={decisions[row.row] === "exclude"}
                              onChange={() => onDecision(row.row, "exclude")}
                            />
                            Exclude
                          </label>
                        </div>
                      </fieldset>
                    </td>
                    <td data-label="Correct">
                      <button
                        type="button"
                        className="button--secondary"
                        aria-expanded={editingRow === row.row}
                        aria-controls={
                          editingRow === row.row ? `correct-${row.row}` : undefined
                        }
                        onClick={() =>
                          editingRow === row.row
                            ? setEditingRow(null)
                            : openEditor(row)
                        }
                      >
                        {overrides[row.row] ? "Edit correction" : "Correct"}
                      </button>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>

        {editingRow !== null && draft
          ? (() => {
              const row = preview.rows.find((entry) => entry.row === editingRow);
              if (!row) {
                return null;
              }
              const firstError = row.errors[0]?.field;
              const focusField = ["sku", "category", "model", "quantity", "price", "specs"].includes(
                firstError ?? "",
              )
                ? firstError
                : "sku";
              return (
                <form
                  className="correction-panel"
                  id={`correct-${row.row}`}
                  onSubmit={(event) => {
                    event.preventDefault();
                    applyEditor(row);
                  }}
                >
                  <h3>Correct row {row.row}</h3>
                  <p className="small">
                    Corrections are revalidated. They only change this preview; the
                    committed snapshot is immutable.
                  </p>
                  <div className="field-grid">
                    <div className="field">
                      <label htmlFor={`correct-sku-${row.row}`}>SKU</label>
                      <input
                        id={`correct-sku-${row.row}`}
                        value={draft.sku}
                        autoFocus={focusField === "sku"}
                        onChange={(event) =>
                          setDraft({ ...draft, sku: event.target.value })
                        }
                      />
                    </div>
                    <div className="field">
                      <label htmlFor={`correct-category-${row.row}`}>Category</label>
                      <select
                        id={`correct-category-${row.row}`}
                        value={draft.category}
                        autoFocus={focusField === "category"}
                        onChange={(event) =>
                          setDraft({ ...draft, category: event.target.value })
                        }
                      >
                        <option value="">Choose a category</option>
                        {!CATEGORIES.includes(draft.category) &&
                        draft.category !== "" ? (
                          <option value={draft.category}>
                            {draft.category} (unsupported)
                          </option>
                        ) : null}
                        {CATEGORIES.map((category) => (
                          <option key={category} value={category}>
                            {category}
                          </option>
                        ))}
                      </select>
                    </div>
                    <div className="field">
                      <label htmlFor={`correct-model-${row.row}`}>Model</label>
                      <input
                        id={`correct-model-${row.row}`}
                        value={draft.model}
                        autoFocus={focusField === "model"}
                        onChange={(event) =>
                          setDraft({ ...draft, model: event.target.value })
                        }
                      />
                    </div>
                    <div className="field">
                      <label htmlFor={`correct-quantity-${row.row}`}>Quantity</label>
                      <input
                        id={`correct-quantity-${row.row}`}
                        inputMode="numeric"
                        value={draft.quantity}
                        autoFocus={focusField === "quantity"}
                        onChange={(event) =>
                          setDraft({ ...draft, quantity: event.target.value })
                        }
                      />
                    </div>
                    <div className="field">
                      <label htmlFor={`correct-price-${row.row}`}>
                        Sale price ({preview.currency})
                      </label>
                      <input
                        id={`correct-price-${row.row}`}
                        inputMode="decimal"
                        value={draft.price}
                        autoFocus={focusField === "price"}
                        onChange={(event) =>
                          setDraft({ ...draft, price: event.target.value })
                        }
                      />
                    </div>
                    <div className="field">
                      <label htmlFor={`correct-specs-${row.row}`}>
                        Specifications (key=value;…)
                      </label>
                      <textarea
                        id={`correct-specs-${row.row}`}
                        rows={2}
                        value={draft.specs}
                        autoFocus={focusField === "specs"}
                        onChange={(event) =>
                          setDraft({ ...draft, specs: event.target.value })
                        }
                      />
                    </div>
                  </div>
                  <div className="button-row">
                    <button type="submit">Apply correction</button>
                    <button
                      type="button"
                      className="button--secondary"
                      onClick={() => {
                        onOverride(row.row, undefined);
                        setEditingRow(null);
                        setDraft(null);
                      }}
                    >
                      Remove correction
                    </button>
                    <button
                      type="button"
                      className="button--secondary"
                      onClick={() => {
                        setEditingRow(null);
                        setDraft(null);
                      }}
                    >
                      Cancel
                    </button>
                  </div>
                </form>
              );
            })()
          : null}

        {preview.duplicateGroups.length > 0 ? (
          <div className="duplicate-list">
            {preview.duplicateGroups.map((group) => {
              const acceptedInGroup = group.rows.filter(
                (row) => decisions[row] === "accept",
              );
              return (
                <fieldset className="duplicate-group" key={group.groupId}>
                  <legend>Duplicate SKU: {group.sku}</legend>
                  <p>
                    Rows {group.rows.join(", ")} share this SKU.{" "}
                    {acceptedInGroup.length > 1
                      ? "Choose how they become stock records."
                      : "Only one row is included, so no merge decision is required."}
                  </p>
                  <div className="duplicate-options">
                    {(
                      [
                        ["merge", "Merge quantities", "One record; quantities add up. Prices, model, and specs must match."],
                        ["replace", "Replace with last row", "One record; the last included row wins."],
                        ["keep_separate", "Keep as separate lots", "Each included row stays a separate record with its own lot."],
                      ] as Array<[DuplicateAction, string, string]>
                    ).map(([action, label, hint]) => (
                      <label className="choice" key={action}>
                        <input
                          type="radio"
                          name={`duplicate-${group.groupId}`}
                          value={action}
                          checked={resolutions[group.groupId] === action}
                          onChange={() => onResolution(group.groupId, action)}
                        />
                        <span>
                          {label}
                          <span className="small muted"> — {hint}</span>
                        </span>
                      </label>
                    ))}
                  </div>
                </fieldset>
              );
            })}
          </div>
        ) : null}

        {unresolved.length > 0 ? (
          <p className="field__error" id="unresolved-note">
            Resolve duplicate group{unresolved.length === 1 ? "" : "s"}{" "}
            {unresolved.join(", ")} before committing.
          </p>
        ) : null}

        <div className="button-row">
          <button
            type="button"
            onClick={onCommit}
            disabled={busy || !readiness.ready}
            aria-describedby={readiness.ready ? undefined : "commit-reason"}
          >
            {busy ? "Committing…" : "Commit snapshot"}
          </button>
          <button type="button" className="button--secondary" onClick={onStartOver}>
            Start over
          </button>
        </div>
        {!readiness.ready ? (
          <p className="small field__error" id="commit-reason">
            {readiness.reason}
          </p>
        ) : null}
      </section>

      <details className="panel">
        <summary>Column mapping</summary>
        <p className="small muted">
          Columns were detected from the header row. Change a mapping and re-parse
          to use different columns; the file is not re-uploaded.
        </p>
        <div className="field-grid">
          {LOGICAL_FIELDS.map((field) => (
            <div className="field" key={field.key}>
              <label htmlFor={`mapping-${field.key}`}>
                {field.label}
                {REQUIRED_FIELDS.includes(field.key) ? " (required)" : ""}
              </label>
              <select
                id={`mapping-${field.key}`}
                value={mapping[field.key] ?? ""}
                onChange={(event) =>
                  setMapping({ ...mapping, [field.key]: event.target.value })
                }
              >
                <option value="">Not mapped</option>
                {preview.columns.map((column) => (
                  <option key={column} value={column}>
                    {column}
                  </option>
                ))}
              </select>
            </div>
          ))}
        </div>
        {missingMapped.length > 0 ? (
          <p className="field__error">
            Map these required columns first: {missingMapped.join(", ")}.
          </p>
        ) : null}
        <div className="button-row">
          <button
            type="button"
            className="button--secondary"
            disabled={busy || missingMapped.length > 0}
            onClick={() => onReparse(mapping)}
          >
            Re-parse with this mapping
          </button>
        </div>
      </details>
    </>
  );
}

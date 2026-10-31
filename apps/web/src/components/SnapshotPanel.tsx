import { useState } from "react";
import type { RecordChange, RecordItem, Snapshot } from "../types";
import { correctSnapshot, ApiFailure } from "../api";
import {
  formatTimestamp,
  formatMoney,
  priceToMinor,
  shortId,
} from "../inventory";
import { ProvenanceList, SpecList } from "./Badges";

interface SnapshotPanelProps {
  snapshot: Snapshot;
  isLatest: boolean;
  onCorrected: (snapshot: Snapshot) => void;
  onView: (id: string) => void;
}

interface Draft {
  quantity: string;
  price: string;
  note: string;
}

export function SnapshotPanel({
  snapshot,
  isLatest,
  onCorrected,
  onView,
}: SnapshotPanelProps) {
  const [editing, setEditing] = useState<string | null>(null);
  const [draft, setDraft] = useState<Draft | null>(null);
  const [pending, setPending] = useState(false);
  const [problem, setProblem] = useState<string | null>(null);
  const [retry, setRetry] = useState<(() => void) | null>(null);

  function open(record: RecordItem) {
    setEditing(record.id);
    setProblem(null);
    setDraft({
      quantity: String(record.quantity),
      price: (record.priceMinor / 100).toFixed(2),
      note: "",
    });
  }

  async function save(record: RecordItem) {
    if (!draft) {
      return;
    }
    const quantity = Number(draft.quantity);
    if (!/^\d+$/.test(draft.quantity.trim()) || !Number.isSafeInteger(quantity)) {
      setProblem("Quantity must be a whole number of zero or more.");
      return;
    }
    const priceMinor = priceToMinor(draft.price);
    if (priceMinor === null) {
      setProblem("Price must be a number with at most two decimals.");
      return;
    }
    const change: RecordChange = { recordId: record.id, quantity, priceMinor };
    const request = {
      note: draft.note.trim() === "" ? undefined : draft.note.trim(),
      changes: [change],
    };
    setPending(true);
    setProblem(null);
    setRetry(null);
    try {
      const corrected = await correctSnapshot(snapshot.id, request);
      onCorrected(corrected);
      setEditing(null);
      setDraft(null);
    } catch (error) {
      const message =
        error instanceof ApiFailure
          ? `${error.message} (${error.code})`
          : "The correction could not be saved. Check the service and retry.";
      setProblem(message);
      setRetry(() => () => void save(record));
    } finally {
      setPending(false);
    }
  }

  return (
    <section className="panel" aria-labelledby="snapshot-title">
      <h2 className="panel__title" id="snapshot-title" tabIndex={-1}>
        Committed snapshot · revision {snapshot.revision}
      </h2>
      <p className="lede">
        The snapshot is immutable. A later correction creates a new revision and
        keeps this one readable.
      </p>
      {!isLatest ? (
        <p className="revision-note">
          You are viewing an earlier revision. Corrections from here would create a
          new branch; the newest revision is available in history below.
        </p>
      ) : null}
      <dl className="record-meta">
        <div>
          <dt>Snapshot ID</dt>
          <dd className="mono" title={snapshot.id}>
            {shortId(snapshot.id)}
          </dd>
        </div>
        <div>
          <dt>Committed</dt>
          <dd>
            <time dateTime={snapshot.createdAt}>{formatTimestamp(snapshot.createdAt)}</time>
          </dd>
        </div>
        <div>
          <dt>Currency</dt>
          <dd>{snapshot.currency}</dd>
        </div>
        <div>
          <dt>Source file</dt>
          <dd>
            {snapshot.source.filename}
            <span className="small muted mono"> · sha256 {snapshot.source.sha256.slice(0, 12)}…</span>
          </dd>
        </div>
        <div>
          <dt>Parent revision</dt>
          <dd>
            {snapshot.parentSnapshotId ? (
              <button
                type="button"
                className="link-button"
                onClick={() => onView(snapshot.parentSnapshotId as string)}
              >
                {shortId(snapshot.parentSnapshotId)}
              </button>
            ) : (
              "None (first revision)"
            )}
          </dd>
        </div>
        <div>
          <dt>Note</dt>
          <dd>{snapshot.note ?? "—"}</dd>
        </div>
      </dl>
      <dl className="summary-grid" style={{ marginTop: "var(--space-4)" }}>
        <div>
          <dt>Records</dt>
          <dd>{snapshot.totals.recordCount}</dd>
        </div>
        <div>
          <dt>Units in stock</dt>
          <dd>{snapshot.totals.unitCount}</dd>
        </div>
        <div>
          <dt>Stock value</dt>
          <dd>{formatMoney(snapshot.totals.valueMinor, snapshot.currency)}</dd>
        </div>
      </dl>

      {problem ? (
        <div className="alert" role="alert">
          <h2>Correction failed</h2>
          <p>{problem}</p>
          {retry ? (
            <div className="button-row">
              <button type="button" onClick={retry} disabled={pending}>
                Retry correction
              </button>
            </div>
          ) : null}
        </div>
      ) : null}

      <div className="table-wrap">
        <table className="responsive-table">
          <caption>Committed stock records with provenance and unknown specs</caption>
          <thead>
            <tr>
              <th scope="col">Part</th>
              <th scope="col">Category</th>
              <th scope="col">Model</th>
              <th scope="col">Stock</th>
              <th scope="col">Price</th>
              <th scope="col">Specifications</th>
              <th scope="col">Provenance</th>
              <th scope="col">Action</th>
            </tr>
          </thead>
          <tbody>
            {snapshot.records.map((record) => (
              <tr key={record.id}>
                <td data-label="Part">
                  <span className="mono">{record.sku}</span>
                  <span className="small muted"> · lot {record.lot}</span>
                </td>
                <td data-label="Category">{record.category}</td>
                <td data-label="Model">
                  {record.modelRaw}
                  {record.modelNormalized !== record.modelRaw ? (
                    <span className="small muted">
                      {" "}
                      → {record.modelNormalized}
                    </span>
                  ) : null}
                  <span className="small muted">
                    {" "}
                    · match {record.match.confidence}
                  </span>
                </td>
                <td data-label="Stock">{record.quantity}</td>
                <td data-label="Price">
                  {formatMoney(record.priceMinor, record.currency)}
                  <span className="small muted">
                    {" "}
                    · line {formatMoney(record.priceMinor * record.quantity, record.currency)}
                  </span>
                </td>
                <td data-label="Specifications">
                  <SpecList specs={record.specs} />
                </td>
                <td data-label="Provenance">
                  <details className="details-block">
                    <summary>Show provenance</summary>
                    <ProvenanceList provenance={record.provenance} match={record.match} />
                  </details>
                </td>
                <td data-label="Action">
                  <button
                    type="button"
                    className="button--secondary"
                    aria-expanded={editing === record.id}
                    aria-controls={editing === record.id ? `edit-${record.id}` : undefined}
                    onClick={() => (editing === record.id ? setEditing(null) : open(record))}
                  >
                    Adjust stock or price
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {editing !== null && draft
        ? (() => {
            const record = snapshot.records.find((entry) => entry.id === editing);
            if (!record) {
              return null;
            }
            return (
              <form
                className="correction-panel"
                id={`edit-${record.id}`}
                onSubmit={(event) => {
                  event.preventDefault();
                  void save(record);
                }}
              >
                <h3>Create a new revision from {record.sku}</h3>
                <p className="small">
                  This never edits revision {snapshot.revision}. It creates revision{" "}
                  {snapshot.revision + 1} with new provenance for the changed fields.
                </p>
                <div className="field-grid">
                  <div className="field">
                    <label htmlFor={`edit-quantity-${record.id}`}>Quantity</label>
                    <input
                      id={`edit-quantity-${record.id}`}
                      inputMode="numeric"
                      autoFocus
                      value={draft.quantity}
                      onChange={(event) => setDraft({ ...draft, quantity: event.target.value })}
                    />
                  </div>
                  <div className="field">
                    <label htmlFor={`edit-price-${record.id}`}>
                      Sale price ({record.currency})
                    </label>
                    <input
                      id={`edit-price-${record.id}`}
                      inputMode="decimal"
                      value={draft.price}
                      onChange={(event) => setDraft({ ...draft, price: event.target.value })}
                    />
                  </div>
                  <div className="field">
                    <label htmlFor={`edit-note-${record.id}`}>Revision note</label>
                    <input
                      id={`edit-note-${record.id}`}
                      value={draft.note}
                      placeholder="For example: stock recount"
                      onChange={(event) => setDraft({ ...draft, note: event.target.value })}
                    />
                  </div>
                </div>
                <div className="button-row">
                  <button type="submit" disabled={pending}>
                    {pending ? "Saving revision…" : "Save new revision"}
                  </button>
                  <button
                    type="button"
                    className="button--secondary"
                    onClick={() => {
                      setEditing(null);
                      setDraft(null);
                      setProblem(null);
                    }}
                  >
                    Cancel
                  </button>
                </div>
              </form>
            );
          })()
        : null}
    </section>
  );
}

import { useEffect, useRef, useState } from "react";
import { sampleCsvUrl } from "../api";

interface ImportFormProps {
  file: File | null;
  currency: string;
  busy: boolean;
  onFile: (file: File | null) => void;
  onCurrency: (currency: string) => void;
  onSubmit: () => void;
}

const MAX_BYTES = 10 * 1024 * 1024;
const COMMON_CURRENCIES = ["USD", "EUR", "CNY", "JPY", "GBP", "CAD", "AUD"];

export function ImportForm({
  file,
  currency,
  busy,
  onFile,
  onCurrency,
  onSubmit,
}: ImportFormProps) {
  const [fileError, setFileError] = useState<string | null>(null);
  const fileInput = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (!file && fileInput.current) {
      fileInput.current.value = "";
    }
  }, [file]);

  function choose(next: File | null) {
    if (!next) {
      onFile(null);
      setFileError(null);
      return;
    }
    if (next.size > MAX_BYTES) {
      setFileError(
        `That file is ${next.size} bytes. CSV imports must be 10 MiB or smaller.`,
      );
      onFile(null);
      return;
    }
    setFileError(null);
    onFile(next);
  }

  return (
    <section className="panel" aria-labelledby="import-title">
      <h2 className="panel__title" id="import-title">
        Import a CSV
      </h2>
      <p className="lede">
        One batch uses one currency. Prices are stored in minor units (for example
        189.00 becomes 18900). Specifications use <code>key=value</code> pairs
        separated by semicolons, such as <code>socket=AM5;tdp=105W</code>.
      </p>
      <div className="field-grid">
        <div className="field">
          <label htmlFor="inventory-file">Inventory CSV (UTF-8)</label>
          <input
            ref={fileInput}
            id="inventory-file"
            name="file"
            type="file"
            accept=".csv,text/csv"
            aria-invalid={fileError ? true : undefined}
            aria-describedby={`file-hint${fileError ? " file-error" : ""}`}
            disabled={busy}
            onChange={(event) => choose(event.target.files?.[0] ?? null)}
          />
          <p className="field__hint" id="file-hint">
            {file
              ? `Selected ${file.name} (${file.size} bytes).`
              : "Choose a CSV with at least sku, category, model, quantity and price columns."}
          </p>
          {fileError ? (
            <p className="field__error" id="file-error" role="alert">
              {fileError}
            </p>
          ) : null}
        </div>
        <div className="field">
          <label htmlFor="inventory-currency">Batch currency</label>
          <input
            id="inventory-currency"
            name="currency"
            list="currency-options"
            value={currency}
            maxLength={3}
            autoComplete="off"
            disabled={busy}
            aria-describedby="currency-hint"
            onChange={(event) => onCurrency(event.target.value.toUpperCase())}
          />
          <datalist id="currency-options">
            {COMMON_CURRENCIES.map((code) => (
              <option key={code} value={code} />
            ))}
          </datalist>
          <p className="field__hint" id="currency-hint">
            A three-letter ISO 4217 code. Every row must match this currency.
          </p>
        </div>
      </div>
      <div className="button-row">
        <button type="button" onClick={onSubmit} disabled={busy}>
          {busy ? "Parsing…" : "Preview CSV"}
        </button>
        <a
          className="button button--secondary"
          href={sampleCsvUrl}
          download
          aria-describedby="sample-licence"
        >
          Download sample CSV
        </a>
      </div>
      <p className="small muted" id="sample-licence">
        The sample is one real AM5 build. Its compatibility facts were checked on
        2026-10-31 against the manufacturer pages recorded in the file's
        <code> sourceUrl</code> column. Sample data is CC BY 4.0 (see NOTICE);
        its quantity and price are operator-entered stock values, not market data.
      </p>
      <p className="privacy-note">
        Files are sent only to this local RigCraft service. CSV contents and
        optional cost fields are not written to logs.
      </p>
      <p className="small muted">
        Need a row removed from the file? Preview first, then choose Exclude or
        correct the row before committing.
      </p>
    </section>
  );
}

import { useEffect, useRef, useState, type FormEvent } from "react";
import type {
  CatalogDataset,
  CatalogPartDetail,
  CatalogReport,
  CatalogSearchResult,
} from "../types";
import {
  ApiFailure,
  getCatalogPart,
  listCatalogDatasets,
  removeCatalogDataset,
  searchCatalog,
  setCatalogDatasetActive,
  uploadCatalog,
} from "../api";
import { CATEGORIES, formatTimestamp } from "../inventory";
import { plural } from "../plural";
import { ProvenanceList, SpecList } from "./Badges";
import { SiteHeader } from "./SiteHeader";

interface ProblemRow {
  row: number;
  field: string;
  code: string;
  message: string;
  suggestion?: string;
}

interface Problem {
  title: string;
  message: string;
  code?: string;
  rows: ProblemRow[];
  specKeys?: string[];
}

function readText(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result ?? ""));
    reader.onerror = () => reject(reader.error);
    reader.readAsText(file);
  });
}

function describeFailure(error: unknown): Problem {
  if (error instanceof ApiFailure) {
    const rows = (error.details?.rows ?? []).filter(
      (entry): entry is ProblemRow => "field" in entry,
    );
    return {
      title: error.status === 422 ? "Catalog refused" : "Upload failed",
      message: error.message,
      code: error.code,
      rows,
      specKeys: error.details?.specKeys,
    };
  }
  return {
    title: "Upload failed",
    message:
      error instanceof Error
        ? error.message
        : "The local service could not be reached. Check that it is running.",
    code: "network_error",
    rows: [],
  };
}

export function CatalogPage() {
  const [file, setFile] = useState<File | null>(null);
  const [acknowledged, setAcknowledged] = useState(false);
  const [busy, setBusy] = useState(false);
  const [report, setReport] = useState<CatalogReport | null>(null);
  const [problem, setProblem] = useState<Problem | null>(null);
  const [datasets, setDatasets] = useState<CatalogDataset[]>([]);
  const [datasetsError, setDatasetsError] = useState<string | null>(null);
  const [pendingRemove, setPendingRemove] = useState<string | null>(null);
  const [busyDataset, setBusyDataset] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [category, setCategory] = useState("");
  const [searching, setSearching] = useState(false);
  const [searchResult, setSearchResult] = useState<CatalogSearchResult | null>(null);
  const [searchProblem, setSearchProblem] = useState<string | null>(null);
  const [selectedPart, setSelectedPart] = useState<CatalogPartDetail | null>(null);
  const [live, setLive] = useState("No catalog uploaded in this session.");

  const alertRef = useRef<HTMLDivElement>(null);
  const reportRef = useRef<HTMLHeadingElement>(null);
  const registryRef = useRef<HTMLHeadingElement>(null);
  const partRef = useRef<HTMLHeadingElement>(null);
  const statusRef = useRef<HTMLDivElement>(null);

  async function refreshRegistry() {
    try {
      const next = await listCatalogDatasets();
      setDatasets(next);
      setDatasetsError(null);
    } catch (error) {
      setDatasetsError(
        error instanceof ApiFailure ? error.message : "The dataset registry is unavailable.",
      );
    }
  }

  useEffect(() => {
    void refreshRegistry();
  }, []);

  useEffect(() => {
    if (problem) {
      alertRef.current?.focus();
    }
  }, [problem]);

  useEffect(() => {
    if (busy) {
      statusRef.current?.focus();
    }
  }, [busy]);

  async function upload() {
    if (!file) {
      setProblem({
        title: "Choose a file",
        message: "Select a rigcraft-catalog JSON document before importing.",
        rows: [],
      });
      return;
    }
    if (!acknowledged) {
      setProblem({
        title: "Confirm the licence",
        message:
          "Confirm that you have the right to use this catalog data before importing it.",
        rows: [],
      });
      return;
    }
    setBusy(true);
    setProblem(null);
    setReport(null);
    setSelectedPart(null);
    setLive("Reading and validating the catalog document…");
    let parsed: unknown;
    try {
      parsed = JSON.parse(await readText(file)) as unknown;
    } catch {
      setBusy(false);
      setProblem({
        title: "Not valid JSON",
        message: "The selected file could not be parsed as JSON. Nothing was uploaded.",
        code: "invalid_json",
        rows: [],
      });
      setLive("Upload failed: the file is not JSON.");
      return;
    }
    try {
      const result = await uploadCatalog(parsed, acknowledged);
      setReport(result);
      setFile(null);
      setAcknowledged(false);
      setLive(
        `Catalog accepted: ${plural(result.partCount, "part")} from ${result.dataset.name}.`,
      );
      window.setTimeout(() => reportRef.current?.focus(), 0);
      await refreshRegistry();
    } catch (error) {
      const next = describeFailure(error);
      setProblem(next);
      setLive("Catalog refused. Nothing was stored; review the named rows.");
    } finally {
      setBusy(false);
    }
  }

  async function toggleActive(dataset: CatalogDataset) {
    setBusyDataset(dataset.id);
    setPendingRemove(null);
    try {
      const updated = await setCatalogDatasetActive(dataset.id, !dataset.active);
      setLive(
        `${updated.name} is now ${updated.active ? "active" : "inactive"} for matching and search.`,
      );
      await refreshRegistry();
    } catch (error) {
      setProblem(describeFailure(error));
    } finally {
      setBusyDataset(null);
    }
  }

  async function remove(dataset: CatalogDataset) {
    setBusyDataset(dataset.id);
    try {
      await removeCatalogDataset(dataset.id);
      setPendingRemove(null);
      setLive(
        `${dataset.name} was removed. Inventory, snapshots, checks, recommendations and diagnoses were not touched.`,
      );
      await refreshRegistry();
    } catch (error) {
      setProblem(describeFailure(error));
    } finally {
      setBusyDataset(null);
    }
  }

  async function runSearch(event: FormEvent) {
    event.preventDefault();
    if (query.trim() === "") {
      setSearchProblem("Enter a SKU, MPN, model or word to search the active catalog.");
      setSearchResult(null);
      return;
    }
    setSearching(true);
    setSearchProblem(null);
    setSelectedPart(null);
    setLive("Searching the active catalog…");
    try {
      const result = await searchCatalog(query.trim(), category, 20, 0);
      setSearchResult(result);
      setLive(
        result.total === 0
          ? "No active catalog part matched that query."
          : `${plural(result.total, "catalog match", "catalog matches")}; showing ${plural(
              result.hits.length,
              "hit",
            )}.`,
      );
    } catch (error) {
      setSearchResult(null);
      setSearchProblem(
        error instanceof ApiFailure ? error.message : "The search could not run.",
      );
      setLive("Search failed.");
    } finally {
      setSearching(false);
    }
  }

  async function openPart(id: string) {
    try {
      const detail = await getCatalogPart(id);
      setSelectedPart(detail);
      setLive(`Showing stored part ${detail.part.sku} from ${detail.dataset.name}.`);
      window.setTimeout(() => partRef.current?.focus(), 0);
    } catch (error) {
      setSearchProblem(
        error instanceof ApiFailure ? error.message : "The part could not be opened.",
      );
    }
  }

  const removing = datasets.find((dataset) => dataset.id === pendingRemove) ?? null;
  const registryParts = datasets.reduce((total, dataset) => total + dataset.rowCount, 0);
  const activeDatasets = datasets.filter((dataset) => dataset.active).length;
  const queryReady = query.trim() !== "";

  return (
    <>
      <SiteHeader current="catalog" />
      <main className="page" id="main" tabIndex={-1}>
        <h1 className="page-title">Catalog</h1>
        <p className="lede">
          Load a hardware catalog you are licensed to use. It is stored in this
          instance with per-field provenance and can enrich an import where your own
          row leaves a fact out.
        </p>
        <p className="privacy-note">
          The catalog is your data, not RigCraft's. It is never bundled, fetched from
          the internet, redistributed or written to the repository, and its licence is
          shown exactly as you supplied it.
        </p>

        <div
          className="status-line"
          role="status"
          aria-live="polite"
          aria-busy={busy}
          data-busy={busy}
          tabIndex={-1}
          ref={statusRef}
        >
          <span>{live}</span>
        </div>

        {problem ? (
          <div className="alert" role="alert" tabIndex={-1} ref={alertRef}>
            <h2>{problem.title}</h2>
            <p>{problem.message}</p>
            {problem.code ? (
              <p className="small muted">
                Code: <span className="mono">{problem.code}</span>
              </p>
            ) : null}
            {problem.rows.length > 0 ? (
              <ul>
                {problem.rows.map((entry) => (
                  <li key={`${entry.row}-${entry.field}-${entry.code}`}>
                    Row {entry.row} · <strong>{entry.field}</strong> — {entry.message}
                    {entry.suggestion ? <> {entry.suggestion}</> : null}
                  </li>
                ))}
              </ul>
            ) : null}
            {problem.specKeys && problem.specKeys.length > 0 ? (
              <p className="small muted">
                Accepted specification keys: {problem.specKeys.join(", ")}.
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

        <section className="panel" aria-labelledby="upload-title">
          <div className="panel__head">
            <p className="caps-label" aria-hidden="true">
              Upload
            </p>
            <h2 className="panel__title" id="upload-title">
              Upload a catalog
            </h2>
          </div>
          <p className="small muted">
            One JSON document in the <span className="mono">rigcraft-catalog</span>{" "}
            version 1 format. The whole document is validated before anything is
            stored; a single bad row refuses the upload.
          </p>
          <form
            onSubmit={(event) => {
              event.preventDefault();
              void upload();
            }}
          >
            <div className="field-grid">
              <div className="field">
                <label htmlFor="catalog-file">Catalog JSON document</label>
                <input
                  id="catalog-file"
                  type="file"
                  accept=".json,application/json"
                  aria-describedby="catalog-file-hint"
                  onChange={(event) => {
                    setFile(event.target.files?.[0] ?? null);
                    setProblem(null);
                  }}
                />
                <p className="small muted" id="catalog-file-hint">
                  {file ? `Selected ${file.name}` : "No catalog file selected."}
                </p>
              </div>
            </div>
            <label className="choice" htmlFor="catalog-acknowledged">
              <input
                id="catalog-acknowledged"
                type="checkbox"
                checked={acknowledged}
                onChange={(event) => setAcknowledged(event.target.checked)}
              />
              <span>
                I have the right to use this data and will keep its licence
                obligations. RigCraft does not verify or redistribute it.
              </span>
            </label>
            <div className="button-row">
              <button type="submit" disabled={busy || !file || !acknowledged}>
                {busy ? "Validating…" : "Import catalog"}
              </button>
            </div>
          </form>

          {report ? (
            <div className="panel__section">
              <h3 tabIndex={-1} ref={reportRef}>
                Catalog accepted
              </h3>
              <p>
                <strong>{report.dataset.name}</strong> ·{" "}
                {report.replaced ? "replaced" : "added"} ·{" "}
                {plural(report.partCount, "part")} ·{" "}
                {Object.entries(report.categoryCounts)
                  .map(([key, value]) => `${value} ${key}`)
                  .join(" · ")}
              </p>
              <p className="small">
                Licence: <strong>{report.dataset.license}</strong> · version{" "}
                {report.dataset.version}
                {report.dataset.sourceUrl ? (
                  <>
                    {" "}
                    · <span className="tag">{report.dataset.sourceUrl}</span>
                  </>
                ) : null}
              </p>
              <p className="small muted">
                Provenance for applied facts will read{" "}
                <span className="mono">
                  catalog:{report.dataset.name}@{report.dataset.version}
                </span>
                .
              </p>
            </div>
          ) : null}
        </section>

        <section className="panel" aria-labelledby="registry-title">
          <div className="panel__head">
            <p className="caps-label" aria-hidden="true">
              Registry
            </p>
            <h2 className="panel__title" id="registry-title" tabIndex={-1} ref={registryRef}>
              Datasets
            </h2>
          </div>
          {datasetsError ? <p className="field__error">{datasetsError}</p> : null}
          {datasets.length > 0 ? (
            <dl className="console-stats">
              <div>
                <dt>Datasets</dt>
                <dd>{datasets.length}</dd>
              </div>
              <div>
                <dt>Parts</dt>
                <dd>{registryParts}</dd>
              </div>
              <div>
                <dt>Active</dt>
                <dd>{activeDatasets}</dd>
              </div>
            </dl>
          ) : null}
          {datasets.length === 0 ? (
            <p>No dataset is stored in this instance yet.</p>
          ) : (
            <div className="table-wrap">
              <table className="responsive-table catalog-table">
                <caption>Stored catalogs with their licence, version and counts</caption>
                <thead>
                  <tr>
                    <th scope="col">Dataset</th>
                    <th scope="col">Licence</th>
                    <th scope="col">Version</th>
                    <th scope="col">Parts</th>
                    <th scope="col">Categories</th>
                    <th scope="col">Active</th>
                    <th scope="col">Actions</th>
                  </tr>
                </thead>
                <tbody>
                  {datasets.map((dataset) => (
                    <tr key={dataset.id}>
                      <td data-label="Dataset">
                        <strong>{dataset.name}</strong>
                        <span className="badge badge--catalog">
                          <span className="badge__symbol" aria-hidden="true">
                            ▣
                          </span>
                          Catalog
                        </span>
                        {dataset.sourceUrl ? (
                          <span className="small muted">
                            {" "}
                            · <span className="tag">{dataset.sourceUrl}</span>
                          </span>
                        ) : null}
                        <span className="small muted">
                          {" "}
                          · imported{" "}
                          <time dateTime={dataset.importedAt}>
                            {formatTimestamp(dataset.importedAt)}
                          </time>
                        </span>
                      </td>
                      <td data-label="Licence">{dataset.license}</td>
                      <td data-label="Version" className="mono">
                        {dataset.version}
                      </td>
                      <td data-label="Parts" className="mono">
                        {dataset.rowCount}
                      </td>
                      <td data-label="Categories">
                        {Object.entries(dataset.categoryCounts)
                          .map(([key, value]) => `${key} ${value}`)
                          .join(", ") || "—"}
                      </td>
                      <td data-label="Active">
                        <span className={`badge badge--${dataset.active ? "valid" : "unknown"}`}>
                          <span className="badge__symbol" aria-hidden="true">
                            {dataset.active ? "✓" : "?"}
                          </span>
                          {dataset.active ? "Active" : "Inactive"}
                        </span>
                      </td>
                      <td data-label="Actions">
                        <div className="button-row">
                          <button
                            type="button"
                            className="button--secondary"
                            disabled={busyDataset === dataset.id}
                            onClick={() => void toggleActive(dataset)}
                          >
                            {dataset.active ? "Deactivate" : "Activate"}
                          </button>
                          <button
                            type="button"
                            className="button--danger"
                            disabled={busyDataset === dataset.id}
                            aria-expanded={pendingRemove === dataset.id}
                            onClick={() => setPendingRemove(dataset.id)}
                          >
                            Remove
                          </button>
                        </div>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}

          {removing ? (
            <div
              className="alert"
              role="group"
              aria-labelledby="remove-title"
            >
              <h3 id="remove-title">Remove {removing.name}?</h3>
              <p>
                This removes {plural(removing.rowCount, "part")} from this instance only.
                Inventory, snapshots, compatibility checks, recommendations and
                diagnoses are not affected.
              </p>
              <div className="button-row">
                <button
                  type="button"
                  className="button--danger"
                  disabled={busyDataset === removing.id}
                  onClick={() => void remove(removing)}
                >
                  {busyDataset === removing.id ? "Removing…" : "Confirm removal"}
                </button>
                <button
                  type="button"
                  className="button--secondary"
                  onClick={() => setPendingRemove(null)}
                >
                  Cancel
                </button>
              </div>
            </div>
          ) : null}
        </section>

        <section className="panel" aria-labelledby="search-title">
          <div className="panel__head">
            <p className="caps-label" aria-hidden="true">
              Search
            </p>
            <h2 className="panel__title" id="search-title">
              Search
            </h2>
          </div>
          <form onSubmit={(event) => void runSearch(event)}>
            <div className="field-grid">
              <div className="field">
                <label htmlFor="catalog-query">SKU, MPN, model or word</label>
                <input
                  id="catalog-query"
                  value={query}
                  aria-describedby="catalog-search-hint"
                  onChange={(event) => setQuery(event.target.value)}
                />
              </div>
              <div className="field">
                <label htmlFor="catalog-category">Category (optional)</label>
                <select
                  id="catalog-category"
                  value={category}
                  onChange={(event) => setCategory(event.target.value)}
                >
                  <option value="">Any category</option>
                  {CATEGORIES.map((entry) => (
                    <option key={entry} value={entry}>
                      {entry}
                    </option>
                  ))}
                </select>
              </div>
            </div>
            <p className="field__hint" id="catalog-search-hint">
              {queryReady
                ? "Only active datasets are searched; an exact SKU or MPN ranks first."
                : "Enter a SKU, MPN, model or word to enable search."}
            </p>
            <div className="button-row">
              <button
                type="submit"
                disabled={searching || !queryReady}
                aria-describedby="catalog-search-hint"
              >
                {searching ? "Searching…" : "Search catalog"}
              </button>
            </div>
          </form>
          {searchProblem ? <p className="field__error">{searchProblem}</p> : null}
          {searchResult ? (
            searchResult.total === 0 ? (
              <p>No active catalog part matched “{searchResult.query}”.</p>
            ) : (
              <>
                <p className="small muted">
                  {plural(searchResult.total, "ranked hit")}; showing{" "}
                  {searchResult.hits.length}. Exact SKU or MPN first, then model
                  prefix, then token overlap.
                </p>
                <div className="table-wrap">
                  <table className="responsive-table catalog-table">
                    <caption>Ranked catalog search results</caption>
                    <thead>
                      <tr>
                        <th scope="col">Rank</th>
                        <th scope="col">SKU</th>
                        <th scope="col">MPN</th>
                        <th scope="col">Category</th>
                        <th scope="col">Model</th>
                        <th scope="col">Dataset</th>
                        <th scope="col">Licence</th>
                        <th scope="col">Version</th>
                        <th scope="col">Open</th>
                      </tr>
                    </thead>
                    <tbody>
                      {searchResult.hits.map((hit, index) => (
                        <tr key={hit.part.id}>
                          <td data-label="Rank" className="mono">
                            {index + 1}
                          </td>
                          <td data-label="SKU" className="mono">
                            {hit.part.sku}
                          </td>
                          <td data-label="MPN" className="mono">
                            {hit.part.externalId ?? "—"}
                          </td>
                          <td data-label="Category">{hit.part.category}</td>
                          <td data-label="Model">{hit.part.model}</td>
                          <td data-label="Dataset">
                            <strong>{hit.dataset.name}</strong>
                            <span className="badge badge--catalog">
                              <span className="badge__symbol" aria-hidden="true">
                                ▣
                              </span>
                              Catalog
                            </span>
                          </td>
                          <td data-label="Licence">{hit.dataset.license}</td>
                          <td data-label="Version" className="mono">
                            {hit.dataset.version}
                          </td>
                          <td data-label="Open">
                            <button
                              type="button"
                              className="button--secondary"
                              onClick={() => void openPart(hit.part.id)}
                            >
                              View part
                            </button>
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </>
            )
          ) : null}

          {selectedPart ? (
            <div className="panel__section">
              <h3 tabIndex={-1} ref={partRef}>
                {selectedPart.part.model}
              </h3>
              <p className="small">
                <span className="mono">{selectedPart.part.sku}</span> ·{" "}
                {selectedPart.part.category} · {selectedPart.dataset.name} ·{" "}
                {selectedPart.part.manufacturer ?? "Manufacturer not recorded"}
              </p>
              <SpecList specs={selectedPart.part.specs} />
              <ProvenanceList provenance={selectedPart.provenance} />
            </div>
          ) : null}
        </section>
      </main>
    </>
  );
}

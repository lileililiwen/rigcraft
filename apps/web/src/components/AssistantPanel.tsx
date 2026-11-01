import { useEffect, useRef, useState } from "react";
import type { AiProposalResponse, ProposedField, RecordItem, Snapshot } from "../types";
import { ApiFailure, parseWithAssistant } from "../api";
import { USE_CASES, parseBudgetMinor, plural } from "../recommend";
import { fieldIsEdited, proposalToForm, recordLabel, sameSkus, skusOf, sourceLabel } from "../ai";

export interface AppliedProposal {
  budgetText: string;
  useCase: string;
  lockedSkus: string[];
  excludedSkus: string[];
}

interface Props {
  snapshot: Snapshot;
  disabled?: boolean;
  onApply: (applied: AppliedProposal) => void;
  onSkip: () => void;
  announce: (message: string) => void;
}

interface Problem {
  title: string;
  message: string;
  code?: string;
}

interface Edits {
  budgetText: string;
  useCase: string;
  locked: Record<string, boolean>;
  excluded: Record<string, boolean>;
}

type Phase = "idle" | "parsing" | "proposed" | "failed";

function emptyEdits(): Edits {
  return { budgetText: "", useCase: "", locked: {}, excluded: {} };
}

function sourceLine(field: ProposedField<unknown>, edited: boolean): string {
  const base = sourceLabel(field);
  return edited ? `${base} · Edited by you` : base;
}

/**
 * The optional BYOK panel. It asks the provider to interpret one sentence and
 * shows a per-field proposal; nothing runs until the operator confirms, and the
 * key stays in component state for this session only.
 */
export function AssistantPanel({ snapshot, disabled, onApply, onSkip, announce }: Props) {
  const [key, setKey] = useState("");
  const [baseUrl, setBaseUrl] = useState("");
  const [model, setModel] = useState("");
  const [sentence, setSentence] = useState("");
  const [includeSummary, setIncludeSummary] = useState(false);
  const [consent, setConsent] = useState(false);
  const [phase, setPhase] = useState<Phase>("idle");
  const [response, setResponse] = useState<AiProposalResponse | null>(null);
  const [baseline, setBaseline] = useState<Edits>(emptyEdits);
  const [edits, setEdits] = useState<Edits>(emptyEdits);
  const [problem, setProblem] = useState<Problem | null>(null);
  const [localError, setLocalError] = useState<string | null>(null);
  const [confirmed, setConfirmed] = useState(false);
  const proposalRef = useRef<HTMLHeadingElement>(null);
  const errorRef = useRef<HTMLDivElement>(null);
  const budgetRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    setResponse(null);
    setPhase("idle");
    setEdits(emptyEdits());
    setBaseline(emptyEdits());
    setConfirmed(false);
    setProblem(null);
    setLocalError(null);
  }, [snapshot.id]);

  useEffect(() => {
    if (phase === "proposed") {
      proposalRef.current?.focus();
    }
  }, [phase]);

  useEffect(() => {
    if (problem) {
      errorRef.current?.focus();
    }
  }, [problem]);

  const busy = phase === "parsing" || disabled === true;
  const noKey = key.trim().length === 0;
  const missing = noKey
    ? "Enter your own provider key to use the assistant."
    : baseUrl.trim().length === 0
      ? "Enter the provider base URL."
      : model.trim().length === 0
        ? "Enter the model name."
        : sentence.trim().length === 0
          ? "Describe the build in one sentence."
          : !consent
            ? "Tick the consent box to send the sentence."
            : "";

  async function parse() {
    if (missing) {
      return;
    }
    setPhase("parsing");
    setProblem(null);
    setLocalError(null);
    setResponse(null);
    setConfirmed(false);
    announce("Asking your provider to interpret the sentence…");
    try {
      const next = await parseWithAssistant({
        text: sentence,
        snapshotId: snapshot.id,
        apiKey: key,
        baseUrl,
        model,
        includeInventorySummary: includeSummary,
      });
      const seeded = proposalToForm(next);
      setResponse(next);
      setBaseline({
        budgetText: seeded.budgetText,
        useCase: seeded.useCase,
        locked: Object.fromEntries(
          snapshot.records.map((record) => [record.id, seeded.lockedSkus.includes(record.sku)]),
        ),
        excluded: Object.fromEntries(
          snapshot.records.map((record) => [record.id, seeded.excludedSkus.includes(record.sku)]),
        ),
      });
      setEdits({
        budgetText: seeded.budgetText,
        useCase: seeded.useCase,
        locked: Object.fromEntries(
          snapshot.records.map((record) => [record.id, seeded.lockedSkus.includes(record.sku)]),
        ),
        excluded: Object.fromEntries(
          snapshot.records.map((record) => [record.id, seeded.excludedSkus.includes(record.sku)]),
        ),
      });
      setPhase("proposed");
      announce(
        `Proposal ready with ${plural(
          next.ambiguities.length,
          "item",
        )} to confirm. Review every field before confirming.`,
      );
    } catch (error) {
      const detail =
        error instanceof ApiFailure
          ? { title: "The assistant could not interpret that", message: error.message, code: error.code }
          : {
              title: "The assistant could not be reached",
              message:
                error instanceof Error
                  ? error.message
                  : "The local service could not be reached. Your sentence is kept.",
              code: "network_error",
            };
      setProblem(detail);
      setPhase("failed");
      announce(
        `${detail.title}. Your sentence and form values are kept. Retry, or continue without AI.`,
      );
    }
  }

  function toggle(kind: "locked" | "excluded", record: RecordItem) {
    setEdits((current) => ({
      ...current,
      [kind]: { ...current[kind], [record.id]: !current[kind][record.id] },
      ...(kind === "locked"
        ? { excluded: { ...current.excluded, [record.id]: false } }
        : { locked: { ...current.locked, [record.id]: false } }),
    }));
  }

  function confirm() {
    const budgetMinor = parseBudgetMinor(edits.budgetText);
    if (budgetMinor === null) {
      setLocalError("Enter a positive budget with at most two decimals, for example 1200.00.");
      announce("Confirmation blocked: the budget is not valid.");
      budgetRef.current?.focus();
      return;
    }
    if (!USE_CASES.some((entry) => entry.value === edits.useCase)) {
      setLocalError("Choose a supported primary use: gaming, office or creation.");
      announce("Confirmation blocked: choose a primary use.");
      return;
    }
    const lockedSkus = skusOf(snapshot, edits.locked);
    const excludedSkus = skusOf(snapshot, edits.excluded).filter(
      (sku) => !lockedSkus.includes(sku),
    );
    setLocalError(null);
    setConfirmed(true);
    announce(
      "Proposal confirmed and applied to the form. Press Generate recommendations to run the deterministic search.",
    );
    onApply({
      budgetText: edits.budgetText.trim(),
      useCase: edits.useCase,
      lockedSkus,
      excludedSkus,
    });
  }

  function cancel() {
    setResponse(null);
    setEdits(emptyEdits());
    setBaseline(emptyEdits());
    setPhase("idle");
    setConfirmed(false);
    setLocalError(null);
    announce("Proposal discarded. Nothing was applied and the sentence is kept.");
  }

  function partPickers(
    kind: "locked" | "excluded",
    field: ProposedField<string[]>,
    legend: string,
  ) {
    const selected = snapshot.records.filter((record) => edits[kind][record.id]).length;
    const edited = !sameSkus(skusOf(snapshot, edits[kind]), skusOf(snapshot, baseline[kind]));
    return (
      <fieldset className="proposal-parts">
        <legend>{legend}</legend>
        <details className="details-block">
          <summary>
            Choose parts ({plural(selected, "part")} selected) ·{" "}
            {sourceLine(field, edited)}
          </summary>
          <ul className="part-list">
            {snapshot.records.map((record) => (
              <li className="part-option part-option--none" key={`${kind}-${record.id}`}>
                <input
                  type="checkbox"
                  id={`assistant-${kind}-${record.id}`}
                  checked={edits[kind][record.id] === true}
                  disabled={busy}
                  onChange={() => toggle(kind, record)}
                />
                <label htmlFor={`assistant-${kind}-${record.id}`}>
                  {kind === "locked" ? "Keep" : "Exclude"} {recordLabel(record)}
                </label>
              </li>
            ))}
          </ul>
        </details>
      </fieldset>
    );
  }

  return (
    <details className="panel assistant" aria-labelledby="assistant-title">
      <summary className="assistant__summary">
        <h2 className="panel__title" id="assistant-title">
          2. Ask in your own words (optional)
        </h2>
      </summary>
      <div className="assistant__body">
        <p className="lede">
          Describe the build in one sentence. Your own provider turns it into a
          proposed request that you review and confirm; the deterministic search
          below still checks every part, price and compatibility rule.
        </p>

        {noKey ? (
          <div className="diagnostic-block diagnostic-block--unknown">
            <h3>No key configured</h3>
            <p>
              The assistant needs your own provider key and is switched off until you
              enter one. Everything on this page — budget, constraints, generate,
              compare and export — works without it. Use{" "}
              <strong>Continue without AI</strong> below to go straight to the form.
            </p>
          </div>
        ) : null}

        {/* A form owner keeps the browser from warning that the password field is
            outside a form. It never submits: the interpret action is a button. */}
        <form
          className="assistant-transport"
          aria-label="Assistant provider settings"
          onSubmit={(event) => event.preventDefault()}
        >
          <div className="field-grid">
            <div className="field">
              <label htmlFor="assistant-key">Provider key</label>
              <input
                id="assistant-key"
                type="password"
                autoComplete="off"
                spellCheck={false}
                value={key}
                disabled={busy}
                aria-describedby="assistant-key-hint"
                onChange={(event) => setKey(event.target.value)}
              />
              <p className="field__hint" id="assistant-key-hint">
                Kept in this tab for this session only. Never stored, logged, exported
                or shown again.
              </p>
            </div>
            <div className="field">
              <label htmlFor="assistant-base-url">Provider base URL</label>
              <input
                id="assistant-base-url"
                inputMode="url"
                spellCheck={false}
                placeholder="https://provider.example/v1"
                value={baseUrl}
                disabled={busy}
                aria-describedby="assistant-base-url-hint"
                onChange={(event) => setBaseUrl(event.target.value)}
              />
              <p className="field__hint" id="assistant-base-url-hint">
                HTTPS, or plain HTTP only for a loopback model such as
                http://127.0.0.1:11434/v1. Redirects are not followed.
              </p>
            </div>
            <div className="field">
              <label htmlFor="assistant-model">Model name</label>
              <input
                id="assistant-model"
                spellCheck={false}
                value={model}
                disabled={busy}
                onChange={(event) => setModel(event.target.value)}
              />
              <p className="field__hint">Whatever your provider calls the model.</p>
            </div>
          </div>
        </form>

        <div className="assistant-disclosure">
          <h3>What leaves this machine</h3>
          <p>
            Your sentence, the snapshot currency ({snapshot.currency}) and the allowed
            use-case names go to the provider URL you typed. Your inventory is{" "}
            <strong>not</strong> sent unless you include it below.
          </p>
          <label className="choice" htmlFor="assistant-include-summary">
            <input
              type="checkbox"
              id="assistant-include-summary"
              checked={includeSummary}
              disabled={busy}
              onChange={(event) => setIncludeSummary(event.target.checked)}
            />
            <span>
              Include a part list: at most 50 records, only SKU, category, model, price
              and quantity.
            </span>
          </label>
          <p className="small muted">
            Your key is used for this request only. It is never saved, never written to
            a log or an export, and never sent anywhere except the URL above.
          </p>
        </div>

        <div className="field">
          <label htmlFor="assistant-sentence">Your sentence</label>
          <textarea
            id="assistant-sentence"
            rows={3}
            maxLength={8192}
            value={sentence}
            disabled={busy}
            placeholder="A $1200 gaming build from my stock, keep the board I already have."
            onChange={(event) => {
              setSentence(event.target.value);
              setConfirmed(false);
            }}
          />
        </div>

        <div className="choice">
          <input
            type="checkbox"
            id="assistant-consent"
            checked={consent}
            disabled={busy}
            onChange={(event) => setConsent(event.target.checked)}
          />
          <label htmlFor="assistant-consent">
            I understand this sends my sentence, the fields above and my key to the
            provider URL I entered.
          </label>
        </div>

        <div className="button-row">
          <button
            type="button"
            disabled={busy || missing !== ""}
            aria-describedby="assistant-parse-hint"
            onClick={() => void parse()}
          >
            {phase === "parsing" ? "Interpreting…" : "Interpret sentence"}
          </button>
          <button type="button" className="button--secondary" onClick={onSkip}>
            Continue without AI
          </button>
        </div>
        <p className="field__hint" id="assistant-parse-hint" aria-live="polite">
          {missing ||
            "Interpretation proposes fields only; it never runs the search or writes a price."}
        </p>

      {problem ? (
        <div className="alert" role="alert" tabIndex={-1} ref={errorRef}>
          <h3>{problem.title}</h3>
          <p>{problem.message}</p>
          {problem.code ? (
            <p className="small muted">
              Code: <span className="mono">{problem.code}</span>
            </p>
          ) : null}
          <p className="small">
            Your sentence, key and form values are kept. Nothing was saved.
          </p>
          <div className="button-row">
            <button type="button" onClick={() => void parse()} disabled={busy || missing !== ""}>
              Retry interpretation
            </button>
            <button type="button" className="button--secondary" onClick={onSkip}>
              Continue without AI
            </button>
          </div>
        </div>
      ) : null}

      {response ? (
        <div className="proposal">
          <h3 className="evidence-title" tabIndex={-1} ref={proposalRef}>
            Proposed request — review and confirm
          </h3>
          <p className="small muted">
            Provider: {response.provider.kind} · {response.provider.model} · contract{" "}
            {response.promptVersion}. Sent: your sentence, currency, allowed use
            cases
            {response.sent.inventoryRecords > 0
              ? ` and ${plural(response.sent.inventoryRecords, "inventory record")} limited to ${response.sent.inventoryFields.join(", ")}.`
              : "; no inventory record."}
          </p>

          <div className="proposal-grid">
            <div className="proposal-field">
              <label htmlFor="assistant-budget">
                Proposed total budget ({snapshot.currency})
              </label>
              <input
                id="assistant-budget"
                ref={budgetRef}
                inputMode="decimal"
                value={edits.budgetText}
                aria-describedby="assistant-budget-source"
                onChange={(event) =>
                  setEdits((current) => ({ ...current, budgetText: event.target.value }))
                }
              />
              <p className="field__hint" id="assistant-budget-source">
                {sourceLine(
                  response.proposal.budgetMinor,
                  fieldIsEdited(edits.budgetText, baseline.budgetText),
                )}
              </p>
            </div>

            <div className="proposal-field">
              <label htmlFor="assistant-use-case">Proposed primary use</label>
              <select
                id="assistant-use-case"
                value={edits.useCase}
                aria-describedby="assistant-use-case-source"
                onChange={(event) =>
                  setEdits((current) => ({ ...current, useCase: event.target.value }))
                }
              >
                <option value="">Choose a primary use</option>
                {USE_CASES.map((entry) => (
                  <option key={entry.value} value={entry.value}>
                    {entry.label}
                  </option>
                ))}
              </select>
              <p className="field__hint" id="assistant-use-case-source">
                {sourceLine(
                  response.proposal.useCase,
                  fieldIsEdited(edits.useCase, baseline.useCase),
                )}
              </p>
            </div>

            <div className="proposal-field">
              <p className="proposal-field__label">Currency</p>
              <p className="proposal-field__value">
                {response.proposal.currency.value ?? "Not established"}
              </p>
              <p className="field__hint">
                {sourceLabel(response.proposal.currency)}
              </p>
            </div>

            <div className="proposal-field">
              <p className="proposal-field__label">Inventory only</p>
              <p className="proposal-field__value">
                {response.proposal.inventoryOnly.value === null
                  ? "Not established"
                  : response.proposal.inventoryOnly.value
                    ? "Yes — snapshot only"
                    : "No — not available"}
              </p>
              <p className="field__hint">
                {sourceLabel(response.proposal.inventoryOnly)}
              </p>
            </div>
          </div>

          {partPickers("locked", response.proposal.lockedSkus, "Keep these parts (already owned)")}
          {partPickers("excluded", response.proposal.excludedSkus, "Exclude these parts")}

          {response.ambiguities.length > 0 ? (
            <div className="diagnostic-block diagnostic-block--unknown" role="group" aria-labelledby="assistant-ambiguity-title">
              <h3 id="assistant-ambiguity-title">
                Needs your confirmation ({response.ambiguities.length})
              </h3>
              <ul className="missing-list">
                {response.ambiguities.map((entry, index) => (
                  <li key={`${entry.field}-${entry.code}-${index}`}>
                    <span className="mono small">{entry.field}</span> {entry.message}
                  </li>
                ))}
              </ul>
            </div>
          ) : null}

          {response.ignoredFields.length > 0 ? (
            <p className="small muted">
              Discarded because they are outside this contract:{" "}
              <span className="mono">{response.ignoredFields.join(", ")}</span>.
            </p>
          ) : null}

          {localError ? (
            <p className="field__error" role="alert">
              {localError}
            </p>
          ) : null}

          <div className="button-row">
            <button type="button" onClick={confirm}>
              Confirm and apply to the form
            </button>
            <button type="button" className="button--secondary" onClick={cancel}>
              Cancel
            </button>
          </div>
          <p className="small muted">
            {confirmed
              ? "Confirmed and applied. Press Generate recommendations to run the deterministic search."
              : "Nothing has run yet. Confirming fills the form; you still press Generate."}
          </p>
        </div>
      ) : null}
      </div>
    </details>
  );
}

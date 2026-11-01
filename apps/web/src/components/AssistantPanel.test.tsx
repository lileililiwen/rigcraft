import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AssistantPanel } from "./AssistantPanel";
import type { AiProposalResponse, RecordItem, Snapshot } from "../types";

const SENTINEL_KEY = "sk-panel-sentinel-11be42";

function record(id: string, category: string, sku: string): RecordItem {
  return {
    id,
    lot: 1,
    sku,
    category,
    modelRaw: `Fixture ${sku}`,
    modelNormalized: `Fixture ${sku}`,
    quantity: 2,
    priceMinor: 19900,
    currency: "USD",
    specs: [],
    provenance: {},
    match: { basis: "lexical", confidence: "unverified" },
  };
}

const snapshot: Snapshot = {
  id: "snap-1",
  createdAt: "2026-10-31T12:00:00Z",
  revision: 1,
  parentSnapshotId: null,
  currency: "USD",
  source: { filename: "ai.csv", sha256: "a".repeat(64) },
  note: null,
  records: [record("r1", "cpu", "FIX-CPU"), record("r2", "gpu", "FIX-GPU")],
  totals: { recordCount: 2, unitCount: 4, valueMinor: 39800 },
};

const proposal: AiProposalResponse = {
  mode: "provider",
  promptVersion: "ai-parse-v1",
  provider: { kind: "mock", baseUrl: "https://provider.example/v1", model: "mock-model" },
  proposal: {
    budgetMinor: { value: 200000, source: "sentence", excerpt: "$2000" },
    currency: { value: "USD", source: "snapshot", note: "Taken from the selected snapshot." },
    useCase: { value: "gaming", source: "sentence", excerpt: "gaming" },
    inventoryOnly: { value: true, source: "default", note: "Always on." },
    lockedSkus: { value: ["FIX-CPU"], source: "sentence", excerpt: "FIX-CPU" },
    excludedSkus: { value: [], source: "default" },
  },
  ambiguities: [
    {
      field: "budgetMinor",
      code: "ungrounded_value",
      message: "The interpreted budget could not be traced to the sentence.",
    },
  ],
  ignoredFields: ["performanceScore"],
  sent: {
    sentence: true,
    currency: true,
    useCases: ["gaming", "office", "creation"],
    inventoryFields: [],
    inventoryRecords: 0,
  },
  confirmRequired: true,
};

function jsonResponse(data: unknown, status = 200): Promise<Response> {
  return Promise.resolve({
    ok: status < 400,
    status,
    headers: { get: () => "application/json" },
    text: async () => JSON.stringify(data),
  } as unknown as Response);
}

function installFetch(handler: (url: string, init?: RequestInit) => Promise<Response>) {
  const fetchMock = vi.fn((input: RequestInfo | URL, init?: RequestInit) =>
    handler(String(input), init),
  );
  vi.stubGlobal("fetch", fetchMock);
  return fetchMock;
}

function renderPanel(overrides: Partial<Parameters<typeof AssistantPanel>[0]> = {}) {
  const onApply = vi.fn();
  const onSkip = vi.fn();
  const announce = vi.fn();
  render(
    <AssistantPanel
      snapshot={snapshot}
      onApply={onApply}
      onSkip={onSkip}
      announce={announce}
      {...overrides}
    />,
  );
  return { onApply, onSkip, announce };
}

async function fillTransport(user: ReturnType<typeof userEvent.setup>) {
  await user.type(screen.getByLabelText("Provider key"), SENTINEL_KEY);
  await user.type(screen.getByLabelText("Provider base URL"), "https://provider.example/v1");
  await user.type(screen.getByLabelText("Model name"), "mock-model");
  await user.type(
    screen.getByLabelText("Your sentence"),
    "Build a $2000 USD gaming PC from only my stock and keep FIX-CPU.",
  );
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("assistant panel", () => {
  it("starts collapsed and keeps the credential fields in a form", async () => {
    const user = userEvent.setup();
    renderPanel();
    const details = document.querySelector("details.assistant") as HTMLDetailsElement;
    expect(details).not.toBeNull();
    expect(details.open).toBe(false);

    await user.click(details.querySelector("summary") as HTMLElement);
    expect(details.open).toBe(true);
    expect(screen.getByText("No key configured")).toBeInTheDocument();
    const key = screen.getByLabelText("Provider key") as HTMLInputElement;
    expect(key.form).toBe(details.querySelector("form.assistant-transport"));
  });

  it("states the no-key state and offers the deterministic flow", async () => {
    const user = userEvent.setup();
    const { onSkip } = renderPanel();
    expect(screen.getByRole("heading", { name: "No key configured" })).toBeInTheDocument();
    const parse = screen.getByRole("button", { name: "Interpret sentence" });
    expect(parse).toBeDisabled();
    expect(screen.getByText("Enter your own provider key to use the assistant.")).toBeInTheDocument();
    await user.click(screen.getAllByRole("button", { name: "Continue without AI" })[0]);
    expect(onSkip).toHaveBeenCalledTimes(1);
  });

  it("requires consent before anything can be sent", async () => {
    installFetch(async () => jsonResponse(proposal));
    const user = userEvent.setup();
    renderPanel();
    await fillTransport(user);
    const parse = screen.getByRole("button", { name: "Interpret sentence" });
    expect(parse).toBeDisabled();
    expect(screen.getByText("Tick the consent box to send the sentence.")).toBeInTheDocument();
    await user.click(screen.getByLabelText(/I understand this sends my sentence/));
    expect(parse).toBeEnabled();
  });

  it("shows a per-field proposal with its source and never reprints the key", async () => {
    const fetchMock = installFetch(async () => jsonResponse(proposal));
    const user = userEvent.setup();
    const { announce } = renderPanel();
    await fillTransport(user);
    await user.click(screen.getByLabelText(/I understand this sends my sentence/));
    await user.click(screen.getByRole("button", { name: "Interpret sentence" }));

    const heading = await screen.findByRole("heading", {
      name: "Proposed request — review and confirm",
    });
    await waitFor(() => expect(heading).toHaveFocus());
    expect(screen.getByText("✎ From your sentence: “$2000”")).toBeInTheDocument();
    expect(
      screen.getByText("▣ From this snapshot — Taken from the selected snapshot."),
    ).toBeInTheDocument();
    expect(screen.getByText(/≡ Default — Always on\./)).toBeInTheDocument();
    expect(screen.getByText(/Needs your confirmation \(1\)/)).toBeInTheDocument();
    expect(screen.getByText(/performanceScore/)).toBeInTheDocument();
    expect(announce).toHaveBeenCalledWith(
      expect.stringContaining("Proposal ready with 1 item to confirm"),
    );
    expect(document.body.textContent ?? "").not.toContain(SENTINEL_KEY);

    const body = JSON.parse(String(fetchMock.mock.calls[0][1]?.body));
    expect(body.apiKey).toBe(SENTINEL_KEY);
    expect(body.snapshotId).toBe("snap-1");
    expect(body.includeInventorySummary).toBe(false);
  });

  it("marks an edited field and applies the confirmed values", async () => {
    installFetch(async () => jsonResponse(proposal));
    const user = userEvent.setup();
    const { onApply, announce } = renderPanel();
    await fillTransport(user);
    await user.click(screen.getByLabelText(/I understand this sends my sentence/));
    await user.click(screen.getByRole("button", { name: "Interpret sentence" }));

    const budget = await screen.findByLabelText("Proposed total budget (USD)");
    await user.clear(budget);
    await user.type(budget, "1500.00");
    expect(screen.getByText(/Edited by you/)).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Confirm and apply to the form" }));
    expect(onApply).toHaveBeenCalledWith({
      budgetText: "1500.00",
      useCase: "gaming",
      lockedSkus: ["FIX-CPU"],
      excludedSkus: [],
    });
    expect(announce).toHaveBeenCalledWith(
      expect.stringContaining("Proposal confirmed and applied to the form"),
    );
  });

  it("blocks confirmation with a readable field error", async () => {
    installFetch(async () => jsonResponse(proposal));
    const user = userEvent.setup();
    const { onApply } = renderPanel();
    await fillTransport(user);
    await user.click(screen.getByLabelText(/I understand this sends my sentence/));
    await user.click(screen.getByRole("button", { name: "Interpret sentence" }));

    const budget = await screen.findByLabelText("Proposed total budget (USD)");
    await user.clear(budget);
    await user.type(budget, "abc");
    await user.click(screen.getByRole("button", { name: "Confirm and apply to the form" }));
    expect(onApply).not.toHaveBeenCalled();
    expect(
      screen.getByText("Enter a positive budget with at most two decimals, for example 1200.00."),
    ).toBeInTheDocument();
  });

  it("keeps the sentence on provider failure and offers retry and the form", async () => {
    installFetch(async () =>
      jsonResponse(
        {
          error: {
            code: "ai_provider_timeout",
            message: "The provider did not answer within 20 seconds.",
          },
        },
        504,
      ),
    );
    const user = userEvent.setup();
    const { onSkip } = renderPanel();
    await fillTransport(user);
    await user.click(screen.getByLabelText(/I understand this sends my sentence/));
    await user.click(screen.getByRole("button", { name: "Interpret sentence" }));

    const alert = await screen.findByRole("alert");
    await waitFor(() => expect(alert).toHaveFocus());
    expect(screen.getByText("ai_provider_timeout")).toBeInTheDocument();
    expect(screen.getByText(/Your sentence, key and form values are kept/)).toBeInTheDocument();
    expect(
      (screen.getByLabelText("Your sentence") as HTMLTextAreaElement).value,
    ).toContain("Build a $2000 USD gaming PC");
    expect(screen.getByRole("button", { name: "Retry interpretation" })).toBeInTheDocument();
    const skip = screen
      .getAllByRole("button", { name: "Continue without AI" })
      .at(-1) as HTMLButtonElement;
    await user.click(skip);
    expect(onSkip).toHaveBeenCalled();
  });

  it("discards a proposal without losing the sentence", async () => {
    installFetch(async () => jsonResponse(proposal));
    const user = userEvent.setup();
    const { announce } = renderPanel();
    await fillTransport(user);
    await user.click(screen.getByLabelText(/I understand this sends my sentence/));
    await user.click(screen.getByRole("button", { name: "Interpret sentence" }));
    await screen.findByRole("heading", { name: "Proposed request — review and confirm" });

    await user.click(screen.getByRole("button", { name: "Cancel" }));
    expect(
      screen.queryByRole("heading", { name: "Proposed request — review and confirm" }),
    ).not.toBeInTheDocument();
    expect(
      (screen.getByLabelText("Your sentence") as HTMLTextAreaElement).value,
    ).toContain("Build a $2000 USD gaming PC");
    expect(announce).toHaveBeenCalledWith(
      "Proposal discarded. Nothing was applied and the sentence is kept.",
    );
  });

  it("keeps a part out of the excluded list when it is also kept", async () => {
    installFetch(async () => jsonResponse(proposal));
    const user = userEvent.setup();
    const { onApply } = renderPanel();
    await fillTransport(user);
    await user.click(screen.getByLabelText(/I understand this sends my sentence/));
    await user.click(screen.getByRole("button", { name: "Interpret sentence" }));
    await screen.findByRole("heading", { name: "Proposed request — review and confirm" });

    await user.click(screen.getByLabelText("Exclude Fixture FIX-CPU (FIX-CPU)"));
    await user.click(screen.getByLabelText("Exclude Fixture FIX-GPU (FIX-GPU)"));
    await user.click(screen.getByLabelText("Keep Fixture FIX-CPU (FIX-CPU)"));

    await user.click(screen.getByRole("button", { name: "Confirm and apply to the form" }));
    expect(onApply).toHaveBeenCalledWith({
      budgetText: "2000.00",
      useCase: "gaming",
      lockedSkus: ["FIX-CPU"],
      excludedSkus: ["FIX-GPU"],
    });
  });
});

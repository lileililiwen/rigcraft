import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { Preview, PreviewRow } from "../types";
import { PreviewReview } from "./PreviewReview";

function row(overrides: Partial<PreviewRow> & { row: number }): PreviewRow {
  return {
    status: "valid",
    values: {
      sku: `SKU-${overrides.row}`,
      category: "cpu",
      model: "Example",
      quantity: "1",
      price: "10",
      currency: "USD",
      specs: "",
    },
    normalized: {
      sku: `SKU-${overrides.row}`,
      category: "cpu",
      modelNormalized: "Example",
      quantity: 1,
      priceMinor: 1000,
      currency: "USD",
    },
    specs: [],
    errors: [],
    duplicateGroupId: null,
    provenance: {},
    match: { basis: "lexical", confidence: "unverified" },
    ...overrides,
  };
}

const preview: Preview = {
  previewId: "p1",
  createdAt: "2026-10-31T00:00:00Z",
  expiresAt: "2026-11-01T00:00:00Z",
  currency: "USD",
  filename: "mixed.csv",
  columns: ["sku", "category", "model", "quantity", "price"],
  mapping: {
    sku: "sku",
    category: "category",
    model: "model",
    quantity: "quantity",
    price: "price",
  },
  rows: [
    row({ row: 2, status: "duplicate", duplicateGroupId: "dup-1" }),
    row({ row: 3, status: "duplicate", duplicateGroupId: "dup-1" }),
    row({
      row: 4,
      status: "error",
      errors: [
        {
          field: "quantity",
          code: "quantity_format",
          message: "Quantity must be a whole number.",
          suggestion: "Enter a whole number such as 3.",
        },
      ],
    }),
  ],
  duplicateGroups: [{ groupId: "dup-1", sku: "SKU-2", rows: [2, 3] }],
  summary: { total: 3, valid: 0, errors: 1, duplicateRows: 2 },
};

const baseProps = {
  preview,
  decisions: { 2: "accept" as const, 3: "accept" as const, 4: "exclude" as const },
  resolutions: {},
  overrides: {},
  serverIssues: {},
  busy: false,
  onDecision: vi.fn(),
  onResolution: vi.fn(),
  onOverride: vi.fn(),
  onCommit: vi.fn(),
  onReparse: vi.fn(),
  onStartOver: vi.fn(),
};

describe("PreviewReview", () => {
  it("names every error field, duplicate group, and blocked commit reason", () => {
    render(<PreviewReview {...baseProps} />);
    expect(screen.getByText("Duplicate SKU: SKU-2")).toBeInTheDocument();
    expect(screen.getByText(/quantity must be a whole number/i)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Commit snapshot" })).toBeDisabled();
    expect(screen.getByText(/merge, replace, or keep separate/i)).toBeInTheDocument();
    expect(screen.getAllByText(/match unverified/i).length).toBe(preview.rows.length);
  });

  it("asks for an explicit duplicate resolution and enables commit after it", async () => {
    const user = userEvent.setup();
    const onResolution = vi.fn();
    const { rerender } = render(
      <PreviewReview {...baseProps} onResolution={onResolution} />,
    );
    await user.click(screen.getByLabelText(/Keep as separate lots/i));
    expect(onResolution).toHaveBeenCalledWith("dup-1", "keep_separate");

    rerender(
      <PreviewReview
        {...baseProps}
        onResolution={onResolution}
        resolutions={{ "dup-1": "keep_separate" }}
      />,
    );
    expect(screen.getByRole("button", { name: "Commit snapshot" })).toBeEnabled();
    await user.click(screen.getByRole("button", { name: "Commit snapshot" }));
    expect(baseProps.onCommit).toHaveBeenCalled();
  });

  it("opens a labelled row correction editor and applies an override", async () => {
    const user = userEvent.setup();
    const onOverride = vi.fn();
    render(<PreviewReview {...baseProps} onOverride={onOverride} />);
    const correctButtons = screen.getAllByRole("button", { name: "Correct" });
    await user.click(correctButtons[2]);
    const quantity = screen.getByLabelText("Quantity");
    expect(quantity).toHaveFocus();
    await user.clear(quantity);
    await user.type(quantity, "5");
    await user.click(screen.getByRole("button", { name: "Apply correction" }));
    expect(onOverride).toHaveBeenCalledWith(4, { quantity: "5" });
  });

  it("re-parses only when every required column is mapped", async () => {
    const user = userEvent.setup();
    const onReparse = vi.fn();
    render(<PreviewReview {...baseProps} onReparse={onReparse} />);
    await user.click(screen.getByText("Column mapping"));
    const price = screen.getByLabelText(/Sale price/);
    await user.selectOptions(price, "");
    expect(screen.getByRole("button", { name: "Re-parse with this mapping" })).toBeDisabled();
    expect(screen.getByText(/Map these required columns first: price/i)).toBeInTheDocument();
    await user.selectOptions(price, "price");
    await user.click(screen.getByRole("button", { name: "Re-parse with this mapping" }));
    expect(onReparse).toHaveBeenCalledWith(preview.mapping);
  });
});

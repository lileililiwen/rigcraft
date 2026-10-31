import type {
  Advice,
  AdviceAction,
  BuildPartInput,
  DiagnosisReport,
  MatchStatus,
  ParsedLine,
  ParsedList,
  PlanStatus,
  Verdict,
} from "./types";

export interface StatusMeta {
  symbol: string;
  label: string;
  className: string;
}

export const MATCH_META: Record<MatchStatus, StatusMeta> = {
  matched: { symbol: "✓", label: "Matched", className: "compatible" },
  ambiguous: { symbol: "?", label: "Ambiguous", className: "unknown" },
  unmatched: { symbol: "✕", label: "No match", className: "incompatible" },
  invalid: { symbol: "!", label: "Needs fixing", className: "error" },
};

export const ACTION_META: Record<AdviceAction, StatusMeta> = {
  must_replace: { symbol: "✕", label: "Must replace", className: "incompatible" },
  should_upgrade: { symbol: "?", label: "Should upgrade", className: "unknown" },
  keep: { symbol: "✓", label: "Fine as is", className: "compatible" },
};

export const PLAN_META: Record<PlanStatus, StatusMeta> = {
  not_needed: { symbol: "—", label: "Nothing to replace", className: "confirmed" },
  validated: { symbol: "✓", label: "Validated", className: "compatible" },
  needs_evidence: { symbol: "?", label: "Needs evidence", className: "unknown" },
  no_option: { symbol: "—", label: "No option in this snapshot", className: "duplicate" },
  no_valid_plan: { symbol: "✕", label: "No validated plan", className: "incompatible" },
};

/**
 * Explicit "leave this line out" decision. An empty choice means the operator
 * has not decided yet, which blocks the diagnosis instead of silently dropping
 * the line.
 */
export const SKIP = "__skip__";

/** A line the operator still has to resolve before the build can be diagnosed. */
export interface ReviewProblem {
  line: number;
  message: string;
}

/**
 * Default choices: an exact match is preselected as a suggestion, everything
 * else starts unresolved. No ambiguous line is ever given a default SKU.
 */
export function initialChoices(list: ParsedList): Record<number, string> {
  const choices: Record<number, string> = {};
  for (const line of list.lines) {
    if (line.duplicateOf != null || line.status === "invalid") {
      choices[line.line] = SKIP;
      continue;
    }
    const exact = line.candidates.filter((candidate) => candidate.exact);
    choices[line.line] =
      line.status === "matched" && exact.length === 1 ? exact[0].sku : "";
  }
  return choices;
}

export function lineIsResolved(line: ParsedLine, choice: string): boolean {
  if (line.duplicateOf != null || line.status === "invalid") {
    return true;
  }
  return choice !== "";
}

export function choiceIsSkipped(choice: string): boolean {
  return choice === SKIP;
}

export function reviewProblems(
  list: ParsedList | null,
  choices: Record<number, string>,
): ReviewProblem[] {
  if (!list) {
    return [];
  }
  const problems: ReviewProblem[] = [];
  const chosen = new Map<string, number>();
  for (const line of list.lines) {
    // An invalid or duplicate line is never added; it stays visible with its
    // issue so the operator can fix or re-match it.
    if (line.status === "invalid" || line.duplicateOf != null) {
      continue;
    }
    const choice = choices[line.line] ?? "";
    if (!lineIsResolved(line, choice)) {
      problems.push({
        line: line.line,
        message:
          line.status === "ambiguous"
            ? `Line ${line.line} is ambiguous: choose a candidate or leave the line out.`
            : `Line ${line.line} has no match: choose a candidate, add it by hand or leave the line out.`,
      });
      continue;
    }
    if (choice !== SKIP) {
      const previous = chosen.get(choice);
      if (previous != null) {
        problems.push({
          line: line.line,
          message: `Line ${line.line} and line ${previous} both use ${choice}; use each SKU once.`,
        });
      } else {
        chosen.set(choice, line.line);
      }
    }
  }
  return problems;
}

export interface ReviewedPart {
  line: number;
  part: BuildPartInput;
}

/** Snapshot-sourced parts from the confirmed match review. */
export function reviewedParts(
  list: ParsedList | null,
  choices: Record<number, string>,
): ReviewedPart[] {
  if (!list) {
    return [];
  }
  const parts: ReviewedPart[] = [];
  for (const line of list.lines) {
    if (line.duplicateOf != null || line.status === "invalid") {
      continue;
    }
    const sku = choices[line.line] ?? "";
    if (!sku || sku === SKIP) {
      continue;
    }
    const candidate = line.candidates.find((entry) => entry.sku === sku);
    if (!candidate) {
      continue;
    }
    parts.push({
      line: line.line,
      part: {
        source: "snapshot",
        sku: candidate.sku,
        quantity: line.quantity,
      },
    });
  }
  return parts;
}

export function groupRows(
  report: DiagnosisReport,
  action: AdviceAction,
): Advice[] {
  const rows =
    action === "must_replace"
      ? report.groups.mustReplace
      : action === "should_upgrade"
        ? report.groups.shouldUpgrade
        : report.groups.keep;
  const byRow = new Map(report.advice.map((entry) => [entry.row, entry]));
  return rows
    .map((row) => byRow.get(row))
    .filter((entry): entry is Advice => Boolean(entry));
}

export function reportHeadline(report: DiagnosisReport): string {
  const conflict = report.check.summary.incompatible;
  const unknown = report.check.summary.unknown;
  const parts = [
    `${report.check.summary.compatible} rule${
      report.check.summary.compatible === 1 ? "" : "s"
    } passed`,
    `${conflict} conflict${conflict === 1 ? "" : "s"}`,
    `${unknown} unknown`,
  ];
  if (report.check.completeness.status === "incomplete") {
    parts.push(
      `${report.check.completeness.missing.length} required part${
        report.check.completeness.missing.length === 1 ? " is" : "s are"
      } missing`,
    );
  }
  return parts.join(" · ");
}

export function planValidated(planStatus: PlanStatus): boolean {
  return planStatus === "validated";
}

export function verdictWord(verdict: Verdict): string {
  return verdict === "compatible"
    ? "compatible"
    : verdict === "incompatible"
      ? "incompatible"
      : "unknown";
}

/** Short label for a line's decision, used in the review summary. */
export function lineDecisionLabel(line: ParsedLine, choice: string): string {
  if (line.status === "invalid") {
    return "Needs fixing";
  }
  if (line.duplicateOf != null) {
    return "Duplicate — not added";
  }
  if (choice === SKIP) {
    return "Left out";
  }
  if (!choice) {
    return line.status === "ambiguous" ? "Choose a candidate" : "Not decided";
  }
  return choice;
}

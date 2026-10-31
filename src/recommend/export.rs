//! Export the stored report as machine-readable JSON or a human-readable plan.
//!
//! Export never recomputes prices or verdicts: it reads the persisted report,
//! so the file matches what the UI showed for that report identifier.

use std::collections::BTreeSet;

use serde_json::json;

use crate::compatibility::Verdict;
use crate::domain::{now_rfc3339, Snapshot};

use super::{format_money, RecommendationReport, EXPORT_VERSION};

fn status_label(status: super::RecommendStatus) -> &'static str {
    match status {
        super::RecommendStatus::Ok => "ok",
        super::RecommendStatus::NoSolution => "no_solution",
        super::RecommendStatus::SearchLimited => "search_limited",
    }
}

fn completeness_label(status: crate::compatibility::CompletenessStatus) -> &'static str {
    match status {
        crate::compatibility::CompletenessStatus::Complete => "complete",
        crate::compatibility::CompletenessStatus::Incomplete => "incomplete",
    }
}

fn verdict_label(status: Verdict) -> &'static str {
    match status {
        Verdict::Compatible => "Compatible",
        Verdict::Incompatible => "Incompatible",
        Verdict::Unknown => "Unknown",
    }
}

pub fn export_json(report: &RecommendationReport, snapshot: &Snapshot) -> serde_json::Value {
    let checks: Vec<serde_json::Value> = report
        .candidates
        .iter()
        .map(|candidate| {
            json!({
                "candidateRank": candidate.rank,
                "checkReportId": candidate.check_report_id,
                "ruleSetVersion": candidate.check.rule_set_version,
                "overallStatus": candidate.check.overall_status,
                "summary": candidate.check.summary,
                "unknownFollowUps": candidate.unknown_follow_ups,
            })
        })
        .collect();
    json!({
        "exportVersion": EXPORT_VERSION,
        "exportedAt": now_rfc3339(),
        "notice": "Prices, stock and verdicts come from the immutable snapshot revision below. Re-run the recommendation after an inventory or rule change; this file never recomputes prices.",
        "snapshot": {
            "id": snapshot.id,
            "revision": snapshot.revision,
            "currency": snapshot.currency,
            "sourceFilename": snapshot.source.filename,
            "sourceSha256": snapshot.source.sha256,
        },
        "report": report,
        "checks": checks,
    })
}

fn evidence_references(report: &RecommendationReport) -> Vec<String> {
    let mut references = BTreeSet::new();
    for candidate in &report.candidates {
        for rule in &candidate.check.rules {
            for evidence in &rule.evidence {
                if let Some(provenance) = &evidence.provenance {
                    references.insert(format!("{} ({})", provenance.reference, provenance.source));
                }
            }
        }
    }
    references.into_iter().collect()
}

pub fn export_markdown(report: &RecommendationReport) -> String {
    let currency = report.currency.as_str();
    let mut out = String::new();
    out.push_str("# RigCraft build recommendation\n\n");
    out.push_str(&format!("- Generated: {}\n", report.created_at));
    out.push_str(&format!(
        "- Report: `{}` · status `{}`\n",
        report.id,
        status_label(report.status)
    ));
    out.push_str(&format!(
        "- Snapshot: `{}` revision {} · currency {}\n",
        report.snapshot_id, report.snapshot_revision, report.currency
    ));
    out.push_str(&format!(
        "- Budget: {} · use case: {} · inventory only: {}\n",
        format_money(report.budget_minor, currency),
        report.request.use_case,
        if report.request.inventory_only {
            "yes"
        } else {
            "no"
        }
    ));
    out.push_str(&format!(
        "- Versions: scoring `{}` · rules `{}`\n",
        report.scoring_version, report.rule_set_version
    ));
    if report.stale {
        out.push_str(&format!(
            "- **Stale:** {}\n",
            report.stale_reason.clone().unwrap_or_else(|| {
                "the snapshot changed after this report was created.".to_string()
            })
        ));
    }
    out.push_str(&format!(
        "- Search: {} node(s), {} complete combination(s), {} valid build(s), limit {}{}\n",
        report.search.nodes_examined,
        report.search.leaves_evaluated,
        report.search.valid_builds,
        report.search.limit,
        if report.search.limited {
            " (limit reached; no global optimum claimed)"
        } else {
            ""
        }
    ));
    if !report.request.locked_skus.is_empty() {
        out.push_str(&format!(
            "- Locked (owned): {}\n",
            report.request.locked_skus.join(", ")
        ));
    }
    if !report.request.excluded_skus.is_empty() {
        out.push_str(&format!(
            "- Excluded by request: {}\n",
            report.request.excluded_skus.join(", ")
        ));
    }
    out.push('\n');

    if let Some(no_solution) = &report.no_solution {
        out.push_str("## No solution\n\n");
        out.push_str(&format!("**{}**\n\n", no_solution.headline));
        out.push_str(&format!("{}\n\n", no_solution.explanation));
        if !no_solution.blockers.is_empty() {
            out.push_str("Blockers:\n");
            for blocker in &no_solution.blockers {
                out.push_str(&format!("- {}\n", blocker.detail));
            }
            out.push('\n');
        }
        out.push_str("What can be adjusted:\n");
        for item in &no_solution.adjustable {
            out.push_str(&format!("- {item}\n"));
        }
        out.push('\n');
    }

    if !report.candidates.is_empty() {
        out.push_str("## Ranked options\n\n");
        for candidate in &report.candidates {
            out.push_str(&format!(
                "### {}. {} — {} ({})\n\n",
                candidate.rank,
                candidate.label,
                format_money(candidate.total_minor, currency),
                verdict_label(candidate.overall_status)
            ));
            out.push_str(&format!(
                "- Objective: `{}` · budget delta {} · stock units {} · owned units {} · completeness `{}`\n",
                candidate.objective,
                format_money(candidate.budget_delta_minor, currency),
                candidate.units_from_stock,
                candidate.owned_units,
                completeness_label(candidate.completeness)
            ));
            out.push_str(&format!(
                "- Score {:.2} ({}) · rules {} compatible / {} conflict / {} unknown\n",
                candidate.scores.total,
                candidate.scores.confidence,
                candidate.check_summary.compatible,
                candidate.check_summary.incompatible,
                candidate.check_summary.unknown
            ));
            out.push_str(&format!(
                "- Check report: `{}` · rule set `{}`\n\n",
                candidate.check_report_id, candidate.check.rule_set_version
            ));
            out.push_str("| Category | Part | SKU | Qty | Unit price | Line total | Stock |\n");
            out.push_str("| --- | --- | --- | --- | --- | --- | --- |\n");
            for part in &candidate.parts {
                let stock = candidate
                    .stock
                    .iter()
                    .find(|line| line.sku == part.sku)
                    .map(|line| {
                        if line.owned {
                            format!("owned; {} available", line.available)
                        } else {
                            format!("{} used; {} left", line.used, line.remaining)
                        }
                    })
                    .unwrap_or_default();
                out.push_str(&format!(
                    "| {} | {} | `{}` | {} | {} | {} | {} |\n",
                    part.category,
                    part.model,
                    part.sku,
                    part.quantity,
                    format_money(part.unit_price_minor, currency),
                    format_money(part.line_total_minor, currency),
                    stock
                ));
            }
            out.push('\n');
            out.push_str("Why this build:\n");
            for reason in &candidate.reasons {
                out.push_str(&format!("- {reason}\n"));
            }
            out.push('\n');
            if !candidate.alternatives.is_empty() {
                out.push_str("Alternatives not selected:\n");
                for alternative in &candidate.alternatives {
                    out.push_str(&format!(
                        "- {} {} (`{}`) at {}: {}\n",
                        alternative.category,
                        alternative.model,
                        alternative.sku,
                        format_money(alternative.line_total_minor, currency),
                        alternative.detail
                    ));
                }
                out.push('\n');
            }
            if !candidate.unknown_follow_ups.is_empty() {
                out.push_str("Confirm before building:\n");
                for follow_up in &candidate.unknown_follow_ups {
                    out.push_str(&format!("- {follow_up}\n"));
                }
                out.push('\n');
            }
        }

        out.push_str("## Comparison\n\n");
        out.push_str("| |");
        for candidate in &report.candidates {
            out.push_str(&format!(" {} {}", candidate.rank, candidate.label));
        }
        out.push_str(" |\n| --- |");
        for _ in &report.candidates {
            out.push_str(" --- |");
        }
        out.push_str("\n| Total |");
        for candidate in &report.candidates {
            out.push_str(&format!(
                " {} |",
                format_money(candidate.total_minor, currency)
            ));
        }
        out.push_str("\n| Budget delta |");
        for candidate in &report.candidates {
            out.push_str(&format!(
                " {} |",
                format_money(candidate.budget_delta_minor, currency)
            ));
        }
        out.push_str("\n| Compatibility |");
        for candidate in &report.candidates {
            out.push_str(&format!(" {} |", verdict_label(candidate.overall_status)));
        }
        out.push_str("\n| Score |");
        for candidate in &report.candidates {
            out.push_str(&format!(" {:.2} |", candidate.scores.total));
        }
        out.push_str("\n| Check report |");
        for candidate in &report.candidates {
            out.push_str(&format!(" `{}` |", candidate.check_report_id));
        }
        out.push_str("\n\n");
    }

    if !report.exclusions.is_empty() {
        out.push_str("## Excluded from consideration\n\n");
        for exclusion in &report.exclusions {
            out.push_str(&format!(
                "- `{}` ({}) — {}: {}\n",
                exclusion.sku, exclusion.category, exclusion.reason, exclusion.detail
            ));
        }
        out.push('\n');
    }

    let references = evidence_references(report);
    if !references.is_empty() {
        out.push_str("## Evidence sources\n\n");
        for reference in references {
            out.push_str(&format!("- {reference}\n"));
        }
        out.push('\n');
    }

    out.push_str("## Manual confirmation required\n\n");
    out.push_str("- Unknown rules listed above are not verified compatibility. Confirm each follow-up before purchase or assembly.\n");
    out.push_str("- This file records the inventory snapshot revision, rule set and scoring versions it was produced from; it is stale after a correction or rule change.\n");
    out.push_str("- Stock is not reserved by producing this plan.\n");
    out
}

//! Candidate assembly: parts, stock use, scores, reasons and alternatives.

use std::collections::BTreeMap;

use crate::compatibility::Verdict;

use super::search::{Evaluated, Pool, CATEGORY_ORDER};
use super::{format_money, AlternativePart, Candidate, CandidatePart, Settings, StockLine};

fn stock_lines(parts: &[CandidatePart]) -> Vec<StockLine> {
    parts
        .iter()
        .map(|part| StockLine {
            sku: part.sku.clone(),
            category: part.category.clone(),
            used: if part.owned { 0 } else { part.quantity },
            available: part.available,
            remaining: if part.owned {
                part.available
            } else {
                part.available.saturating_sub(part.quantity)
            },
            owned: part.owned,
        })
        .collect()
}

fn category_rank(category: &str) -> usize {
    CATEGORY_ORDER
        .iter()
        .position(|entry| *entry == category)
        .unwrap_or(CATEGORY_ORDER.len())
}

pub(crate) fn make_candidate(
    evaluated: Evaluated,
    rank: usize,
    objective: &str,
    label: &str,
    pools: &[Pool],
    settings: &Settings,
    valid_builds: u64,
) -> Candidate {
    let mut parts: Vec<CandidatePart> = evaluated
        .parts
        .iter()
        .map(|part| CandidatePart {
            record_id: part.record.id.clone(),
            sku: part.record.sku.clone(),
            category: part.record.category.clone(),
            model: if part.record.model_normalized.is_empty() {
                part.record.model_raw.clone()
            } else {
                part.record.model_normalized.clone()
            },
            quantity: part.quantity,
            owned: part.owned,
            unit_price_minor: part.record.price_minor,
            line_total_minor: part.line_total_minor,
            available: part.record.quantity,
        })
        .collect();
    parts.sort_by(|left, right| {
        category_rank(&left.category)
            .cmp(&category_rank(&right.category))
            .then(left.sku.cmp(&right.sku))
    });
    let stock = stock_lines(&parts);
    let units_from_stock = stock
        .iter()
        .filter(|line| !line.owned)
        .map(|line| line.used)
        .sum();
    let owned_units = stock
        .iter()
        .filter(|line| line.owned)
        .map(|line| line.used.max(1))
        .sum();
    let total = parts.iter().map(|part| part.line_total_minor).sum();
    let budget_delta = settings.budget_minor - total;
    let unknown_follow_ups: Vec<String> = evaluated
        .report
        .rules
        .iter()
        .filter(|rule| rule.status == Verdict::Unknown)
        .map(|rule| {
            rule.follow_up
                .clone()
                .unwrap_or_else(|| rule.explanation.clone())
        })
        .collect();
    let mut reasons = Vec::new();
    match objective {
        "best_overall" => reasons.push(format!(
            "Highest weighted score for {}: {:.2}.",
            settings.use_case, evaluated.scores.total
        )),
        "lowest_cost" => reasons.push(format!(
            "Lowest total price among {} validated build(s): {}.",
            valid_builds,
            format_money(total, &settings.currency)
        )),
        "highest_performance" => reasons.push(format!(
            "Highest sourced performance proxy in this search: {:.2}.",
            evaluated.performance.unwrap_or(0.0)
        )),
        _ => reasons.push("Next best distinct build by weighted score.".to_string()),
    }
    match settings.use_case.as_str() {
        "gaming" => {
            let gpu = parts.iter().find(|part| part.category == "gpu");
            reasons.push(match gpu {
                Some(gpu) => format!(
                    "Gaming requires a discrete graphics card; this build includes the {}.",
                    gpu.model
                ),
                None => "Gaming requires a discrete graphics card.".to_string(),
            });
        }
        "office" => reasons.push(
            "Office accepts a CPU with sourced `igpu=yes`; a discrete graphics card is optional."
                .to_string(),
        ),
        _ => reasons.push(
            "A discrete graphics card was not required for creation; completeness does not depend on one."
                .to_string(),
        ),
    }
    reasons.push(format!(
        "Total {} leaves {} {}, and {} unit(s) come from snapshot stock.",
        format_money(total, &settings.currency),
        format_money(budget_delta, &settings.currency),
        if budget_delta >= 0 {
            "under the budget"
        } else {
            "over the budget"
        },
        units_from_stock
    ));
    if owned_units > 0 {
        reasons.push(format!(
            "{owned_units} owned part(s) are included at zero incremental price and take no stock."
        ));
    }
    let status_label = match evaluated.report.overall_status {
        Verdict::Compatible => "compatible",
        Verdict::Incompatible => "incompatible",
        Verdict::Unknown => "unknown",
    };
    reasons.push(format!(
        "Complete build; the compatibility check returned {status_label} with {} conflict(s) and {} unknown rule(s).",
        evaluated.report.summary.incompatible, evaluated.report.summary.unknown
    ));
    if let Some(dimension) = evaluated
        .scores
        .dimensions
        .iter()
        .find(|dimension| dimension.key == "performance" && !dimension.available)
    {
        reasons.push(dimension.basis.clone());
    }
    let chosen_by_category: BTreeMap<&str, &CandidatePart> = parts
        .iter()
        .map(|part| (part.category.as_str(), part))
        .collect();
    let mut alternatives = Vec::new();
    for pool in pools {
        let Some(chosen) = chosen_by_category.get(pool.category) else {
            continue;
        };
        let other = pool
            .options
            .iter()
            .filter(|piece| piece.record.sku != chosen.sku)
            .min_by(|left, right| {
                left.line_total_minor
                    .cmp(&right.line_total_minor)
                    .then(left.record.sku.cmp(&right.record.sku))
            });
        let Some(other) = other else {
            continue;
        };
        let model = if other.record.model_normalized.is_empty() {
            other.record.model_raw.clone()
        } else {
            other.record.model_normalized.clone()
        };
        let reason = if other.line_total_minor > chosen.line_total_minor {
            "higher_cost"
        } else {
            "different_build"
        };
        let detail = if reason == "higher_cost" {
            format!(
                "Costs {} more than the selected {} on this build.",
                format_money(
                    other.line_total_minor - chosen.line_total_minor,
                    &settings.currency
                ),
                chosen.sku
            )
        } else {
            format!(
                "Same or lower cost than the selected {}; not part of this build's selected combination.",
                chosen.sku
            )
        };
        alternatives.push(AlternativePart {
            category: pool.category.to_string(),
            sku: other.record.sku.clone(),
            model,
            line_total_minor: other.line_total_minor,
            reason: reason.to_string(),
            detail,
        });
    }
    Candidate {
        rank,
        objective: objective.to_string(),
        label: label.to_string(),
        material_signature: evaluated.material_signature.clone(),
        parts,
        total_minor: total,
        budget_delta_minor: budget_delta,
        stock,
        units_from_stock,
        owned_units,
        overall_status: evaluated.report.overall_status,
        completeness: evaluated.report.completeness.status,
        check_report_id: evaluated.report.id.clone(),
        check_summary: evaluated.report.summary.clone(),
        unknown_follow_ups,
        scores: evaluated.scores,
        reasons,
        alternatives,
        check: evaluated.report,
    }
}

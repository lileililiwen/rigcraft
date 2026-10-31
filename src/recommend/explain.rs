//! Concrete no-solution explanations for each binding constraint.

use std::collections::BTreeMap;

use super::search::{category_min, fact_is_yes, Pool};
use super::{format_money, Blocker, NoSolution, Settings, SEARCH_WORK_LIMIT};

pub(crate) fn minimum_complete_cost(pools: &[Pool]) -> Option<i64> {
    let cpu_min = pools
        .iter()
        .find(|pool| pool.category == "cpu")
        .and_then(|pool| {
            pool.options
                .iter()
                .filter_map(|cpu| {
                    let gpu = if fact_is_yes(&cpu.record, "igpu") {
                        Some(0)
                    } else {
                        category_min(pools, "gpu")
                    };
                    let cooler = if fact_is_yes(&cpu.record, "bundled_cooler") {
                        Some(0)
                    } else {
                        category_min(pools, "cooler")
                    };
                    Some(
                        cpu.line_total_minor
                            .saturating_add(gpu?)
                            .saturating_add(cooler?),
                    )
                })
                .min()
        })?;
    let rest: i64 = pools
        .iter()
        .filter(|pool| pool.required && pool.category != "cpu")
        .map(|pool| {
            pool.options
                .iter()
                .map(|piece| piece.line_total_minor)
                .min()
                .unwrap_or(0)
        })
        .sum();
    Some(cpu_min.saturating_add(rest))
}

pub(crate) fn required_cheapest_blockers(pools: &[Pool], currency: &str) -> Vec<Blocker> {
    pools
        .iter()
        .filter(|pool| pool.required)
        .filter_map(|pool| {
            let cheapest = pool.options.iter().min_by(|left, right| {
                left.line_total_minor
                    .cmp(&right.line_total_minor)
                    .then(left.record.sku.cmp(&right.record.sku))
            })?;
            Some(Blocker {
                code: "cheapest_part".to_string(),
                category: Some(pool.category.to_string()),
                sku: Some(cheapest.record.sku.clone()),
                detail: format!(
                    "Cheapest {}: {} at {}.",
                    pool.category,
                    cheapest.record.model_normalized,
                    format_money(cheapest.line_total_minor, currency)
                ),
            })
        })
        .collect()
}

fn describe_rejection(key: &str) -> String {
    if let Some(rule) = key.strip_prefix("incompatible:") {
        return format!("the `{rule}` compatibility rule reported a conflict");
    }
    if let Some(category) = key.strip_prefix("incomplete:") {
        return format!("the build was missing {category}");
    }
    if key == "check_error" {
        return "the compatibility check could not resolve the selection".to_string();
    }
    key.to_string()
}

pub(crate) fn compatibility_solution(
    leaves: u64,
    rejections: &BTreeMap<String, u64>,
    settings: &Settings,
) -> NoSolution {
    let mut entries: Vec<(&String, &u64)> = rejections.iter().collect();
    entries.sort_by(|left, right| right.1.cmp(left.1).then(left.0.cmp(right.0)));
    let blockers: Vec<Blocker> = entries
        .iter()
        .take(5)
        .map(|(key, count)| Blocker {
            code: "rejected_build".to_string(),
            category: key.split(':').nth(1).map(str::to_string),
            sku: None,
            detail: format!(
                "{count} complete combination(s): {}.",
                describe_rejection(key)
            ),
        })
        .collect();
    let rejected: u64 = rejections.values().sum();
    NoSolution {
        binding: "compatibility".to_string(),
        headline: "No complete build passes the compatibility rules.".to_string(),
        explanation: format!(
            "{} complete combination(s) were checked within the {} budget and none passed: {} rejection(s) came from compatibility conflicts or missing required evidence. No build the checker would fail is presented as a recommendation.",
            leaves,
            format_money(settings.budget_minor, &settings.currency),
            rejected
        ),
        blockers,
        adjustable: vec![
            "Correct the conflicting specification or record the missing fact named above, then import a corrected snapshot.".to_string(),
            "Add alternative parts so a compatible combination exists in stock.".to_string(),
            "Re-run after the correction; the recommendation reads the newest snapshot revision.".to_string(),
        ],
        minimum_build_minor: None,
        budget_shortfall_minor: None,
    }
}

pub(crate) fn search_limit_solution(nodes: u64, leaves: u64, minimum: Option<i64>) -> NoSolution {
    NoSolution {
        binding: "search_limit".to_string(),
        headline: "The search limit was reached before a valid build was confirmed.".to_string(),
        explanation: format!(
            "The search examined {} node(s) and {} complete combination(s) before hitting the {} node limit. This is not proof that no build exists, and no global optimum is claimed.",
            nodes, leaves, SEARCH_WORK_LIMIT
        ),
        blockers: Vec::new(),
        adjustable: vec![
            "Narrow the request (exclude parts or categories) so the search finishes within the limit.".to_string(),
            minimum
                .map(|value| {
                    format!(
                        "The cheapest structurally complete build costs at least {}.",
                        format_money(value, "USD")
                    )
                })
                .unwrap_or_else(|| "Raise the budget or import a smaller snapshot.".to_string()),
        ],
        minimum_build_minor: minimum,
        budget_shortfall_minor: None,
    }
}

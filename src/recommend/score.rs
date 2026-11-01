//! Published scoring dimensions and weights.

use super::search::{confirmed_number, Piece};
use super::{ScoreBreakdown, ScoreDimension};

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn round3(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

fn base_weights(use_case: &str) -> [(&'static str, &'static str, f64); 3] {
    match use_case {
        "office" => [
            ("cost", "Budget headroom", 0.6),
            ("performance", "Performance proxy", 0.2),
            ("upgrade", "Upgrade room", 0.2),
        ],
        "creation" => [
            ("cost", "Budget headroom", 0.25),
            ("performance", "Performance proxy", 0.5),
            ("upgrade", "Upgrade room", 0.25),
        ],
        _ => [
            ("cost", "Budget headroom", 0.3),
            ("performance", "Performance proxy", 0.55),
            ("upgrade", "Upgrade room", 0.15),
        ],
    }
}

fn performance_proxy(parts: &[&Piece]) -> (Option<f64>, String) {
    let mut values = Vec::new();
    for piece in parts {
        if piece.record.category != "cpu" && piece.record.category != "gpu" {
            continue;
        }
        let Some(value) = confirmed_number(&piece.record, "perf_score") else {
            continue;
        };
        let source = piece
            .record
            .specs
            .iter()
            .find(|spec| spec.key == "perf_score")
            .map(|spec| spec.provenance.reference.clone())
            .unwrap_or_else(|| "no source recorded".to_string());
        values.push(format!("{value} on {} ({source})", piece.record.sku));
    }
    if values.is_empty() {
        return (
            None,
            "No confirmed perf_score is recorded on the CPU or graphics card, so performance is not scored instead of counted as zero.".to_string(),
        );
    }
    let mean = values
        .iter()
        .map(|entry| {
            entry
                .split_whitespace()
                .next()
                .and_then(|value| value.parse::<f64>().ok())
                .unwrap_or(0.0)
        })
        .sum::<f64>()
        / values.len() as f64;
    (
        Some(round2(mean)),
        format!("Mean of confirmed perf_score: {}.", values.join("; ")),
    )
}

fn upgrade_room(parts: &[&Piece]) -> (Option<f64>, String) {
    let board = parts
        .iter()
        .find(|piece| piece.record.category == "motherboard");
    let memory: Vec<&&Piece> = parts
        .iter()
        .filter(|piece| piece.record.category == "ram")
        .collect();
    let Some(board) = board else {
        return (
            None,
            "No motherboard is selected, so free memory slots are unknown.".to_string(),
        );
    };
    let Some(slots) = confirmed_number(&board.record, "ram_slots") else {
        return (
            None,
            format!(
                "The motherboard {} has no confirmed ram_slots fact, so upgrade room is not scored.",
                board.record.sku
            ),
        );
    };
    let mut modules = 0.0;
    for piece in &memory {
        let Some(count) = confirmed_number(&piece.record, "modules") else {
            return (
                None,
                format!(
                    "Memory {} has no confirmed modules fact, so upgrade room is not scored.",
                    piece.record.sku
                ),
            );
        };
        modules += count;
    }
    let free = (slots - modules).max(0.0);
    (
        Some(round2((free / 4.0).clamp(0.0, 1.0))),
        format!(
            "{}: {slots} on {} minus {} recorded on the memory.",
            crate::text::plural(free, "free memory slot", "free memory slots"),
            board.record.sku,
            crate::text::plural(modules, "module", "modules")
        ),
    )
}

pub(crate) fn score_build(
    parts: &[&Piece],
    total: i64,
    budget: i64,
    use_case: &str,
) -> ScoreBreakdown {
    let cost_value =
        round2(((((budget - total) as f64) / (budget as f64)).clamp(0.0, 1.0)) * 100.0);
    let (performance_value, performance_basis) = performance_proxy(parts);
    let (upgrade_value, upgrade_basis) = upgrade_room(parts);
    // Every dimension is reported on the same 0-100 index. The performance
    // proxy is a sourced 0-100 index recorded by the operator; cost and upgrade
    // room are derived proportions. Weights are fractions and sum to one.
    let display = [
        (
            cost_value,
            "Derived from this build's total price and the requested budget.",
        ),
        (
            performance_value
                .map(|value| round2(value.clamp(0.0, 100.0)))
                .unwrap_or(0.0),
            performance_basis.as_str(),
        ),
        (
            upgrade_value
                .map(|value| round2(value * 100.0))
                .unwrap_or(0.0),
            upgrade_basis.as_str(),
        ),
    ];
    let available_flags = [true, performance_value.is_some(), upgrade_value.is_some()];
    let weights = base_weights(use_case);
    let available_weight: f64 = weights
        .iter()
        .zip(available_flags.iter())
        .filter(|(_, available)| **available)
        .map(|(weight, _)| weight.2)
        .sum();
    let mut dimensions = Vec::with_capacity(3);
    let mut total_score = 0.0;
    for (index, (key, label, base)) in weights.iter().enumerate() {
        let available = available_flags[index];
        let weight = if available && available_weight > 0.0 {
            round3(base / available_weight)
        } else {
            0.0
        };
        let value = if available {
            Some(display[index].0)
        } else {
            None
        };
        if let Some(value) = value {
            total_score += value * weight;
        }
        dimensions.push(ScoreDimension {
            key: (*key).to_string(),
            label: (*label).to_string(),
            available,
            value,
            weight,
            basis: display[index].1.to_string(),
        });
    }
    ScoreBreakdown {
        total: round2(total_score),
        confidence: if available_flags.iter().all(|flag| *flag) {
            "full".to_string()
        } else {
            "partial".to_string()
        },
        dimensions,
    }
}

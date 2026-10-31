//! Reuse advice and the validated upgrade plan.
//!
//! Advice is a projection of the compatibility report and of the plan: a part
//! that a validated step replaces must be replaced, a part whose required fact
//! is missing should be verified before any purchase, and every other part is
//! kept. Each replacement is substituted into the described build and
//! re-checked with the same engine, so a plan the checker rejects is never
//! presented as valid.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::compatibility::{CheckReport, RuleEvidence, RuleResult, Verdict};
use crate::domain::{Record, Snapshot};
use crate::error::ApiError;

use super::{
    check_described, evidence_for, rule_ids_for, PartSpec, MAX_OPTIONS_EVALUATED,
    MAX_OPTIONS_SHOWN, MAX_REJECTED_SHOWN,
};

pub const MUST_REPLACE: &str = "must_replace";
pub const SHOULD_UPGRADE: &str = "should_upgrade";
pub const KEEP: &str = "keep";
pub const UNVERIFIED: &str = "unverified";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AdviceCost {
    pub currency: String,
    /// A kept part adds no purchase cost; it is already owned.
    pub keep_minor: i64,
    pub replacement_sku: Option<String>,
    pub replacement_model: Option<String>,
    pub replacement_minor: Option<i64>,
    pub additional_minor: Option<i64>,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Advice {
    pub row: usize,
    pub sku: String,
    pub category: String,
    pub model: String,
    /// `must_replace`, `should_upgrade` or `keep`.
    pub action: String,
    /// True when the action depends on confirming a missing fact first.
    pub conditional: bool,
    pub reason: String,
    pub rule_ids: Vec<String>,
    pub evidence: Vec<RuleEvidence>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification: Option<String>,
    pub cost: AdviceCost,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AdviceGroups {
    pub must_replace: Vec<usize>,
    pub should_upgrade: Vec<usize>,
    pub keep: Vec<usize>,
    pub unverified: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlanOption {
    pub row: usize,
    pub category: String,
    pub sku: String,
    pub model: String,
    pub price_minor: i64,
    /// The verdict of the whole described build with this one substitution.
    pub status: Verdict,
    /// Conflict rules this substitution removes from the composed build.
    pub resolves: Vec<String>,
    /// Conflict rules that remain after this substitution.
    pub rule_ids: Vec<String>,
    pub follow_ups: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RejectedOption {
    pub row: usize,
    pub category: String,
    pub sku: String,
    pub model: String,
    pub price_minor: i64,
    pub rule_ids: Vec<String>,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlanStep {
    pub row: usize,
    pub category: String,
    pub from_sku: String,
    pub from_model: String,
    pub to_sku: String,
    pub to_model: String,
    pub price_minor: i64,
    pub rule_ids: Vec<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UpgradePlan {
    /// `not_needed`, `validated`, `needs_evidence`, `no_option` or `no_valid_plan`.
    pub status: String,
    pub summary: String,
    pub steps: Vec<PlanStep>,
    pub options: Vec<PlanOption>,
    pub rejected: Vec<RejectedOption>,
    pub additional_cost_minor: i64,
    pub currency: String,
    /// The whole-plan check. Present only when steps are presented.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validation: Option<CheckReport>,
    pub follow_ups: Vec<String>,
    /// How many candidate substitutions were actually checked.
    pub evaluated: usize,
    pub method: String,
}

struct RowSearch {
    /// Substitutions that resolve at least one conflict of the described build.
    resolved: Vec<(PlanOption, Record)>,
    compatible: Vec<(PlanOption, Record)>,
    unvalidated: Vec<(PlanOption, Record)>,
    rejected: Vec<RejectedOption>,
    evaluated: usize,
}

fn unknown_follow_ups(check: &CheckReport) -> Vec<String> {
    let mut follow_ups: Vec<String> = check
        .rules
        .iter()
        .filter(|rule| rule.status == Verdict::Unknown)
        .map(|rule| {
            rule.follow_up
                .clone()
                .unwrap_or_else(|| rule.explanation.clone())
        })
        .collect();
    for missing in &check.completeness.missing {
        follow_ups.push(format!(
            "{}: {} {}",
            missing.category, missing.reason, missing.follow_up
        ));
    }
    follow_ups
}

fn used_skus(parts: &[PartSpec], row: usize) -> BTreeSet<String> {
    parts
        .iter()
        .filter(|part| part.row != row)
        .map(|part| part.sku.clone())
        .collect()
}

fn describe_candidate(record: &Record) -> String {
    if record.model_normalized.is_empty() {
        record.model_raw.clone()
    } else {
        record.model_normalized.clone()
    }
}

/// Substitute every candidate of this part's category, cheapest first, and
/// record the verdict of the whole described build for each one.
fn search_candidates(
    part: &PartSpec,
    parts: &[PartSpec],
    snapshot: &Snapshot,
    currency: &str,
    baseline_conflicts: &[String],
) -> Result<RowSearch, ApiError> {
    let used = used_skus(parts, part.row);
    let mut seen = BTreeSet::new();
    let mut pool: Vec<&Record> = snapshot
        .records
        .iter()
        .filter(|record| record.category == part.category)
        .filter(|record| record.quantity > 0)
        .filter(|record| record.currency == currency)
        .filter(|record| record.sku != part.sku && !used.contains(&record.sku))
        .filter(|record| seen.insert(record.sku.clone()))
        .collect();
    pool.sort_by(|left, right| {
        left.price_minor
            .cmp(&right.price_minor)
            .then(left.sku.cmp(&right.sku))
    });

    let mut search = RowSearch {
        resolved: Vec::new(),
        compatible: Vec::new(),
        unvalidated: Vec::new(),
        rejected: Vec::new(),
        evaluated: 0,
    };
    for candidate in pool {
        if search.evaluated >= MAX_OPTIONS_EVALUATED {
            break;
        }
        if search.compatible.len() >= MAX_OPTIONS_SHOWN {
            break;
        }
        search.evaluated += 1;
        let mut replacements = BTreeMap::new();
        replacements.insert(part.row, candidate.clone());
        let report = check_described(parts, currency, &replacements)?;
        let model = describe_candidate(candidate);
        let remaining: Vec<String> = report
            .rules
            .iter()
            .filter(|rule| rule.status == Verdict::Incompatible)
            .map(|rule| rule.rule_id.clone())
            .collect();
        let resolves: Vec<String> = baseline_conflicts
            .iter()
            .filter(|rule_id| !remaining.contains(rule_id))
            .cloned()
            .collect();
        let option = PlanOption {
            row: part.row,
            category: part.category.clone(),
            sku: candidate.sku.clone(),
            model,
            price_minor: candidate.price_minor,
            status: report.overall_status,
            resolves: resolves.clone(),
            rule_ids: remaining,
            follow_ups: unknown_follow_ups(&report),
        };
        if !resolves.is_empty() {
            search.resolved.push((option.clone(), candidate.clone()));
        }
        match report.overall_status {
            Verdict::Compatible => search.compatible.push((option, candidate.clone())),
            Verdict::Unknown => search.unvalidated.push((option, candidate.clone())),
            Verdict::Incompatible => {
                if search.rejected.len() < MAX_REJECTED_SHOWN {
                    search.rejected.push(RejectedOption {
                        row: part.row,
                        category: part.category.clone(),
                        sku: candidate.sku.clone(),
                        model: option.model.clone(),
                        price_minor: candidate.price_minor,
                        rule_ids: option.rule_ids.clone(),
                        detail: report
                            .rules
                            .iter()
                            .find(|rule| rule.status == Verdict::Incompatible)
                            .map(|rule| rule.explanation.clone())
                            .unwrap_or_else(|| {
                                "The composed build did not pass the checker.".to_string()
                            }),
                    });
                }
            }
        }
    }
    Ok(search)
}

fn keep_cost(currency: &str) -> AdviceCost {
    AdviceCost {
        currency: currency.to_string(),
        keep_minor: 0,
        replacement_sku: None,
        replacement_model: None,
        replacement_minor: None,
        additional_minor: None,
        note: "Already owned: keeping this part adds no purchase cost and takes no stock."
            .to_string(),
    }
}

fn replacement_cost(currency: &str, search: &RowSearch, conditional: bool) -> AdviceCost {
    let (option, note) = if let Some((option, _)) = search.compatible.first() {
        (
            Some(option),
            "Cheapest replacement in this snapshot that the checker validates as compatible."
                .to_string(),
        )
    } else if let Some((option, _)) = search.resolved.first() {
        (
            Some(option),
            "Cheapest replacement that removes this conflict, but the composed build still needs evidence."
                .to_string(),
        )
    } else if let Some((option, _)) = search.unvalidated.first() {
        (
            Some(option),
            "Cheapest replacement that avoids a conflict, but the composed build is still unknown; confirm the follow-ups before buying."
                .to_string(),
        )
    } else {
        (
            None,
            "No candidate of this category in the snapshot avoided a conflict.".to_string(),
        )
    };
    match option {
        Some(option) => AdviceCost {
            currency: currency.to_string(),
            keep_minor: 0,
            replacement_sku: Some(option.sku.clone()),
            replacement_model: Some(option.model.clone()),
            replacement_minor: Some(option.price_minor),
            additional_minor: Some(option.price_minor),
            note: if conditional {
                format!("Only if the missing evidence shows this part is inadequate. {note}")
            } else {
                note
            },
        },
        None => AdviceCost {
            note: if conditional {
                format!("Keep this part until the missing fact is confirmed; nothing is bought today. {note}")
            } else {
                note
            },
            ..keep_cost(currency)
        },
    }
}

fn rule_explanations(check: &CheckReport, rule_ids: &[String]) -> String {
    check
        .rules
        .iter()
        .filter(|rule| rule_ids.contains(&rule.rule_id))
        .map(|rule| rule.explanation.clone())
        .collect::<Vec<_>>()
        .join(" ")
}

fn advice_for(
    part: &PartSpec,
    check: &CheckReport,
    search: Option<&RowSearch>,
    step: Option<&PlanStep>,
    resolution: ResolutionState<'_>,
    currency: &str,
) -> Advice {
    let record_id = part.record_id();
    let conflicts = rule_ids_for(check, &record_id, Verdict::Incompatible);
    let unknowns = rule_ids_for(check, &record_id, Verdict::Unknown);
    let passes = rule_ids_for(check, &record_id, Verdict::Compatible);
    let (action, conditional, reason, rule_ids, evidence, verification, cost) = if let Some(step) =
        step
    {
        (
            MUST_REPLACE,
            false,
            format!(
                "The checker found a definite conflict involving this part: {} Replacing {} with {} resolves it and the composed plan is {}.",
                rule_explanations(check, &step.rule_ids),
                part.sku,
                step.to_sku,
                resolution.label()
            ),
            conflicts,
            evidence_for(check, &record_id, Verdict::Incompatible),
            None,
            AdviceCost {
                currency: currency.to_string(),
                keep_minor: 0,
                replacement_sku: Some(step.to_sku.clone()),
                replacement_model: Some(step.to_model.clone()),
                replacement_minor: Some(step.price_minor),
                additional_minor: Some(step.price_minor),
                note: if resolution.presents_steps() {
                    format!(
                        "This step adds {}; no trade-in or resale value is assumed.",
                        crate::recommend::format_money(step.price_minor, currency)
                    )
                } else {
                    "No validated plan is offered, so nothing is bought from this report.".to_string()
                },
            },
        )
    } else if !conflicts.is_empty() && !resolution.presents_steps() {
        (
            MUST_REPLACE,
            false,
            format!(
                "The checker found a definite conflict involving this part: {} No replacement in this snapshot is validated for this part, so it remains in conflict.",
                rule_explanations(check, &conflicts)
            ),
            conflicts,
            evidence_for(check, &record_id, Verdict::Incompatible),
            None,
            search
                .map(|search| replacement_cost(currency, search, false))
                .unwrap_or_else(|| keep_cost(currency)),
        )
    } else if !conflicts.is_empty() {
        let replacers: Vec<String> = resolution
            .steps
            .iter()
            .map(|step| step.to_sku.clone())
            .collect();
        (
            KEEP,
            false,
            format!(
                "This part is in a conflict, but the plan resolves it by replacing {}. Keep this part. The upgraded build is {}.",
                replacers.join(", "),
                resolution.label()
            ),
            conflicts,
            evidence_for(check, &record_id, Verdict::Incompatible),
            None,
            keep_cost(currency),
        )
    } else if !unknowns.is_empty() {
        let follow_up = check
            .rules
            .iter()
            .find(|rule| unknowns.contains(&rule.rule_id))
            .and_then(|rule| rule.follow_up.clone())
            .unwrap_or_else(|| "Confirm the missing specification.".to_string());
        (
            SHOULD_UPGRADE,
            true,
            format!(
                "No definite conflict is recorded for this part, but the checker cannot confirm it: {follow_up} Gather that evidence first; replace the part only if the evidence shows it is inadequate."
            ),
            unknowns,
            evidence_for(check, &record_id, Verdict::Unknown),
            Some(follow_up),
            search
                .map(|search| replacement_cost(currency, search, true))
                .unwrap_or_else(|| keep_cost(currency)),
        )
    } else if !passes.is_empty() {
        let titles: Vec<String> = check
            .rules
            .iter()
            .filter(|rule| passes.contains(&rule.rule_id))
            .map(|rule| rule.title.clone())
            .collect();
        (
            KEEP,
            false,
            format!(
                "Every rule that involves this part passed: {}. Keep it.",
                titles.join(", ")
            ),
            passes,
            evidence_for(check, &record_id, Verdict::Compatible),
            None,
            keep_cost(currency),
        )
    } else {
        (
            UNVERIFIED,
            false,
            "No rule that could run involves this part, so the checker did not evaluate it. Its compatibility is not verified and it is retained exactly as described."
                .to_string(),
            Vec::new(),
            Vec::new(),
            None,
            keep_cost(currency),
        )
    };
    Advice {
        row: part.row,
        sku: part.sku.clone(),
        category: part.category.clone(),
        model: part.model.clone(),
        action: action.to_string(),
        conditional,
        reason,
        rule_ids,
        evidence,
        verification,
        cost,
    }
}

/// Whether the assembled plan presents steps, which decides whether a part in
/// a conflict is kept (another part resolves it) or still marked to replace.
struct ResolutionState<'a> {
    plan_status: &'a str,
    steps: &'a [PlanStep],
}

impl ResolutionState<'_> {
    fn presents_steps(&self) -> bool {
        matches!(self.plan_status, "validated" | "needs_evidence")
    }

    fn label(&self) -> &'static str {
        match self.plan_status {
            "validated" => "validated as compatible",
            "needs_evidence" => "not fully validated yet",
            _ => "not validated",
        }
    }
}

fn find_part<'a>(parts: &'a [PartSpec], record_id: &str) -> Option<&'a PartSpec> {
    parts.iter().find(|part| part.record_id() == record_id)
}

fn cheapest_resolution<'a>(
    rule: &RuleResult,
    parts: &'a [PartSpec],
    searches: &'a BTreeMap<usize, RowSearch>,
) -> Option<(PlanStep, Record)> {
    let mut best: Option<(PlanStep, Record)> = None;
    for part_ref in &rule.parts {
        let Some(part) = find_part(parts, &part_ref.record_id) else {
            continue;
        };
        let Some(search) = searches.get(&part.row) else {
            continue;
        };
        let Some((option, record)) = search
            .resolved
            .iter()
            .find(|(option, _)| option.resolves.contains(&rule.rule_id))
        else {
            continue;
        };
        let candidate = PlanStep {
            row: part.row,
            category: part.category.clone(),
            from_sku: part.sku.clone(),
            from_model: part.model.clone(),
            to_sku: option.sku.clone(),
            to_model: option.model.clone(),
            price_minor: option.price_minor,
            rule_ids: vec![rule.rule_id.clone()],
            reason: format!(
                "Replaces the part that resolves {} and keeps every other described part unchanged.",
                rule.rule_id
            ),
        };
        let replace = best
            .as_ref()
            .map(|(current, _)| {
                (candidate.price_minor, candidate.to_sku.as_str())
                    < (current.price_minor, current.to_sku.as_str())
            })
            .unwrap_or(true);
        if replace {
            best = Some((candidate, record.clone()));
        }
    }
    best
}

/// Build advice, its three groups, and the validated upgrade plan.
pub(crate) fn advise(
    parts: &[PartSpec],
    check: &CheckReport,
    snapshot: &Snapshot,
    currency: &str,
) -> Result<(Vec<Advice>, AdviceGroups, UpgradePlan), ApiError> {
    let conflict_rules: Vec<&RuleResult> = check
        .rules
        .iter()
        .filter(|rule| rule.status == Verdict::Incompatible)
        .collect();
    let baseline_conflicts: Vec<String> = conflict_rules
        .iter()
        .map(|rule| rule.rule_id.clone())
        .collect();

    // Search a replacement for every part that carries a conflict or an unknown.
    let mut searches: BTreeMap<usize, RowSearch> = BTreeMap::new();
    for part in parts {
        let record_id = part.record_id();
        let needs_search = !rule_ids_for(check, &record_id, Verdict::Incompatible).is_empty()
            || !rule_ids_for(check, &record_id, Verdict::Unknown).is_empty();
        if needs_search {
            searches.insert(
                part.row,
                search_candidates(part, parts, snapshot, currency, &baseline_conflicts)?,
            );
        }
    }

    // One resolution per conflicting rule, cheapest first across the rule's parts.
    let mut chosen: BTreeMap<usize, (PlanStep, Record)> = BTreeMap::new();
    let mut unresolved: Vec<String> = Vec::new();
    for rule in &conflict_rules {
        match cheapest_resolution(rule, parts, &searches) {
            Some((step, record)) => {
                chosen
                    .entry(step.row)
                    .and_modify(|(existing, _)| {
                        if step.price_minor < existing.price_minor {
                            *existing = step.clone();
                        }
                        if !existing.rule_ids.contains(&rule.rule_id) {
                            existing.rule_ids.push(rule.rule_id.clone());
                        }
                    })
                    .or_insert((step, record));
            }
            None => unresolved.push(rule.rule_id.clone()),
        }
    }

    let mut options = Vec::new();
    let mut rejected = Vec::new();
    let mut evaluated = 0;
    for search in searches.values() {
        evaluated += search.evaluated;
        options.extend(search.compatible.iter().map(|(option, _)| option.clone()));
        if search.compatible.is_empty() {
            options.extend(search.resolved.iter().map(|(option, _)| option.clone()));
        }
        rejected.extend(search.rejected.iter().cloned());
    }
    rejected.truncate(MAX_REJECTED_SHOWN);

    let method = format!(
        "Each candidate was substituted into the described build and re-checked with {}. At most {MAX_OPTIONS_EVALUATED} candidates per part were evaluated; the count is reported.",
        check.rule_set_version
    );

    let mut steps: Vec<PlanStep> = chosen.values().map(|(step, _)| step.clone()).collect();
    steps.sort_by_key(|left| left.row);

    let (plan_status, plan_summary, validation, follow_ups, additional, rejected, steps) =
        if conflict_rules.is_empty() {
            (
                "not_needed",
                "No described part must be replaced. Keep the build and confirm the listed facts."
                    .to_string(),
                None,
                unknown_follow_ups(check),
                0,
                rejected,
                Vec::new(),
            )
        } else if !unresolved.is_empty() {
            (
                "no_option",
                format!(
                    "This snapshot holds no validated replacement for the rule(s) {}, so no upgrade plan is offered.",
                    unresolved.join(", ")
                ),
                None,
                unknown_follow_ups(check),
                0,
                rejected,
                Vec::new(),
            )
        } else {
            let replacements: BTreeMap<usize, Record> = chosen
                .iter()
                .map(|(row, (_, record))| (*row, record.clone()))
                .collect();
            let composed = check_described(parts, currency, &replacements)?;
            let additional: i64 = steps.iter().map(|step| step.price_minor).sum();
            match composed.overall_status {
                Verdict::Compatible => (
                    "validated",
                    format!(
                        "The checker validates the upgraded build as compatible. {} step(s) add {}; no trade-in or resale value is assumed.",
                        steps.len(),
                        crate::recommend::format_money(additional, currency)
                    ),
                    Some(composed.clone()),
                    unknown_follow_ups(&composed),
                    additional,
                    rejected,
                    steps,
                ),
                Verdict::Unknown => {
                    let mut follow_ups = unknown_follow_ups(&composed);
                    follow_ups.sort();
                    follow_ups.dedup();
                    (
                        "needs_evidence",
                        "No conflict remains, but the upgraded build is not fully validated: the checker returns unknown. Confirm the listed facts before treating this plan as compatible."
                            .to_string(),
                        Some(composed),
                        follow_ups,
                        additional,
                        rejected,
                        steps,
                    )
                }
                Verdict::Incompatible => {
                    let mut rejected = rejected;
                    for step in &steps {
                        rejected.push(RejectedOption {
                            row: step.row,
                            category: step.category.clone(),
                            sku: step.to_sku.clone(),
                            model: step.to_model.clone(),
                            price_minor: step.price_minor,
                            rule_ids: composed
                                .rules
                                .iter()
                                .filter(|rule| rule.status == Verdict::Incompatible)
                                .map(|rule| rule.rule_id.clone())
                                .collect(),
                            detail: "The composed upgrade plan still fails the checker, so it is not offered as valid."
                                .to_string(),
                        });
                    }
                    rejected.truncate(MAX_REJECTED_SHOWN);
                    (
                        "no_valid_plan",
                        "The composed upgrade plan fails the compatibility checker. No plan is presented as valid; the rejected candidates and their rules are listed."
                            .to_string(),
                        None,
                        unknown_follow_ups(&composed),
                        0,
                        rejected,
                        Vec::new(),
                    )
                }
            }
        };

    let mut advice = Vec::with_capacity(parts.len());
    for part in parts {
        let step = steps.iter().find(|step| step.row == part.row);
        advice.push(advice_for(
            part,
            check,
            searches.get(&part.row),
            step,
            ResolutionState {
                plan_status,
                steps: &steps,
            },
            currency,
        ));
    }
    let mut groups = AdviceGroups::default();
    for entry in &advice {
        match entry.action.as_str() {
            MUST_REPLACE => groups.must_replace.push(entry.row),
            SHOULD_UPGRADE => groups.should_upgrade.push(entry.row),
            KEEP => groups.keep.push(entry.row),
            _ => groups.unverified.push(entry.row),
        }
    }

    Ok((
        advice,
        groups,
        UpgradePlan {
            status: plan_status.to_string(),
            summary: plan_summary,
            steps,
            options,
            rejected,
            additional_cost_minor: additional,
            currency: currency.to_string(),
            validation,
            follow_ups,
            evaluated,
            method,
        },
    ))
}

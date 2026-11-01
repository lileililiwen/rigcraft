//! Deterministic three-state compatibility checks with rule-level evidence.
//!
//! Every verdict is derived from facts already stored in the immutable
//! inventory snapshot. Missing facts produce `unknown` with a specific
//! follow-up question; they are never filled in from a similar model name,
//! a catalog, or a default. Conflicts always win over unknowns, and unknowns
//! always win over a compatible result.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::{now_rfc3339, Provenance, Record, Snapshot, Spec, SpecStatus};
use crate::error::ApiError;

mod rules;

use rules::evaluate_rules;

pub const RULE_SET_VERSION: &str = "compat-rules-v1";
const PSU_HEADROOM_W: f64 = 50.0;
const SINGLE_CATEGORIES: &[&str] = &["cpu", "motherboard", "gpu", "psu", "case", "cooler"];
const KNOWN_CATEGORIES: &[&str] = &[
    "cpu",
    "motherboard",
    "ram",
    "gpu",
    "psu",
    "case",
    "cooler",
    "storage",
];

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Verdict {
    Compatible,
    Incompatible,
    Unknown,
}

impl Verdict {
    fn priority(self) -> u8 {
        match self {
            Verdict::Compatible => 0,
            Verdict::Unknown => 1,
            Verdict::Incompatible => 2,
        }
    }

    fn merge(self, other: Verdict) -> Verdict {
        if other.priority() > self.priority() {
            other
        } else {
            self
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CompletenessStatus {
    Complete,
    Incomplete,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SelectionItem {
    pub sku: String,
    pub quantity: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CheckRequest {
    pub snapshot_id: String,
    #[serde(default)]
    pub selection: Vec<SelectionItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PartRef {
    pub record_id: String,
    pub sku: String,
    pub category: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RuleEvidence {
    pub part: PartRef,
    pub field: String,
    pub value: Option<String>,
    pub provenance: Option<Provenance>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RuleResult {
    pub rule_id: String,
    pub rule_version: String,
    pub title: String,
    pub category: String,
    pub status: Verdict,
    pub parts: Vec<PartRef>,
    pub evidence: Vec<RuleEvidence>,
    pub explanation: String,
    pub follow_up: Option<String>,
    pub method: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MissingPart {
    pub category: String,
    pub reason: String,
    pub follow_up: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Completeness {
    pub status: CompletenessStatus,
    pub missing: Vec<MissingPart>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SelectedPart {
    pub record_id: String,
    pub sku: String,
    pub category: String,
    pub model: String,
    pub quantity: i64,
    pub lots: usize,
    pub available: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CheckSummary {
    pub compatible: usize,
    pub incompatible: usize,
    pub unknown: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CheckReport {
    pub id: String,
    pub snapshot_id: String,
    pub snapshot_revision: i64,
    pub rule_set_version: String,
    pub checked_at: String,
    pub selection: Vec<SelectedPart>,
    pub completeness: Completeness,
    pub overall_status: Verdict,
    pub rules: Vec<RuleResult>,
    pub summary: CheckSummary,
    #[serde(default)]
    pub stale: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_snapshot_revision: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stale_reason: Option<String>,
}

struct Resolved {
    record: Record,
    quantity: i64,
    available: i64,
    lots: usize,
}

#[derive(Default)]
struct BuildParts {
    cpu: Option<Resolved>,
    motherboard: Option<Resolved>,
    gpu: Option<Resolved>,
    psu: Option<Resolved>,
    case_part: Option<Resolved>,
    cooler: Option<Resolved>,
    ram: Vec<Resolved>,
    storage: Vec<Resolved>,
}

/// Validate a selection against one immutable snapshot.
fn resolve_selection(
    snapshot: &Snapshot,
    request: &CheckRequest,
) -> Result<Vec<Resolved>, ApiError> {
    if request.snapshot_id.trim().is_empty() {
        return Err(ApiError::bad_request(
            "snapshot_required",
            "Choose an inventory snapshot first.",
        )
        .with_field("snapshotId"));
    }
    if request.selection.is_empty() {
        return Err(ApiError::bad_request(
            "selection_empty",
            "Select at least one part before checking the build.",
        )
        .with_field("selection"));
    }
    let mut seen = BTreeSet::new();
    let mut resolved = Vec::with_capacity(request.selection.len());
    for item in &request.selection {
        let sku = item.sku.trim();
        if sku.is_empty() || item.quantity <= 0 {
            return Err(ApiError::bad_request(
                "selection_invalid",
                "Every selected part needs a non-empty SKU and a positive whole quantity.",
            )
            .with_field("selection"));
        }
        if !seen.insert(sku.to_string()) {
            return Err(ApiError::bad_request(
                "duplicate_selection",
                format!("SKU {sku} appears more than once in the selection."),
            )
            .with_field("selection")
            .with_details(serde_json::json!({ "sku": sku })));
        }
        let lots: Vec<&Record> = snapshot.records.iter().filter(|r| r.sku == sku).collect();
        let Some(first) = lots.first() else {
            return Err(ApiError::unprocessable(
                "sku_not_found",
                format!("SKU {sku} is not present in this snapshot."),
            )
            .with_details(serde_json::json!({ "sku": sku })));
        };
        if !KNOWN_CATEGORIES.contains(&first.category.as_str()) {
            return Err(ApiError::unprocessable(
                "category_unsupported",
                format!("SKU {sku} has an unsupported category."),
            )
            .with_details(serde_json::json!({ "sku": sku, "category": first.category })));
        }
        if lots.iter().any(|r| {
            r.category != first.category
                || r.model_normalized != first.model_normalized
                || r.specs != first.specs
        }) {
            return Err(ApiError::unprocessable(
                "sku_ambiguous",
                format!(
                    "SKU {sku} appears in this snapshot with conflicting lots. Correct or merge it before checking."
                ),
            )
            .with_details(serde_json::json!({ "sku": sku, "lots": lots.len() })));
        }
        let available = lots.iter().map(|r| r.quantity).sum::<i64>();
        if item.quantity > available {
            return Err(ApiError::unprocessable(
                "insufficient_stock",
                format!(
                    "SKU {sku} needs {} but this snapshot has {available}.",
                    crate::text::plural(item.quantity, "unit", "units")
                ),
            )
            .with_details(
                serde_json::json!({ "sku": sku, "requested": item.quantity, "available": available }),
            ));
        }
        resolved.push(Resolved {
            record: (*first).clone(),
            quantity: item.quantity,
            available,
            lots: lots.len(),
        });
    }
    for category in SINGLE_CATEGORIES {
        let skus: Vec<&str> = resolved
            .iter()
            .filter(|r| r.record.category == *category)
            .map(|r| r.record.sku.as_str())
            .collect();
        if skus.len() > 1 {
            return Err(ApiError::unprocessable(
                "category_multiple",
                format!(
                    "A single build can contain only one {category}; the selection names {}.",
                    skus.join(", ")
                ),
            )
            .with_details(serde_json::json!({ "category": category, "skus": skus })));
        }
    }
    Ok(resolved)
}

fn into_parts(resolved: Vec<Resolved>) -> BuildParts {
    let mut parts = BuildParts::default();
    for mut item in resolved {
        item.record.quantity = item.quantity;
        let slot = match item.record.category.as_str() {
            "cpu" => &mut parts.cpu,
            "motherboard" => &mut parts.motherboard,
            "gpu" => &mut parts.gpu,
            "psu" => &mut parts.psu,
            "case" => &mut parts.case_part,
            "cooler" => &mut parts.cooler,
            "ram" => {
                parts.ram.push(item);
                continue;
            }
            "storage" => {
                parts.storage.push(item);
                continue;
            }
            _ => continue,
        };
        *slot = Some(item);
    }
    parts
}

fn part_ref(part: &Resolved) -> PartRef {
    PartRef {
        record_id: part.record.id.clone(),
        sku: part.record.sku.clone(),
        category: part.record.category.clone(),
        model: part.record.model_normalized.clone(),
    }
}

fn spec_of<'a>(record: &'a Record, key: &str) -> Option<&'a Spec> {
    record
        .specs
        .iter()
        .find(|spec| spec.key == key && spec.status == SpecStatus::Confirmed)
}

fn spec_display(spec: &Spec) -> String {
    if let Some(number) = spec.number {
        let value = if number.fract() == 0.0 {
            format!("{}", number as i64)
        } else {
            format!("{number}")
        };
        return match &spec.unit {
            Some(unit) => format!("{value} {unit}"),
            None => value,
        };
    }
    spec.text.clone().unwrap_or_else(|| spec.raw.clone())
}

fn field_evidence(part: &Resolved, field: &str) -> RuleEvidence {
    let spec = spec_of(&part.record, field);
    RuleEvidence {
        part: part_ref(part),
        field: field.to_string(),
        value: spec.map(spec_display),
        provenance: spec.map(|spec| spec.provenance.clone()),
    }
}

fn model_evidence(part: &Resolved) -> RuleEvidence {
    let value = if part.record.model_normalized.is_empty() {
        part.record.model_raw.clone()
    } else {
        part.record.model_normalized.clone()
    };
    RuleEvidence {
        part: part_ref(part),
        field: "model".to_string(),
        value: Some(value),
        provenance: part.record.provenance.get("model").cloned(),
    }
}

fn text<'a>(record: &'a Record, key: &str) -> Option<&'a str> {
    spec_of(record, key).and_then(|spec| spec.text.as_deref())
}

fn number(record: &Record, key: &str) -> Option<f64> {
    spec_of(record, key).and_then(|spec| {
        spec.number
            .or_else(|| spec.text.as_deref().and_then(parse_leading_number))
    })
}

fn parse_leading_number(value: &str) -> Option<f64> {
    let trimmed = value.trim();
    let end = trimmed
        .char_indices()
        .find(|(_, ch)| !ch.is_ascii_digit() && *ch != '.')
        .map(|(index, _)| index)
        .unwrap_or(trimmed.len());
    if end == 0 {
        return None;
    }
    trimmed[..end].parse::<f64>().ok()
}

fn is_yes(record: &Record, key: &str) -> Option<bool> {
    let value = text(record, key)?;
    Some(matches!(
        value.trim().to_lowercase().as_str(),
        "yes" | "true" | "1" | "confirmed"
    ))
}

fn category_rank(category: &str) -> u8 {
    match category {
        "cpu" => 0,
        "memory" => 1,
        "gpu" => 2,
        "cooling" => 3,
        "case" => 4,
        "power" => 5,
        "storage" => 6,
        _ => 7,
    }
}

#[allow(clippy::too_many_arguments)]
fn rule(
    rule_id: &str,
    title: &str,
    category: &str,
    status: Verdict,
    parts: Vec<PartRef>,
    evidence: Vec<RuleEvidence>,
    explanation: String,
    follow_up: Option<String>,
    method: &str,
) -> RuleResult {
    RuleResult {
        rule_id: rule_id.to_string(),
        rule_version: RULE_SET_VERSION.to_string(),
        title: title.to_string(),
        category: category.to_string(),
        status,
        parts,
        evidence,
        explanation,
        follow_up,
        method: method.to_string(),
    }
}

fn missing_fields(evidence: &[RuleEvidence]) -> Vec<String> {
    evidence
        .iter()
        .filter(|entry| entry.value.is_none())
        .map(|entry| format!("{} on {}", entry.field, entry.part.sku))
        .collect()
}

fn unknown_follow_up(evidence: &[RuleEvidence], question: &str) -> String {
    let missing = missing_fields(evidence);
    if missing.is_empty() {
        question.to_string()
    } else {
        format!("{question} Missing: {}.", missing.join(", "))
    }
}

fn completeness(parts: &BuildParts) -> Completeness {
    let mut missing = Vec::new();
    let required = [
        ("cpu", parts.cpu.is_some(), "A CPU is required.", "a CPU"),
        (
            "motherboard",
            parts.motherboard.is_some(),
            "A motherboard is required.",
            "a motherboard",
        ),
        (
            "ram",
            !parts.ram.is_empty(),
            "Memory is required.",
            "memory",
        ),
        (
            "psu",
            parts.psu.is_some(),
            "A power supply is required.",
            "a power supply",
        ),
        (
            "case",
            parts.case_part.is_some(),
            "A case is required.",
            "a case",
        ),
        (
            "storage",
            !parts.storage.is_empty(),
            "Startup storage is required.",
            "startup storage",
        ),
    ];
    for (category, present, reason, selection_noun) in required {
        if !present {
            missing.push(MissingPart {
                category: category.to_string(),
                reason: reason.to_string(),
                follow_up: format!("Select {selection_noun} from this snapshot."),
            });
        }
    }
    if parts.gpu.is_none() {
        let igpu = parts
            .cpu
            .as_ref()
            .and_then(|cpu| is_yes(&cpu.record, "igpu"));
        if igpu != Some(true) {
            missing.push(MissingPart {
                category: "gpu".to_string(),
                reason: "No graphics card is selected and sourced integrated-graphics evidence is absent."
                    .to_string(),
                follow_up: "Select a graphics card, or record `igpu=yes` on the CPU with its source."
                    .to_string(),
            });
        }
    }
    if parts.cooler.is_none() {
        let bundled = parts
            .cpu
            .as_ref()
            .and_then(|cpu| is_yes(&cpu.record, "bundled_cooler"));
        if bundled != Some(true) {
            missing.push(MissingPart {
                category: "cooler".to_string(),
                reason: "No cooler is selected and sourced bundled-cooling evidence is absent."
                    .to_string(),
                follow_up:
                    "Select a cooler, or record `bundled_cooler=yes` on the CPU with its source."
                        .to_string(),
            });
        }
    }
    Completeness {
        status: if missing.is_empty() {
            CompletenessStatus::Complete
        } else {
            CompletenessStatus::Incomplete
        },
        missing,
    }
}

fn overall_status(rules: &[RuleResult], completeness: &Completeness) -> Verdict {
    let mut status = Verdict::Compatible;
    for rule in rules {
        status = status.merge(rule.status);
    }
    if completeness.status == CompletenessStatus::Incomplete {
        status = status.merge(Verdict::Unknown);
    }
    status
}

fn summary(rules: &[RuleResult]) -> CheckSummary {
    let mut summary = CheckSummary::default();
    for rule in rules {
        match rule.status {
            Verdict::Compatible => summary.compatible += 1,
            Verdict::Incompatible => summary.incompatible += 1,
            Verdict::Unknown => summary.unknown += 1,
        }
    }
    summary
}

/// Check a selection against one immutable snapshot and build a report. The
/// caller persists the report; no snapshot, stock, or rule state is mutated.
pub fn check_build(snapshot: &Snapshot, request: &CheckRequest) -> Result<CheckReport, ApiError> {
    let resolved = resolve_selection(snapshot, request)?;
    let selection: Vec<SelectedPart> = resolved
        .iter()
        .map(|item| SelectedPart {
            record_id: item.record.id.clone(),
            sku: item.record.sku.clone(),
            category: item.record.category.clone(),
            model: item.record.model_normalized.clone(),
            quantity: item.quantity,
            lots: item.lots,
            available: item.available,
        })
        .collect();
    let parts = into_parts(resolved);
    let rules = evaluate_rules(&parts);
    let completeness = completeness(&parts);
    let overall = overall_status(&rules, &completeness);
    Ok(CheckReport {
        id: Uuid::new_v4().to_string(),
        snapshot_id: snapshot.id.clone(),
        snapshot_revision: snapshot.revision,
        rule_set_version: RULE_SET_VERSION.to_string(),
        checked_at: now_rfc3339(),
        selection,
        completeness,
        overall_status: overall,
        summary: summary(&rules),
        rules,
        stale: false,
        current_snapshot_revision: Some(snapshot.revision),
        stale_reason: None,
    })
}

#[cfg(test)]
mod tests {
    use super::rules::evaluate_rules;
    use super::*;
    use crate::domain::{MatchInfo, Provenance};

    fn record(id: &str, category: &str, specs: &str) -> Record {
        let mut provenance = BTreeMap::new();
        provenance.insert(
            "model".to_string(),
            Provenance::user("csv:fixture.csv", "2026-10-31T00:00:00Z"),
        );
        let user = Provenance::user("csv:fixture.csv", "2026-10-31T00:00:00Z");
        Record {
            id: id.to_string(),
            lot: 1,
            sku: id.to_uppercase(),
            category: category.to_string(),
            model_raw: format!("Fixture {category} {id}"),
            model_normalized: format!("Fixture {category} {id}"),
            quantity: 2,
            price_minor: 100,
            currency: "USD".to_string(),
            specs: crate::domain::parse_specs(specs, &user).unwrap(),
            provenance,
            match_info: MatchInfo::default(),
        }
    }

    fn snapshot(records: Vec<Record>) -> Snapshot {
        Snapshot {
            id: "snapshot-1".to_string(),
            created_at: "2026-10-31T00:00:00Z".to_string(),
            revision: 1,
            parent_snapshot_id: None,
            currency: "USD".to_string(),
            source: crate::domain::SnapshotSource {
                filename: "fixture.csv".to_string(),
                sha256: "abc".to_string(),
            },
            note: None,
            totals: Snapshot::totals_for(&records).unwrap(),
            records,
        }
    }

    fn request(skus: &[&str]) -> CheckRequest {
        CheckRequest {
            snapshot_id: "snapshot-1".to_string(),
            selection: skus
                .iter()
                .map(|sku| SelectionItem {
                    sku: sku.to_string(),
                    quantity: 1,
                })
                .collect(),
        }
    }

    fn parts() -> BuildParts {
        BuildParts {
            cpu: Some(Resolved {
                record: record(
                    "cpu",
                    "cpu",
                    "socket=AM5;tdp=105W;peak_power=142W;igpu=no",
                ),
                quantity: 1,
                available: 2,
                lots: 1,
            }),
            motherboard: Some(Resolved {
                record: record(
                    "board",
                    "motherboard",
                    "socket=AM5;memory_type=DDR5;form_factor=ATX;ram_slots=4;max_memory=192GB;bios_supports=Fixture cpu cpu;storage_interface=NVMe;pcie_version=PCIe5",
                ),
                quantity: 1,
                available: 2,
                lots: 1,
            }),
            ram: vec![Resolved {
                record: record("ram", "ram", "memory_type=DDR5;capacity=32GB;modules=2"),
                quantity: 1,
                available: 2,
                lots: 1,
            }],
            gpu: Some(Resolved {
                record: record(
                    "gpu",
                    "gpu",
                    "interface=PCIe4;length=300mm;tdp=220W;peak_power=260W;connector=2x8pin",
                ),
                quantity: 1,
                available: 2,
                lots: 1,
            }),
            psu: Some(Resolved {
                record: record(
                    "psu",
                    "psu",
                    "wattage=850W;psu_connectors=2x8pin;form_factor=ATX;length=160mm",
                ),
                quantity: 1,
                available: 2,
                lots: 1,
            }),
            case_part: Some(Resolved {
                record: record(
                    "case",
                    "case",
                    "supported_form_factors=ATX;max_gpu_length=360mm;max_cooler_height=170mm;psu_form_factor=ATX;max_psu_length=180mm",
                ),
                quantity: 1,
                available: 2,
                lots: 1,
            }),
            cooler: Some(Resolved {
                record: record("cooler", "cooler", "cooler_socket=AM5;height=155mm;type=air"),
                quantity: 1,
                available: 2,
                lots: 1,
            }),
            storage: vec![Resolved {
                record: record("ssd", "storage", "interface=NVMe;capacity=1000GB"),
                quantity: 1,
                available: 2,
                lots: 1,
            }],
        }
    }

    fn find<'a>(rules: &'a [RuleResult], id: &str) -> &'a RuleResult {
        rules.iter().find(|rule| rule.rule_id == id).unwrap()
    }

    #[test]
    fn rules_are_compatible_with_complete_sourced_facts() {
        let parts = parts();
        let rules = evaluate_rules(&parts);
        assert_eq!(rules.len(), 14);
        assert!(rules.iter().all(|rule| rule.status == Verdict::Compatible));
        assert!(rules
            .iter()
            .all(|rule| rule.rule_version == RULE_SET_VERSION));
        assert_eq!(
            overall_status(&rules, &completeness(&parts)),
            Verdict::Compatible
        );
        assert_eq!(completeness(&parts).status, CompletenessStatus::Complete);
    }

    #[test]
    fn socket_conflict_dominates_and_names_both_values() {
        let mut parts = parts();
        parts.motherboard.as_mut().unwrap().record.specs = crate::domain::parse_specs(
            "socket=AM4;memory_type=DDR5;form_factor=ATX;ram_slots=4;max_memory=192GB",
            &Provenance::user("csv:fixture.csv", "2026-10-31T00:00:00Z"),
        )
        .unwrap();
        let rules = evaluate_rules(&parts);
        let socket = find(&rules, "cpu-socket");
        assert_eq!(socket.status, Verdict::Incompatible);
        assert!(socket.explanation.contains("AM5"));
        assert_eq!(
            overall_status(&rules, &completeness(&parts)),
            Verdict::Incompatible
        );
    }

    #[test]
    fn missing_bios_fact_is_unknown_with_a_specific_follow_up() {
        let mut parts = parts();
        parts.motherboard.as_mut().unwrap().record.specs =
            crate::domain::parse_specs(
                "socket=AM5;memory_type=DDR5;form_factor=ATX;ram_slots=4;max_memory=192GB;storage_interface=NVMe;pcie_version=PCIe5",
                &Provenance::user("csv:fixture.csv", "2026-10-31T00:00:00Z"),
            )
            .unwrap();
        let rules = evaluate_rules(&parts);
        let bios = find(&rules, "cpu-bios");
        assert_eq!(bios.status, Verdict::Unknown);
        assert!(bios.follow_up.as_deref().unwrap().contains("BIOS"));
        assert_eq!(
            overall_status(&rules, &completeness(&parts)),
            Verdict::Unknown
        );
        assert_eq!(find(&rules, "cpu-socket").status, Verdict::Compatible);
    }

    #[test]
    fn conflict_beats_a_separate_unknown() {
        let rules = vec![
            rule(
                "a",
                "a",
                "cpu",
                Verdict::Unknown,
                vec![],
                vec![],
                String::new(),
                None,
                "test",
            ),
            rule(
                "b",
                "b",
                "gpu",
                Verdict::Incompatible,
                vec![],
                vec![],
                String::new(),
                None,
                "test",
            ),
        ];
        assert_eq!(
            overall_status(&rules, &completeness(&parts())),
            Verdict::Incompatible
        );
    }

    #[test]
    fn incomplete_build_is_unknown_even_when_rules_pass() {
        let mut parts = parts();
        parts.storage.clear();
        let rules = evaluate_rules(&parts);
        assert!(rules.iter().all(|rule| rule.status == Verdict::Compatible));
        let state = completeness(&parts);
        assert_eq!(state.status, CompletenessStatus::Incomplete);
        assert_eq!(state.missing[0].category, "storage");
        assert_eq!(overall_status(&rules, &state), Verdict::Unknown);
    }

    #[test]
    fn missing_part_sentences_are_grammatical_and_do_not_repeat_the_reason() {
        let state = completeness(&BuildParts::default());
        assert_eq!(state.missing.len(), 8);
        let motherboard = state
            .missing
            .iter()
            .find(|entry| entry.category == "motherboard")
            .unwrap();
        assert_eq!(motherboard.reason, "A motherboard is required.");
        assert_eq!(
            motherboard.follow_up,
            "Select a motherboard from this snapshot."
        );
        for entry in &state.missing {
            assert!(
                entry.reason.ends_with('.'),
                "reason must be a complete sentence: {}",
                entry.reason
            );
            assert!(
                entry.follow_up.starts_with("Select "),
                "follow-up must start with an action: {}",
                entry.follow_up
            );
            assert!(
                !entry.follow_up.contains("is required"),
                "follow-up must not repeat the reason: {}",
                entry.follow_up
            );
        }
        let gpu = state
            .missing
            .iter()
            .find(|entry| entry.category == "gpu")
            .unwrap();
        assert!(gpu.follow_up.contains("igpu=yes"));
        let cooler = state
            .missing
            .iter()
            .find(|entry| entry.category == "cooler")
            .unwrap();
        assert!(cooler.follow_up.contains("bundled_cooler=yes"));
    }

    #[test]
    fn missing_gpu_needs_sourced_integrated_graphics_evidence() {
        let mut parts = parts();
        parts.gpu = None;
        let state = completeness(&parts);
        assert!(state.missing.iter().any(|entry| entry.category == "gpu"));
        parts.cpu.as_mut().unwrap().record.specs = crate::domain::parse_specs(
            "socket=AM5;tdp=105W;peak_power=142W;igpu=yes",
            &Provenance::user("csv:fixture.csv", "2026-10-31T00:00:00Z"),
        )
        .unwrap();
        assert!(!completeness(&parts)
            .missing
            .iter()
            .any(|entry| entry.category == "gpu"));
    }

    #[test]
    fn liquid_cooler_without_radiator_clearance_is_unknown() {
        let mut parts = parts();
        parts.cooler.as_mut().unwrap().record.specs = crate::domain::parse_specs(
            "cooler_socket=AM5;type=liquid;radiator_size=240mm",
            &Provenance::user("csv:fixture.csv", "2026-10-31T00:00:00Z"),
        )
        .unwrap();
        let rules = evaluate_rules(&parts);
        let clearance = find(&rules, "cooler-clearance");
        assert_eq!(clearance.status, Verdict::Unknown);
        assert!(clearance.follow_up.as_deref().unwrap().contains("radiator"));
    }

    #[test]
    fn connector_shortfall_is_a_conflict() {
        let mut parts = parts();
        parts.psu.as_mut().unwrap().record.specs = crate::domain::parse_specs(
            "wattage=850W;psu_connectors=1x8pin;form_factor=ATX;length=160mm",
            &Provenance::user("csv:fixture.csv", "2026-10-31T00:00:00Z"),
        )
        .unwrap();
        let rules = evaluate_rules(&parts);
        assert_eq!(
            find(&rules, "psu-gpu-connectors").status,
            Verdict::Incompatible
        );
    }

    #[test]
    fn nominal_power_without_peak_is_unknown_not_safe() {
        let mut parts = parts();
        parts.cpu.as_mut().unwrap().record.specs = crate::domain::parse_specs(
            "socket=AM5;tdp=105W;igpu=no",
            &Provenance::user("csv:fixture.csv", "2026-10-31T00:00:00Z"),
        )
        .unwrap();
        let rules = evaluate_rules(&parts);
        let power = find(&rules, "psu-capacity");
        assert_eq!(power.status, Verdict::Unknown);
        assert!(power.follow_up.as_deref().unwrap().contains("peak"));
    }

    #[test]
    fn selection_errors_are_stable_and_explicit() {
        let snapshot = snapshot(vec![record("cpu", "cpu", "socket=AM5")]);
        let mut bad = request(&["CPU"]);
        bad.selection[0].quantity = 0;
        assert_eq!(
            check_build(&snapshot, &bad).unwrap_err().code,
            "selection_invalid"
        );
        let empty = CheckRequest {
            snapshot_id: "snapshot-1".to_string(),
            selection: vec![],
        };
        assert_eq!(
            check_build(&snapshot, &empty).unwrap_err().code,
            "selection_empty"
        );
        let mut duplicate = request(&["CPU", "CPU"]);
        duplicate.selection[1].quantity = 1;
        assert_eq!(
            check_build(&snapshot, &duplicate).unwrap_err().code,
            "duplicate_selection"
        );
        let missing = request(&["NOPE"]);
        assert_eq!(
            check_build(&snapshot, &missing).unwrap_err().code,
            "sku_not_found"
        );
        let too_many = request(&["CPU"]);
        let too_many = CheckRequest {
            selection: vec![SelectionItem {
                sku: "CPU".to_string(),
                quantity: 99,
            }],
            ..too_many
        };
        assert_eq!(
            check_build(&snapshot, &too_many).unwrap_err().code,
            "insufficient_stock"
        );
    }
}

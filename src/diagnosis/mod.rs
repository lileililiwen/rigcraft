//! Deterministic diagnosis of an existing build.
//!
//! The operator describes the build they already have — owned parts entered by
//! hand, or a pasted list matched against one immutable snapshot. The described
//! build is evaluated by the same compatibility engine and rule set as
//! `/builds/check`; every reuse or replacement recommendation cites that
//! evidence. Nothing is inferred from a model name: an ambiguous or unmatched
//! pasted line stays ambiguous or unmatched with the evidence that would
//! resolve it, and missing specifications stay `unknown` instead of being
//! defaulted.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::compatibility::{
    check_build, CheckReport, CheckRequest, RuleEvidence, SelectionItem, Verdict,
};
use crate::domain::{
    normalize_category, normalize_model, now_rfc3339, parse_currency, parse_specs,
    totals_overflow_error, MatchInfo, Provenance, Record, Snapshot, SnapshotSource, Spec,
};
use crate::error::ApiError;

mod advice;
mod matching;

pub use advice::{
    Advice, AdviceCost, AdviceGroups, PlanOption, PlanStep, RejectedOption, UpgradePlan,
};
pub use matching::{
    parse_list, LineIssue, MatchCandidate, MatchStatus, ParseListRequest, ParsedLine, ParsedList,
};

pub const MATCH_VERSION: &str = "diagnosis-match-v1";
pub const ADVICE_VERSION: &str = "diagnosis-advice-v1";
/// Pasted lists are bounded so one paste can never become an unbounded import.
pub const MAX_TEXT_BYTES: usize = 8 * 1024;
pub const MAX_LINES: usize = 40;
pub const MAX_PARTS: usize = 40;
pub const MAX_SPEC_BYTES: usize = 2 * 1024;
pub const MAX_CANDIDATES: usize = matching::MAX_CANDIDATES;
/// Candidates the plan evaluates per part. The report states the count, so a
/// shortened search is never presented as an exhaustive one.
pub const MAX_OPTIONS_EVALUATED: usize = 25;
pub const MAX_OPTIONS_SHOWN: usize = 3;
pub const MAX_REJECTED_SHOWN: usize = 5;

const SINGLE_CATEGORIES: &[&str] = &["cpu", "motherboard", "gpu", "psu", "case", "cooler"];

/// One part as submitted. Every field is optional so a field-coded error can
/// name the row instead of failing at deserialization.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BuildPartInput {
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub sku: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub quantity: Option<i64>,
    #[serde(default)]
    pub specs: Option<String>,
    #[serde(default)]
    pub price_minor: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiagnoseRequest {
    #[serde(default)]
    pub snapshot_id: String,
    #[serde(default)]
    pub currency: Option<String>,
    #[serde(default)]
    pub build: Vec<BuildPartInput>,
}

/// Validated part of the described build. `source` records where the facts came
/// from: the operator's description or a snapshot record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DescribedPart {
    pub row: usize,
    pub source: String,
    pub sku: String,
    pub category: String,
    pub model: String,
    pub quantity: i64,
    /// Every described part is already owned, so it adds no purchase cost and
    /// takes no stock. Only replacements are priced.
    pub owned: bool,
    pub reported_price_minor: Option<i64>,
    pub specs: Vec<Spec>,
    pub provenance: BTreeMap<String, Provenance>,
    #[serde(rename = "match")]
    pub match_info: MatchInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosisReport {
    pub id: String,
    pub created_at: String,
    pub snapshot_id: String,
    pub snapshot_revision: i64,
    pub rule_set_version: String,
    pub match_version: String,
    pub advice_version: String,
    pub currency: String,
    pub build: Vec<DescribedPart>,
    /// The compatibility report for the described build. Its `snapshotId` is
    /// the synthetic described-build id, never a purchase snapshot.
    pub check: CheckReport,
    pub advice: Vec<Advice>,
    pub groups: AdviceGroups,
    pub plan: UpgradePlan,
    #[serde(default)]
    pub stale: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_snapshot_revision: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stale_reason: Option<String>,
}

/// Internal, validated part used to build check records.
pub(crate) struct PartSpec {
    pub row: usize,
    pub source: String,
    pub sku: String,
    pub category: String,
    pub model: String,
    pub model_normalized: String,
    pub quantity: i64,
    pub price_minor: i64,
    pub specs: Vec<Spec>,
    pub provenance: BTreeMap<String, Provenance>,
    pub match_info: MatchInfo,
    pub reported_price_minor: Option<i64>,
}

impl PartSpec {
    pub(crate) fn record_id(&self) -> String {
        format!("diagnosis:{}", self.row)
    }

    pub(crate) fn record(&self) -> Record {
        Record {
            id: self.record_id(),
            lot: 1,
            sku: self.sku.clone(),
            category: self.category.clone(),
            model_raw: self.model.clone(),
            model_normalized: self.model_normalized.clone(),
            quantity: self.quantity,
            price_minor: self.price_minor,
            currency: String::new(),
            specs: self.specs.clone(),
            provenance: self.provenance.clone(),
            match_info: self.match_info.clone(),
        }
    }

    fn described(&self) -> DescribedPart {
        DescribedPart {
            row: self.row,
            source: self.source.clone(),
            sku: self.sku.clone(),
            category: self.category.clone(),
            model: self.model.clone(),
            quantity: self.quantity,
            owned: true,
            reported_price_minor: self.reported_price_minor,
            specs: self.specs.clone(),
            provenance: self.provenance.clone(),
            match_info: self.match_info.clone(),
        }
    }
}

fn invalid(code: &str, message: impl Into<String>, field: &str, row: Option<usize>) -> ApiError {
    let error = ApiError::bad_request(code, message).with_field(field);
    match row {
        Some(row) => error.with_row(row),
        None => error,
    }
}

fn part_invalid(row: usize, detail: impl Into<String>) -> ApiError {
    invalid("part_invalid", detail, "build", Some(row))
        .with_details(serde_json::json!({ "row": row }))
}

fn text_field(
    row: usize,
    field: &str,
    value: Option<&str>,
    required: bool,
    max_len: usize,
) -> Result<String, ApiError> {
    let value = value.unwrap_or_default().trim();
    if value.is_empty() {
        if required {
            return Err(part_invalid(row, format!("Every part needs a {field}.")));
        }
        return Ok(String::new());
    }
    if value.chars().count() > max_len {
        return Err(part_invalid(
            row,
            format!("Keep the {field} to {max_len} characters or fewer."),
        ));
    }
    if value.chars().any(char::is_control) {
        return Err(part_invalid(
            row,
            format!("Remove control characters from the {field}."),
        ));
    }
    Ok(value.to_string())
}

/// Group snapshot records by SKU so a repeated SKU with conflicting lots is
/// reported rather than silently collapsed.
fn lots_by_sku(snapshot: &Snapshot) -> BTreeMap<String, Vec<&Record>> {
    let mut lots: BTreeMap<String, Vec<&Record>> = BTreeMap::new();
    for record in &snapshot.records {
        lots.entry(record.sku.clone()).or_default().push(record);
    }
    lots
}

fn take_snapshot_part(
    row: usize,
    input: &BuildPartInput,
    lots: &BTreeMap<String, Vec<&Record>>,
) -> Result<PartSpec, ApiError> {
    let sku = input.sku.as_deref().unwrap_or_default().trim();
    if sku.is_empty() {
        return Err(part_invalid(
            row,
            "A snapshot part needs the SKU of the record it was matched to.",
        ));
    }
    let Some(records) = lots.get(sku) else {
        return Err(ApiError::unprocessable(
            "sku_not_found",
            format!("SKU {sku} is not present in this snapshot."),
        )
        .with_field("build")
        .with_row(row)
        .with_details(serde_json::json!({ "sku": sku })));
    };
    let first = records[0];
    if records.iter().any(|record| {
        record.category != first.category
            || record.model_normalized != first.model_normalized
            || record.specs != first.specs
    }) {
        return Err(ApiError::unprocessable(
            "sku_ambiguous",
            format!(
                "SKU {sku} appears in this snapshot with conflicting lots. Correct or merge it before diagnosing."
            ),
        )
        .with_field("build")
        .with_row(row)
        .with_details(serde_json::json!({ "sku": sku, "lots": records.len() })));
    }
    if let Some(category) = input
        .category
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        match normalize_category(category) {
            Some(normalized) if normalized == first.category => {}
            _ => {
                return Err(part_invalid(
                    row,
                    format!(
                    "The description calls {sku} a {category}, but the snapshot record is a {}.",
                    first.category
                ),
                ))
            }
        }
    }
    let quantity = match input.quantity {
        Some(quantity) if quantity > 0 && quantity <= 1000 => quantity,
        Some(_) => {
            return Err(part_invalid(
                row,
                "Quantity must be a whole number from 1 to 1000.",
            ))
        }
        None => 1,
    };
    Ok(PartSpec {
        row,
        source: "snapshot".to_string(),
        sku: first.sku.clone(),
        category: first.category.clone(),
        model: if first.model_normalized.is_empty() {
            first.model_raw.clone()
        } else {
            first.model_normalized.clone()
        },
        model_normalized: first.model_normalized.clone(),
        quantity,
        price_minor: first.price_minor,
        specs: first.specs.clone(),
        provenance: first.provenance.clone(),
        match_info: first.match_info.clone(),
        reported_price_minor: first.price_minor.into(),
    })
}

fn take_owned_part(
    row: usize,
    input: &BuildPartInput,
    reference: &str,
    observed_at: &str,
) -> Result<PartSpec, ApiError> {
    let raw_category = input.category.as_deref().unwrap_or_default();
    let Some(category) = normalize_category(raw_category) else {
        return Err(part_invalid(
            row,
            if raw_category.trim().is_empty() {
                "Every described part needs a category.".to_string()
            } else {
                format!("`{}` is not a supported category.", raw_category.trim())
            },
        ));
    };
    let model = text_field(row, "model", input.model.as_deref(), true, 200)?;
    let model_normalized = normalize_model(&model);
    let quantity = match input.quantity {
        Some(quantity) if quantity > 0 && quantity <= 1000 => quantity,
        Some(_) => {
            return Err(part_invalid(
                row,
                "Quantity must be a whole number from 1 to 1000.",
            ))
        }
        None => 1,
    };
    let sku = match input.sku.as_deref().map(str::trim) {
        Some(value) if !value.is_empty() => text_field(row, "SKU", Some(value), true, 120)?,
        _ => format!("OWNED-{row:02}"),
    };
    let user = Provenance::user(reference, observed_at);
    let mut provenance = BTreeMap::new();
    provenance.insert("sku".to_string(), user.clone());
    provenance.insert("category".to_string(), user.clone());
    provenance.insert("model".to_string(), user.clone());
    provenance.insert(
        "modelNormalized".to_string(),
        Provenance::normalizer("model-lexical-v1", observed_at),
    );
    provenance.insert("quantity".to_string(), user.clone());
    if let Some(price) = input.price_minor {
        if price < 0 {
            return Err(part_invalid(row, "A reported price cannot be negative."));
        }
        provenance.insert("price".to_string(), user.clone());
    }
    let specs_raw = input.specs.as_deref().unwrap_or_default().trim();
    if specs_raw.len() > MAX_SPEC_BYTES {
        return Err(part_invalid(
            row,
            format!("Keep the specifications to {MAX_SPEC_BYTES} bytes or fewer."),
        ));
    }
    let specs: Vec<Spec> = if specs_raw.is_empty() {
        Vec::new()
    } else {
        parse_specs(specs_raw, &user).map_err(|error| {
            part_invalid(
                row,
                format!("{} {}", error.message, error.suggestion.unwrap_or_default()),
            )
            .with_details(serde_json::json!({ "row": row, "specCode": error.code }))
        })?
    };
    Ok(PartSpec {
        row,
        source: "description".to_string(),
        sku,
        category: category.to_string(),
        model: model.clone(),
        model_normalized,
        quantity,
        price_minor: input.price_minor.unwrap_or(0),
        specs,
        provenance,
        match_info: MatchInfo {
            basis: "user-described part; no hardware catalog match was attempted".to_string(),
            confidence: "unverified".to_string(),
        },
        reported_price_minor: input.price_minor,
    })
}

/// Validate the whole described build. Validation finishes before any check or
/// write, so a rejected request never stores a partial diagnosis.
fn validate_build(
    snapshot: &Snapshot,
    request: &DiagnoseRequest,
) -> Result<(String, Vec<PartSpec>, String), ApiError> {
    if request.snapshot_id.trim().is_empty() {
        return Err(invalid(
            "snapshot_required",
            "Choose an inventory snapshot first.",
            "snapshotId",
            None,
        ));
    }
    let raw_currency = request.currency.as_deref().ok_or_else(|| {
        invalid(
            "currency_required",
            "Choose the currency for this diagnosis.",
            "currency",
            None,
        )
    })?;
    let currency = parse_currency(raw_currency)
        .map_err(|error| invalid(&error.code, error.message, "currency", None))?;
    if currency != snapshot.currency {
        return Err(invalid(
            "currency_mismatch",
            format!(
                "The snapshot uses {}; the diagnosis uses {currency}. One currency per build.",
                snapshot.currency
            ),
            "currency",
            None,
        )
        .with_details(serde_json::json!({
            "snapshotCurrency": snapshot.currency,
            "requestCurrency": currency,
        })));
    }
    if request.build.is_empty() {
        return Err(invalid(
            "build_empty",
            "Describe at least one part of the existing build.",
            "build",
            None,
        ));
    }
    if request.build.len() > MAX_PARTS {
        return Err(invalid(
            "build_too_large",
            format!("Describe at most {MAX_PARTS} parts."),
            "build",
            None,
        )
        .with_details(serde_json::json!({ "limit": MAX_PARTS })));
    }
    let observed_at = now_rfc3339();
    let reference = format!("diagnosis-entry:{observed_at}");
    let lots = lots_by_sku(snapshot);
    let mut parts = Vec::with_capacity(request.build.len());
    for (index, input) in request.build.iter().enumerate() {
        let row = index + 1;
        let source = input.source.trim().to_lowercase();
        match source.as_str() {
            "snapshot" => parts.push(take_snapshot_part(row, input, &lots)?),
            "owned" | "description" => {
                parts.push(take_owned_part(row, input, &reference, &observed_at)?)
            }
            "" => {
                return Err(part_invalid(
                    row,
                    "Say whether this part is matched from the snapshot or described by hand.",
                ))
            }
            other => {
                return Err(part_invalid(
                    row,
                    format!("`{other}` is not a supported part source; use `snapshot` or `owned`."),
                ))
            }
        }
    }
    let mut keys = BTreeSet::new();
    let mut skus = BTreeSet::new();
    for part in &parts {
        let key = (part.category.clone(), part.model_normalized.to_lowercase());
        if !keys.insert(key) {
            return Err(ApiError::bad_request(
                "duplicate_part",
                format!(
                    "Row {} repeats {} `{}` already described in this build.",
                    part.row, part.category, part.model
                ),
            )
            .with_field("build")
            .with_row(part.row)
            .with_details(serde_json::json!({ "row": part.row, "sku": part.sku })));
        }
        if !skus.insert(part.sku.clone()) {
            return Err(ApiError::bad_request(
                "duplicate_part",
                format!(
                    "Row {} reuses SKU {} already used by another part of this build.",
                    part.row, part.sku
                ),
            )
            .with_field("build")
            .with_row(part.row)
            .with_details(serde_json::json!({ "row": part.row, "sku": part.sku })));
        }
    }
    for category in SINGLE_CATEGORIES {
        let rows: Vec<usize> = parts
            .iter()
            .filter(|part| part.category == *category)
            .map(|part| part.row)
            .collect();
        if rows.len() > 1 {
            return Err(ApiError::unprocessable(
                "category_multiple",
                format!(
                    "A single build can contain only one {category}; rows {} describe more than one.",
                    rows.iter()
                        .map(usize::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            )
            .with_field("build")
            .with_details(serde_json::json!({ "category": category, "rows": rows })));
        }
    }
    Ok((currency, parts, observed_at))
}

fn described_snapshot(
    parts: &[PartSpec],
    currency: &str,
    observed_at: &str,
) -> Result<Snapshot, ApiError> {
    let records: Vec<Record> = parts
        .iter()
        .map(|part| {
            let mut record = part.record();
            record.currency = currency.to_string();
            record
        })
        .collect();
    let mut digest = Sha256::new();
    for record in &records {
        digest.update(record.id.as_bytes());
        digest.update(record.sku.as_bytes());
        digest.update(record.category.as_bytes());
        digest.update(record.quantity.to_le_bytes());
    }
    Ok(Snapshot {
        id: "diagnosis:described-build".to_string(),
        created_at: observed_at.to_string(),
        revision: 0,
        parent_snapshot_id: None,
        currency: currency.to_string(),
        source: SnapshotSource {
            filename: "described-build".to_string(),
            sha256: format!("{:x}", digest.finalize()),
        },
        note: Some(
            "Existing build described by the operator; not purchasable inventory.".to_string(),
        ),
        totals: Snapshot::totals_for(&records).map_err(|_| totals_overflow_error())?,
        records,
    })
}

/// Evaluate one described build with the compatibility engine. `replacements`
/// maps a part row to the snapshot record that would take its place.
pub(crate) fn check_described(
    parts: &[PartSpec],
    currency: &str,
    replacements: &BTreeMap<usize, Record>,
) -> Result<CheckReport, ApiError> {
    let mut snapshot = described_snapshot(parts, currency, &now_rfc3339())?;
    let mut records: Vec<Record> = parts
        .iter()
        .map(|part| match replacements.get(&part.row) {
            Some(replacement) => {
                let mut record = replacement.clone();
                record.id = part.record_id();
                record.lot = 1;
                record.quantity = part.quantity;
                record.currency = currency.to_string();
                record
            }
            None => part.record(),
        })
        .collect();
    for record in &mut records {
        record.currency = currency.to_string();
    }
    let selection: Vec<SelectionItem> = records
        .iter()
        .map(|record| SelectionItem {
            sku: record.sku.clone(),
            quantity: record.quantity,
        })
        .collect();
    // The described build is its own snapshot, so a replacement record is
    // spliced in before the engine reads it back by SKU.
    snapshot.records = records;
    snapshot.totals =
        Snapshot::totals_for(&snapshot.records).map_err(|_| totals_overflow_error())?;
    check_build(
        &snapshot,
        &CheckRequest {
            snapshot_id: snapshot.id.clone(),
            selection,
        },
    )
}

/// Diagnose the described build against one snapshot.
pub fn diagnose(
    snapshot: &Snapshot,
    request: &DiagnoseRequest,
) -> Result<DiagnosisReport, ApiError> {
    let (currency, parts, observed_at) = validate_build(snapshot, request)?;
    let check = check_described(&parts, &currency, &BTreeMap::new())?;
    let (advice, groups, plan) = advice::advise(&parts, &check, snapshot, &currency)?;
    Ok(DiagnosisReport {
        id: Uuid::new_v4().to_string(),
        created_at: observed_at,
        snapshot_id: snapshot.id.clone(),
        snapshot_revision: snapshot.revision,
        rule_set_version: check.rule_set_version.clone(),
        match_version: MATCH_VERSION.to_string(),
        advice_version: ADVICE_VERSION.to_string(),
        currency,
        build: parts.iter().map(PartSpec::described).collect(),
        check,
        advice,
        groups,
        plan,
        stale: false,
        current_snapshot_revision: Some(snapshot.revision),
        stale_reason: None,
    })
}

/// Rule identifiers that link a rule to one described part.
pub(crate) fn rule_ids_for(check: &CheckReport, record_id: &str, status: Verdict) -> Vec<String> {
    check
        .rules
        .iter()
        .filter(|rule| rule.status == status)
        .filter(|rule| rule.parts.iter().any(|part| part.record_id == record_id))
        .map(|rule| rule.rule_id.clone())
        .collect()
}

/// Evidence entries that belong to one part across the rules with `status`.
pub(crate) fn evidence_for(
    check: &CheckReport,
    record_id: &str,
    status: Verdict,
) -> Vec<RuleEvidence> {
    let mut seen = BTreeSet::new();
    let mut evidence = Vec::new();
    for rule in check.rules.iter().filter(|rule| rule.status == status) {
        for entry in &rule.evidence {
            if entry.part.record_id != record_id {
                continue;
            }
            let key = format!(
                "{}|{}|{:?}",
                entry.field,
                entry.value.clone().unwrap_or_default(),
                entry.provenance.is_some()
            );
            if seen.insert(key) {
                evidence.push(entry.clone());
            }
        }
    }
    evidence
}

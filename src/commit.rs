//! Explicit row decisions, duplicate resolution, and versioned corrections.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::domain::{
    make_record, normalize_category, normalize_model, normalize_sku, now_rfc3339, parse_specs,
    row_from_values, totals_overflow_error, FieldError, Preview, PreviewRow, Provenance, Record,
    RowValues, Snapshot, SnapshotSource,
};
use crate::error::ApiError;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DecisionAction {
    Accept,
    Exclude,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RowOverrides {
    #[serde(default)]
    pub sku: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub quantity: Option<String>,
    #[serde(default)]
    pub price: Option<String>,
    #[serde(default)]
    pub specs: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RowDecision {
    pub row: usize,
    pub action: DecisionAction,
    #[serde(default)]
    pub overrides: Option<RowOverrides>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DuplicateAction {
    Merge,
    Replace,
    KeepSeparate,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateResolution {
    pub group_id: String,
    pub action: DuplicateAction,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommitRequest {
    pub decisions: Vec<RowDecision>,
    #[serde(default)]
    pub duplicate_resolutions: Vec<DuplicateResolution>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommitResponse {
    pub snapshot_id: String,
    pub snapshot: Snapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecordChange {
    pub record_id: String,
    #[serde(default)]
    pub quantity: Option<i64>,
    #[serde(default)]
    pub price_minor: Option<i64>,
    #[serde(default)]
    pub sku: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub specs: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CorrectionRequest {
    #[serde(default)]
    pub note: Option<String>,
    pub changes: Vec<RecordChange>,
}

fn apply_overrides(values: &RowValues, overrides: Option<&RowOverrides>) -> RowValues {
    let mut next = values.clone();
    if let Some(overrides) = overrides {
        if let Some(value) = &overrides.sku {
            next.sku = value.clone();
        }
        if let Some(value) = &overrides.category {
            next.category = value.clone();
        }
        if let Some(value) = &overrides.model {
            next.model = value.clone();
        }
        if let Some(value) = &overrides.quantity {
            next.quantity = value.clone();
        }
        if let Some(value) = &overrides.price {
            next.price = value.clone();
        }
        if let Some(value) = &overrides.specs {
            next.specs = value.clone();
        }
    }
    next
}

/// Re-evaluate committed rows and apply duplicate decisions. Returns records
/// with new record IDs; nothing is persisted here.
pub fn build_records(preview: &Preview, request: &CommitRequest) -> Result<Vec<Record>, ApiError> {
    let mut by_row: BTreeMap<usize, &RowDecision> = BTreeMap::new();
    for decision in &request.decisions {
        if by_row.insert(decision.row, decision).is_some() {
            return Err(ApiError::unprocessable(
                "duplicate_decision",
                format!("Row {} has more than one decision.", decision.row),
            )
            .with_row(decision.row));
        }
    }
    let known_rows = preview
        .rows
        .iter()
        .map(|row| row.row)
        .collect::<BTreeSet<_>>();
    let unknown = by_row
        .keys()
        .filter(|row| !known_rows.contains(row))
        .copied()
        .collect::<Vec<_>>();
    if !unknown.is_empty() {
        return Err(ApiError::unprocessable(
            "unknown_row",
            "A decision names a row that is not in the preview.",
        )
        .with_details(serde_json::json!({ "rows": unknown })));
    }
    let missing = preview
        .rows
        .iter()
        .filter(|row| !by_row.contains_key(&row.row))
        .map(|row| row.row)
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(ApiError::unprocessable(
            "row_decision_missing",
            "Every preview row needs an explicit accept or exclude decision.",
        )
        .with_details(serde_json::json!({ "rows": missing })));
    }

    let correction_reference = format!("correction:{}", preview.preview_id);
    let reference = format!("csv:{}", preview.filename);
    let mut accepted: Vec<PreviewRow> = Vec::new();
    let mut invalid: Vec<serde_json::Value> = Vec::new();
    for row in &preview.rows {
        let decision = by_row[&row.row];
        if decision.action == DecisionAction::Exclude {
            continue;
        }
        let overrides = decision.overrides.as_ref();
        let values = apply_overrides(&row.values, overrides);
        let observed_at = if overrides.is_some() {
            now_rfc3339()
        } else {
            preview.created_at.clone()
        };
        let evaluated = row_from_values(
            row.row,
            values,
            &preview.currency,
            if overrides.is_some() {
                &correction_reference
            } else {
                &reference
            },
            &observed_at,
            overrides.and_then(|value| value.model.as_ref()).is_none(),
            overrides.and_then(|value| value.specs.as_ref()).is_none(),
        );
        let evaluated = PreviewRow {
            duplicate_group_id: row.duplicate_group_id.clone(),
            ..evaluated
        };
        if !evaluated.errors.is_empty() {
            invalid.push(serde_json::json!({
                "row": row.row,
                "errors": evaluated.errors,
            }));
            continue;
        }
        accepted.push(evaluated);
    }
    if !invalid.is_empty() {
        return Err(ApiError::unprocessable(
            "invalid_row",
            "A row marked for commit is still invalid. Correct it or exclude it.",
        )
        .with_details(serde_json::json!({ "rows": invalid })));
    }

    let mut groups: BTreeMap<String, Vec<PreviewRow>> = BTreeMap::new();
    for row in accepted {
        if let Some(group_id) = row.duplicate_group_id.clone() {
            groups.entry(group_id).or_default().push(row);
        } else {
            groups
                .entry(format!("row-{}", row.row))
                .or_default()
                .push(row);
        }
    }
    let known_groups = preview
        .duplicate_groups
        .iter()
        .map(|group| group.group_id.clone())
        .collect::<BTreeSet<_>>();
    let mut resolution_by_group: BTreeMap<String, DuplicateAction> = BTreeMap::new();
    for resolution in &request.duplicate_resolutions {
        if !known_groups.contains(&resolution.group_id) {
            return Err(ApiError::unprocessable(
                "unknown_duplicate_group",
                format!(
                    "Duplicate group {} does not exist in this preview.",
                    resolution.group_id
                ),
            ));
        }
        if resolution_by_group
            .insert(resolution.group_id.clone(), resolution.action)
            .is_some()
        {
            return Err(ApiError::unprocessable(
                "duplicate_resolution_conflict",
                format!(
                    "Duplicate group {} has more than one resolution.",
                    resolution.group_id
                ),
            ));
        }
    }

    let mut records = Vec::new();
    let mut allowed_duplicate_skus: BTreeSet<String> = BTreeSet::new();
    let mut unresolved: Vec<String> = Vec::new();
    for (group_id, rows) in &groups {
        let is_preview_group = known_groups.contains(group_id);
        let action = resolution_by_group.get(group_id);
        if is_preview_group && rows.len() > 1 && action.is_none() {
            unresolved.push(group_id.clone());
            continue;
        }
        match (is_preview_group, action) {
            (true, Some(DuplicateAction::Merge)) if rows.len() > 1 => {
                let first = &rows[0];
                let price = first.normalized.price_minor;
                let category = &first.normalized.category;
                let model = &first.normalized.model_normalized;
                let mut quantity: i64 = 0;
                for row in rows {
                    if row.normalized.price_minor != price {
                        return Err(ApiError::unprocessable(
                            "duplicate_price_mismatch",
                            "Rows merged under one SKU must share the same price.",
                        )
                        .with_details(serde_json::json!({
                            "groupId": group_id,
                            "rows": rows.iter().map(|row| row.row).collect::<Vec<_>>(),
                        })));
                    }
                    if &row.normalized.category != category
                        || &row.normalized.model_normalized != model
                        || row.specs != first.specs
                    {
                        return Err(ApiError::unprocessable(
                            "duplicate_conflict",
                            "Rows merged under one SKU must describe the same part.",
                        )
                        .with_details(serde_json::json!({
                            "groupId": group_id,
                            "rows": rows.iter().map(|row| row.row).collect::<Vec<_>>(),
                        })));
                    }
                    quantity = quantity
                        .checked_add(row.normalized.quantity.unwrap_or(0))
                        .ok_or_else(|| {
                            ApiError::unprocessable(
                                "quantity_overflow",
                                "Merged quantity is too large to store.",
                            )
                        })?;
                }
                let mut record = make_record(first, 1, Uuid::new_v4().to_string());
                record.quantity = quantity;
                records.push(record);
            }
            (true, Some(DuplicateAction::Replace)) if rows.len() > 1 => {
                let last = rows.last().expect("group is non-empty");
                records.push(make_record(last, 1, Uuid::new_v4().to_string()));
            }
            (true, Some(DuplicateAction::KeepSeparate)) if rows.len() > 1 => {
                for (index, row) in rows.iter().enumerate() {
                    allowed_duplicate_skus.insert(normalize_sku(
                        row.normalized.sku.as_deref().unwrap_or_default(),
                    ));
                    records.push(make_record(
                        row,
                        index as i64 + 1,
                        Uuid::new_v4().to_string(),
                    ));
                }
            }
            _ => {
                let row = rows.first().expect("every accepted row lands in a group");
                records.push(make_record(row, 1, Uuid::new_v4().to_string()));
            }
        }
    }
    if !unresolved.is_empty() {
        return Err(ApiError::unprocessable(
            "duplicate_unresolved",
            "Each duplicated SKU needs an explicit merge, replace, or keep-separate decision.",
        )
        .with_details(serde_json::json!({ "groups": unresolved })));
    }
    if records.is_empty() {
        return Err(ApiError::unprocessable(
            "empty_snapshot",
            "No rows were accepted, so there is no snapshot to commit.",
        ));
    }

    // Overrides can move rows across preview groups. Any remaining duplicate
    // SKU must be one the operator explicitly chose to keep separate.
    let mut per_sku: BTreeMap<String, usize> = BTreeMap::new();
    for record in &records {
        *per_sku.entry(normalize_sku(&record.sku)).or_default() += 1;
    }
    for (sku, count) in per_sku {
        if count > 1 && !allowed_duplicate_skus.contains(&sku) {
            return Err(ApiError::unprocessable(
                "duplicate_unresolved",
                format!("SKU {sku} would appear more than once without a keep-separate decision."),
            )
            .with_details(serde_json::json!({ "sku": sku })));
        }
    }
    Ok(records)
}

pub fn decision_hash(request: &CommitRequest) -> String {
    let mut decisions = request.decisions.clone();
    decisions.sort_by_key(|decision| decision.row);
    let mut resolutions = request.duplicate_resolutions.clone();
    resolutions.sort_by(|left, right| left.group_id.cmp(&right.group_id));
    let canonical = serde_json::json!({
        "decisions": decisions,
        "duplicateResolutions": resolutions,
    });
    let mut hasher = Sha256::new();
    hasher.update(canonical.to_string().as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn snapshot_from_preview(
    preview: &Preview,
    records: Vec<Record>,
    sha256: &str,
) -> Result<Snapshot, ApiError> {
    let totals = Snapshot::totals_for(&records).map_err(|_| totals_overflow_error())?;
    Ok(Snapshot {
        id: Uuid::new_v4().to_string(),
        created_at: now_rfc3339(),
        revision: 1,
        parent_snapshot_id: None,
        currency: preview.currency.clone(),
        source: SnapshotSource {
            filename: preview.filename.clone(),
            sha256: sha256.to_string(),
        },
        note: None,
        records,
        totals,
    })
}

pub fn build_correction(
    snapshot: &Snapshot,
    request: &CorrectionRequest,
) -> Result<Snapshot, ApiError> {
    if request.changes.is_empty() {
        return Err(ApiError::unprocessable(
            "no_changes",
            "Add at least one change before saving a new revision.",
        ));
    }
    let new_id = Uuid::new_v4().to_string();
    let observed_at = now_rfc3339();
    let reference = format!("correction:{new_id}");
    let mut seen = BTreeSet::new();
    let mut changed = 0usize;
    let mut records = snapshot.records.clone();
    for change in &request.changes {
        if !seen.insert(change.record_id.clone()) {
            return Err(ApiError::unprocessable(
                "duplicate_record_change",
                format!("Record {} is changed more than once.", change.record_id),
            ));
        }
        let Some(record) = records
            .iter_mut()
            .find(|record| record.id == change.record_id)
        else {
            return Err(ApiError::not_found(
                "record_not_found",
                format!("Record {} is not in this snapshot.", change.record_id),
            ));
        };
        let mut touched = false;
        if let Some(quantity) = change.quantity {
            if quantity < 0 {
                return Err(ApiError::unprocessable(
                    "quantity_negative",
                    "Quantity cannot be negative.",
                )
                .with_field("quantity"));
            }
            record.quantity = quantity;
            record.provenance.insert(
                "quantity".to_string(),
                Provenance::user(&reference, &observed_at),
            );
            touched = true;
        }
        if let Some(price_minor) = change.price_minor {
            if price_minor < 0 {
                return Err(
                    ApiError::unprocessable("price_negative", "Price cannot be negative.")
                        .with_field("price"),
                );
            }
            record.price_minor = price_minor;
            record.provenance.insert(
                "price".to_string(),
                Provenance::user(&reference, &observed_at),
            );
            touched = true;
        }
        if let Some(sku) = &change.sku {
            let value = sku.trim();
            if value.is_empty() || value.chars().count() > 120 {
                return Err(ApiError::unprocessable(
                    "sku_invalid",
                    "SKU is required and must be 120 characters or fewer.",
                )
                .with_field("sku"));
            }
            record.sku = value.to_string();
            record.provenance.insert(
                "sku".to_string(),
                Provenance::user(&reference, &observed_at),
            );
            touched = true;
        }
        if let Some(category) = &change.category {
            let Some(normalized) = normalize_category(category) else {
                return Err(ApiError::unprocessable(
                    "category_unsupported",
                    format!("`{category}` is not a supported category."),
                )
                .with_field("category"));
            };
            record.category = normalized.to_string();
            record.provenance.insert(
                "category".to_string(),
                Provenance::user(&reference, &observed_at),
            );
            touched = true;
        }
        if let Some(model) = &change.model {
            let value = model.trim();
            if value.is_empty() || value.chars().count() > 200 {
                return Err(ApiError::unprocessable(
                    "model_invalid",
                    "Model is required and must be 200 characters or fewer.",
                )
                .with_field("model"));
            }
            record.model_raw = value.to_string();
            record.model_normalized = normalize_model(value);
            record.provenance.insert(
                "model".to_string(),
                Provenance::user(&reference, &observed_at),
            );
            record.provenance.insert(
                "modelNormalized".to_string(),
                Provenance::normalizer("model-lexical-v1", &observed_at),
            );
            touched = true;
        }
        if let Some(specs) = &change.specs {
            let provenance = Provenance::user(&reference, &observed_at);
            let parsed = parse_specs(specs, &provenance).map_err(|error: FieldError| {
                ApiError::unprocessable(error.code.as_str(), error.message).with_field("specs")
            })?;
            record.specs = parsed;
            record.provenance.insert(
                "specs".to_string(),
                Provenance::user(&reference, &observed_at),
            );
            touched = true;
        }
        if touched {
            changed += 1;
        }
    }
    if changed == 0 {
        return Err(ApiError::unprocessable(
            "no_changes",
            "The request contains no actual field changes.",
        ));
    }
    for record in &mut records {
        record.id = Uuid::new_v4().to_string();
    }
    let totals = Snapshot::totals_for(&records).map_err(|_| totals_overflow_error())?;
    Ok(Snapshot {
        id: new_id,
        created_at: observed_at,
        revision: snapshot.revision + 1,
        parent_snapshot_id: Some(snapshot.id.clone()),
        currency: snapshot.currency.clone(),
        source: snapshot.source.clone(),
        note: request
            .note
            .clone()
            .or_else(|| Some(format!("Correction of revision {}", snapshot.revision))),
        records,
        totals,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::csv_import::{preview_csv, SAMPLE_CSV};

    fn preview() -> Preview {
        preview_csv(
            SAMPLE_CSV.as_bytes(),
            "sample.csv",
            "USD",
            &BTreeMap::new(),
            "preview-1".to_string(),
        )
        .unwrap()
        .preview
    }

    fn accept_all(preview: &Preview) -> CommitRequest {
        CommitRequest {
            decisions: preview
                .rows
                .iter()
                .map(|row| RowDecision {
                    row: row.row,
                    action: DecisionAction::Accept,
                    overrides: None,
                })
                .collect(),
            duplicate_resolutions: Vec::new(),
        }
    }

    #[test]
    fn accepting_all_sample_rows_builds_records() {
        let preview = preview();
        let records = build_records(&preview, &accept_all(&preview)).unwrap();
        assert_eq!(records.len(), 10);
        let cpu = records
            .iter()
            .find(|record| record.sku == "CPU-7800X3D")
            .expect("the sample has a CPU");
        assert_eq!(cpu.provenance["sku"].source, "user");
        assert_eq!(
            cpu.provenance["model"].source,
            "catalog:manufacturer@2026-10-31"
        );
        assert!(cpu.provenance["specs"].reference.starts_with("https://"));
        assert_eq!(cpu.match_info.confidence, "catalog-declared");
    }

    #[test]
    fn operator_override_replaces_the_catalog_claim() {
        let preview = preview();
        let cpu_row = preview
            .rows
            .iter()
            .find(|row| row.normalized.sku.as_deref() == Some("CPU-7800X3D"))
            .expect("the sample has a CPU");
        let mut request = accept_all(&preview);
        request.decisions.iter_mut().for_each(|decision| {
            if decision.row == cpu_row.row {
                decision.overrides = Some(RowOverrides {
                    model: Some("Operator's own CPU name".to_string()),
                    ..RowOverrides::default()
                });
            } else {
                decision.action = DecisionAction::Exclude;
            }
        });
        let records = build_records(&preview, &request).unwrap();
        let record = records.first().expect("one accepted row");
        assert_eq!(record.provenance["model"].source, "user");
        assert!(record.provenance["model"]
            .reference
            .starts_with("correction:"));
        // The specifications were not edited, so their catalog claim remains.
        assert_eq!(
            record.provenance["specs"].source,
            "catalog:manufacturer@2026-10-31"
        );
        assert_eq!(record.match_info.confidence, "catalog-declared");
    }

    #[test]
    fn editing_specs_leaves_the_model_catalog_claim() {
        let preview = preview();
        let cpu_row = preview
            .rows
            .iter()
            .find(|row| row.normalized.sku.as_deref() == Some("CPU-7800X3D"))
            .expect("the sample has a CPU");
        let mut request = accept_all(&preview);
        request.decisions.iter_mut().for_each(|decision| {
            if decision.row == cpu_row.row {
                decision.overrides = Some(RowOverrides {
                    specs: Some("socket=AM5".to_string()),
                    ..RowOverrides::default()
                });
            } else {
                decision.action = DecisionAction::Exclude;
            }
        });
        let records = build_records(&preview, &request).unwrap();
        let record = records.first().expect("one accepted row");
        assert_eq!(record.provenance["specs"].source, "user");
        assert!(record.provenance["specs"]
            .reference
            .starts_with("correction:"));
        assert_eq!(
            record.provenance["model"].source,
            "catalog:manufacturer@2026-10-31"
        );
    }

    #[test]
    fn missing_decisions_are_rejected() {
        let mut request = accept_all(&preview());
        request.decisions.pop();
        let error = build_records(&preview(), &request).unwrap_err();
        assert_eq!(error.code, "row_decision_missing");
    }

    #[test]
    fn every_duplicate_group_needs_a_resolution() {
        let csv = "sku,category,model,quantity,price\nA,cpu,X,1,10\nA,cpu,X,2,10\n";
        let preview = preview_csv(csv.as_bytes(), "d.csv", "USD", &BTreeMap::new(), "p".into())
            .unwrap()
            .preview;
        let error = build_records(&preview, &accept_all(&preview)).unwrap_err();
        assert_eq!(error.code, "duplicate_unresolved");

        let mut request = accept_all(&preview);
        request.duplicate_resolutions = vec![DuplicateResolution {
            group_id: "dup-1".to_string(),
            action: DuplicateAction::Merge,
        }];
        let records = build_records(&preview, &request).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].quantity, 3);
    }

    #[test]
    fn keep_separate_preserves_each_lot_and_replace_takes_the_last() {
        let csv = "sku,category,model,quantity,price\nA,cpu,X,1,10\nA,cpu,X,2,10\n";
        let preview = preview_csv(csv.as_bytes(), "d.csv", "USD", &BTreeMap::new(), "p".into())
            .unwrap()
            .preview;
        let mut request = accept_all(&preview);
        request.duplicate_resolutions = vec![DuplicateResolution {
            group_id: "dup-1".to_string(),
            action: DuplicateAction::KeepSeparate,
        }];
        let records = build_records(&preview, &request).unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].lot, 1);
        assert_eq!(records[1].lot, 2);

        request.duplicate_resolutions[0].action = DuplicateAction::Replace;
        let records = build_records(&preview, &request).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].quantity, 2);
    }

    #[test]
    fn corrections_version_the_old_snapshot() {
        let preview = preview();
        let records = build_records(&preview, &accept_all(&preview)).unwrap();
        let snapshot = snapshot_from_preview(&preview, records, "sha").unwrap();
        let request = CorrectionRequest {
            note: Some("Stock recount".to_string()),
            changes: vec![RecordChange {
                record_id: snapshot.records[0].id.clone(),
                quantity: Some(9),
                price_minor: None,
                sku: None,
                category: None,
                model: None,
                specs: None,
            }],
        };
        let corrected = build_correction(&snapshot, &request).unwrap();
        assert_eq!(corrected.revision, 2);
        assert_eq!(
            corrected.parent_snapshot_id.as_deref(),
            Some(snapshot.id.as_str())
        );
        assert_eq!(corrected.records[0].quantity, 9);
        assert_eq!(snapshot.records[0].quantity, 1);
        assert!(corrected.records[0].provenance["quantity"]
            .reference
            .starts_with("correction:"));
    }

    #[test]
    fn hash_is_stable_for_reordered_requests() {
        let preview = preview();
        let mut request = accept_all(&preview);
        let first = decision_hash(&request);
        request.decisions.reverse();
        assert_eq!(first, decision_hash(&request));
    }
}

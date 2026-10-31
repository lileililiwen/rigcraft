//! Grounding: turn one model reply into a proposal whose every value is either
//! traceable to the sentence, derived from the snapshot, a stated default, or
//! explicitly unresolved. Nothing here writes a price, a specification, a stock
//! figure or a compatibility verdict, and nothing here runs a search.

use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{Map, Value};

use crate::ai::{Ambiguity, CurrentRequest, FieldSource, Proposal, ProposedField, USE_CASES};
use crate::domain::{parse_currency, Snapshot};
use crate::error::ApiError;
use crate::recommend::RecommendRequest;

const ALLOWED_FIELDS: [&str; 6] = [
    "budgetMinor",
    "currency",
    "useCase",
    "inventoryOnly",
    "lockedSkus",
    "excludedSkus",
];
const MAX_EXCERPT_CHARS: usize = 200;

pub struct Interpretation {
    pub proposal: Proposal,
    pub ambiguities: Vec<Ambiguity>,
    pub ignored_fields: Vec<String>,
    pub request: Option<RecommendRequest>,
}

/// One field-level change a revision proposes.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FieldChange {
    pub field: String,
    pub from: Value,
    pub to: Value,
}

struct Claim<T> {
    value: Option<T>,
    excerpt: Option<String>,
}

fn normalize(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn ambiguity(field: &str, code: &str, message: impl Into<String>) -> Ambiguity {
    Ambiguity {
        field: field.to_string(),
        code: code.to_string(),
        message: message.into(),
    }
}

/// Accept a bare JSON object, a fenced block, or one wrapper object.
fn extract_object(content: &str) -> Result<Map<String, Value>, ApiError> {
    let stripped = content
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```");
    let start = stripped.find('{');
    let end = stripped.rfind('}');
    let slice = match (start, end) {
        (Some(start), Some(end)) if end > start => &stripped[start..=end],
        _ => {
            return Err(ApiError::unprocessable(
                "ai_response_invalid",
                "The provider reply contained no JSON object.",
            ))
        }
    };
    let value: Value = serde_json::from_str(slice).map_err(|_| {
        ApiError::unprocessable(
            "ai_response_invalid",
            "The provider reply was not a valid JSON object.",
        )
    })?;
    let Some(mut object) = value.as_object().cloned() else {
        return Err(ApiError::unprocessable(
            "ai_response_invalid",
            "The provider reply was not a JSON object.",
        ));
    };
    let has_allowed = object
        .keys()
        .any(|key| ALLOWED_FIELDS.contains(&key.as_str()));
    if !has_allowed && object.len() == 1 {
        if let Some(inner) = object.values().next().and_then(Value::as_object) {
            object = inner.clone();
        }
    }
    Ok(object)
}

fn claim<T: DeserializeOwned>(object: &Map<String, Value>, key: &str) -> Option<Claim<T>> {
    match object.get(key) {
        None | Some(Value::Null) => None,
        Some(Value::Object(map)) => Some(Claim {
            value: map
                .get("value")
                .and_then(|value| serde_json::from_value(value.clone()).ok()),
            excerpt: map
                .get("excerpt")
                .and_then(Value::as_str)
                .map(str::to_string),
        }),
        Some(other) => Some(Claim {
            value: serde_json::from_value(other.clone()).ok(),
            excerpt: None,
        }),
    }
}

/// A grounded excerpt is a literal substring of the sentence after whitespace
/// normalisation; anything else is rejected rather than trusted.
fn grounded_excerpt(sentence: &str, claim: &Option<String>) -> Option<String> {
    let excerpt = normalize(claim.as_deref()?);
    if excerpt.is_empty() || excerpt.chars().count() > MAX_EXCERPT_CHARS {
        return None;
    }
    sentence.contains(&excerpt).then_some(excerpt)
}

fn mentioned(sentence: &str, needle: &str) -> Option<String> {
    let lowered_sentence = sentence.to_ascii_lowercase();
    lowered_sentence
        .contains(&needle.to_ascii_lowercase())
        .then(|| needle.to_string())
}

fn ground_budget(
    object: &Map<String, Value>,
    sentence: &str,
    ambiguities: &mut Vec<Ambiguity>,
) -> ProposedField<i64> {
    let Some(claim) = claim::<i64>(object, "budgetMinor") else {
        ambiguities.push(ambiguity(
            "budgetMinor",
            "field_unresolved",
            "The sentence did not state a total budget. Enter one before confirming.",
        ));
        return ProposedField::unresolved("No budget was stated.");
    };
    let Some(value) = claim.value else {
        ambiguities.push(ambiguity(
            "budgetMinor",
            "field_unresolved",
            "The interpreted budget was not a whole number of minor units. Enter it explicitly.",
        ));
        return ProposedField::unresolved("The interpreted budget was not a number.");
    };
    if value <= 0 {
        ambiguities.push(ambiguity(
            "budgetMinor",
            "field_unresolved",
            "The interpreted budget was not greater than zero. Enter a positive amount.",
        ));
        return ProposedField::unresolved("The interpreted budget was not positive.");
    }
    match grounded_excerpt(sentence, &claim.excerpt) {
        Some(excerpt) => ProposedField {
            value: Some(value),
            source: FieldSource::Sentence,
            excerpt: Some(excerpt),
            note: None,
        },
        None => {
            ambiguities.push(ambiguity(
                "budgetMinor",
                "ungrounded_value",
                "The interpreted budget could not be traced to the sentence. Enter it explicitly.",
            ));
            ProposedField::unresolved("The budget was not traceable to the sentence.")
        }
    }
}

fn ground_currency(
    object: &Map<String, Value>,
    snapshot: Option<&Snapshot>,
    sentence: &str,
    ambiguities: &mut Vec<Ambiguity>,
) -> ProposedField<String> {
    match claim::<String>(object, "currency") {
        Some(claim) => {
            let Some(raw) = claim.value.as_deref() else {
                return fall_back_to_snapshot_currency(snapshot, ambiguities);
            };
            let Ok(code) = parse_currency(raw) else {
                ambiguities.push(ambiguity(
                    "currency",
                    "field_unresolved",
                    "The interpreted currency was not a three-letter ISO 4217 code. Choose one explicitly.",
                ));
                return ProposedField::unresolved(
                    "The interpreted currency was not an ISO 4217 code.",
                );
            };
            if let Some(snapshot) = snapshot {
                if code != snapshot.currency {
                    ambiguities.push(ambiguity(
                        "currency",
                        "currency_mismatch",
                        format!(
                            "The snapshot uses {}; the sentence named {code}. One currency is used per inventory batch, so choose {}.",
                            snapshot.currency, snapshot.currency
                        ),
                    ));
                    return ProposedField::unresolved("The currency differs from the snapshot.");
                }
            }
            match grounded_excerpt(sentence, &claim.excerpt) {
                Some(excerpt) => ProposedField {
                    value: Some(code),
                    source: FieldSource::Sentence,
                    excerpt: Some(excerpt),
                    note: None,
                },
                None if snapshot.is_some() => ProposedField {
                    value: Some(code),
                    source: FieldSource::Snapshot,
                    excerpt: None,
                    note: Some("Confirmed against the selected snapshot.".to_string()),
                },
                None => {
                    ambiguities.push(ambiguity(
                        "currency",
                        "ungrounded_value",
                        "The interpreted currency could not be traced to the sentence. Choose one explicitly.",
                    ));
                    ProposedField::unresolved("The currency was not traceable to the sentence.")
                }
            }
        }
        None => fall_back_to_snapshot_currency(snapshot, ambiguities),
    }
}

fn fall_back_to_snapshot_currency(
    snapshot: Option<&Snapshot>,
    ambiguities: &mut Vec<Ambiguity>,
) -> ProposedField<String> {
    match snapshot {
        Some(snapshot) => ProposedField {
            value: Some(snapshot.currency.clone()),
            source: FieldSource::Snapshot,
            excerpt: None,
            note: Some("Taken from the selected snapshot.".to_string()),
        },
        None => {
            ambiguities.push(ambiguity(
                "currency",
                "field_unresolved",
                "No snapshot is selected, so the currency is unknown. Choose a snapshot or a currency.",
            ));
            ProposedField::unresolved("The currency needs an explicit choice.")
        }
    }
}

fn ground_use_case(
    object: &Map<String, Value>,
    sentence: &str,
    ambiguities: &mut Vec<Ambiguity>,
) -> ProposedField<String> {
    let Some(claim) = claim::<String>(object, "useCase") else {
        ambiguities.push(ambiguity(
            "useCase",
            "field_unresolved",
            "The sentence did not state a primary use. Choose gaming, office or creation.",
        ));
        return ProposedField::unresolved("No primary use was stated.");
    };
    let Some(raw) = claim.value.as_deref() else {
        ambiguities.push(ambiguity(
            "useCase",
            "field_unresolved",
            "The interpreted use case was not a supported name. Choose gaming, office or creation.",
        ));
        return ProposedField::unresolved("The interpreted use case was not supported.");
    };
    let code = raw.trim().to_lowercase();
    if !USE_CASES.contains(&code.as_str()) {
        ambiguities.push(ambiguity(
            "useCase",
            "field_unresolved",
            format!(
                "`{}` is not a supported use case. Use gaming, office or creation.",
                raw.trim()
            ),
        ));
        return ProposedField::unresolved("The interpreted use case was not supported.");
    }
    match grounded_excerpt(sentence, &claim.excerpt) {
        Some(excerpt) => ProposedField {
            value: Some(code),
            source: FieldSource::Sentence,
            excerpt: Some(excerpt),
            note: None,
        },
        None => {
            ambiguities.push(ambiguity(
                "useCase",
                "ungrounded_value",
                "The interpreted use case could not be traced to the sentence. Choose one explicitly.",
            ));
            ProposedField::unresolved("The use case was not traceable to the sentence.")
        }
    }
}

fn ground_inventory_only(
    object: &Map<String, Value>,
    sentence: &str,
    ambiguities: &mut Vec<Ambiguity>,
) -> ProposedField<bool> {
    let default_field = || {
        ProposedField {
        value: Some(true),
        source: FieldSource::Default,
        excerpt: None,
        note: Some(
            "Inventory-only mode is always on: no licensed catalog is configured, so only this snapshot can be recommended."
                .to_string(),
        ),
    }
    };
    let Some(claim) = claim::<bool>(object, "inventoryOnly") else {
        return default_field();
    };
    match claim.value {
        Some(true) => match grounded_excerpt(sentence, &claim.excerpt) {
            Some(excerpt) => ProposedField {
                value: Some(true),
                source: FieldSource::Sentence,
                excerpt: Some(excerpt),
                note: None,
            },
            None => default_field(),
        },
        Some(false) => {
            if grounded_excerpt(sentence, &claim.excerpt).is_none() {
                return default_field();
            }
            ambiguities.push(ambiguity(
                "inventoryOnly",
                "catalog_unavailable",
                "No licensed hardware catalog is configured, so a build can only use this snapshot. Inventory-only stays on.",
            ));
            ProposedField {
                value: Some(false),
                source: FieldSource::Sentence,
                excerpt: grounded_excerpt(sentence, &claim.excerpt),
                note: Some(
                    "This request cannot run until inventory-only is confirmed.".to_string(),
                ),
            }
        }
        None => default_field(),
    }
}

fn canonical_sku(snapshot: Option<&Snapshot>, raw: &str) -> Option<String> {
    let snapshot = snapshot?;
    snapshot
        .records
        .iter()
        .find(|record| record.sku.eq_ignore_ascii_case(raw.trim()))
        .map(|record| record.sku.clone())
}

fn category_of(snapshot: Option<&Snapshot>, sku: &str) -> Option<String> {
    let snapshot = snapshot?;
    snapshot
        .records
        .iter()
        .find(|record| record.sku == sku)
        .map(|record| record.category.clone())
}

fn ground_sku_list(
    object: &Map<String, Value>,
    key: &str,
    label: &str,
    sentence: &str,
    snapshot: Option<&Snapshot>,
    included_skus: &[String],
    ambiguities: &mut Vec<Ambiguity>,
) -> ProposedField<Vec<String>> {
    let Some(claim) = claim::<Vec<String>>(object, key) else {
        return ProposedField {
            value: Some(Vec::new()),
            source: FieldSource::Default,
            excerpt: None,
            note: Some(format!(
                "The sentence named no part to {label}; no part is added. Edit this field to choose one."
            )),
        };
    };
    let Some(raw_list) = claim.value else {
        ambiguities.push(ambiguity(
            key,
            "field_unresolved",
            format!(
                "The interpreted {key} list was not a list of SKUs. Clear or edit it explicitly."
            ),
        ));
        return ProposedField::unresolved("The interpreted part list was not readable.");
    };
    if raw_list.is_empty() {
        return ProposedField {
            value: Some(Vec::new()),
            source: FieldSource::Default,
            excerpt: None,
            note: Some(format!(
                "The sentence named no part to {label}; no part is added. Edit this field to choose one."
            )),
        };
    }

    let mut kept: Vec<String> = Vec::new();
    let mut from_part_list = false;
    for raw in raw_list {
        let Some(canonical) = canonical_sku(snapshot, &raw) else {
            ambiguities.push(ambiguity(
                key,
                "unknown_sku",
                format!(
                    "`{}` is not in the selected snapshot, so it was dropped rather than invented.",
                    raw.trim()
                ),
            ));
            continue;
        };
        if kept.contains(&canonical) {
            continue;
        }
        if mentioned(sentence, &canonical).is_some() {
            kept.push(canonical);
        } else if included_skus.iter().any(|sku| sku == &canonical) {
            kept.push(canonical);
            from_part_list = true;
        } else {
            ambiguities.push(ambiguity(
                key,
                "ungrounded_value",
                format!(
                    "`{canonical}` was not named in the sentence and no part list was included, so it was dropped."
                ),
            ));
        }
    }

    if kept.is_empty() {
        return ProposedField::unresolved(
            "No listed part could be verified against the snapshot and the sentence. Choose parts explicitly.",
        );
    }

    let excerpt = grounded_excerpt(sentence, &claim.excerpt).or_else(|| {
        if from_part_list {
            None
        } else {
            mentioned(sentence, &kept[0])
        }
    });
    let source = if from_part_list || excerpt.is_none() {
        FieldSource::Snapshot
    } else {
        FieldSource::Sentence
    };
    let note = if from_part_list {
        Some("Some parts were chosen from the part list you included.".to_string())
    } else if source == FieldSource::Snapshot {
        Some("Confirmed against the selected snapshot.".to_string())
    } else {
        None
    };
    ProposedField {
        value: Some(kept),
        source,
        excerpt,
        note,
    }
}

/// Resolve a lock/exclude overlap and a duplicate locked category, reporting
/// each decision instead of silently choosing.
fn resolve_part_conflicts(
    locked: ProposedField<Vec<String>>,
    excluded: ProposedField<Vec<String>>,
    snapshot: Option<&Snapshot>,
    ambiguities: &mut Vec<Ambiguity>,
) -> (ProposedField<Vec<String>>, ProposedField<Vec<String>>) {
    let mut locked = locked;
    let mut excluded = excluded;
    if let (Some(locked_value), Some(excluded_value)) =
        (locked.value.clone(), excluded.value.clone())
    {
        let overlap: Vec<String> = excluded_value
            .into_iter()
            .filter(|sku| locked_value.contains(sku))
            .collect();
        if !overlap.is_empty() {
            ambiguities.push(ambiguity(
                "excludedSkus",
                "locked_excluded_conflict",
                format!(
                    "{} cannot be both kept and excluded; it was kept.",
                    overlap.join(", ")
                ),
            ));
            excluded.value = Some(
                excluded
                    .value
                    .clone()
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|sku| !overlap.contains(sku))
                    .collect(),
            );
        }
    }
    if let Some(locked_value) = locked.value.clone() {
        let mut seen_categories: Vec<String> = Vec::new();
        let mut kept: Vec<String> = Vec::new();
        let mut dropped: Vec<String> = Vec::new();
        for sku in locked_value {
            let category = category_of(snapshot, &sku).unwrap_or_default();
            if !category.is_empty() && seen_categories.contains(&category) {
                dropped.push(sku);
                continue;
            }
            seen_categories.push(category);
            kept.push(sku);
        }
        if !dropped.is_empty() {
            ambiguities.push(ambiguity(
                "lockedSkus",
                "locked_category_conflict",
                format!(
                    "A build contains one part per category; {} was dropped from the kept list.",
                    dropped.join(", ")
                ),
            ));
            locked.value = Some(kept);
        }
    }
    (locked, excluded)
}

fn build_request(proposal: &Proposal, snapshot_id: &str) -> Option<RecommendRequest> {
    let budget_minor = *proposal.budget_minor.value()?;
    let currency = proposal.currency.value()?.clone();
    let use_case = proposal.use_case.value()?.clone();
    if proposal.inventory_only.value() != Some(&true) {
        return None;
    }
    if !proposal.locked_skus.is_resolved() || !proposal.excluded_skus.is_resolved() {
        return None;
    }
    Some(RecommendRequest {
        snapshot_id: snapshot_id.to_string(),
        budget_minor: Some(budget_minor),
        currency: Some(currency),
        use_case: Some(use_case),
        inventory_only: Some(true),
        locked_skus: proposal.locked_skus.value()?.clone(),
        excluded_skus: proposal.excluded_skus.value()?.clone(),
    })
}

/// Ground a model reply against the sentence and the snapshot.
pub fn interpret(
    content: &str,
    sentence: &str,
    snapshot: Option<&Snapshot>,
    snapshot_id: &str,
    included_skus: &[String],
) -> Result<Interpretation, ApiError> {
    let object = extract_object(content)?;
    let mut ignored_fields: Vec<String> = object
        .keys()
        .filter(|key| !ALLOWED_FIELDS.contains(&key.as_str()))
        .cloned()
        .collect();
    ignored_fields.sort();
    let sentence = normalize(sentence);
    let mut ambiguities = Vec::new();

    let proposal = Proposal {
        budget_minor: ground_budget(&object, &sentence, &mut ambiguities),
        currency: ground_currency(&object, snapshot, &sentence, &mut ambiguities),
        use_case: ground_use_case(&object, &sentence, &mut ambiguities),
        inventory_only: ground_inventory_only(&object, &sentence, &mut ambiguities),
        locked_skus: ProposedField {
            value: Some(Vec::new()),
            source: FieldSource::Sentence,
            excerpt: None,
            note: None,
        },
        excluded_skus: ProposedField {
            value: Some(Vec::new()),
            source: FieldSource::Sentence,
            excerpt: None,
            note: None,
        },
    };
    let locked_skus = ground_sku_list(
        &object,
        "lockedSkus",
        "keep",
        &sentence,
        snapshot,
        included_skus,
        &mut ambiguities,
    );
    let excluded_skus = ground_sku_list(
        &object,
        "excludedSkus",
        "exclude",
        &sentence,
        snapshot,
        included_skus,
        &mut ambiguities,
    );
    let (locked_skus, excluded_skus) =
        resolve_part_conflicts(locked_skus, excluded_skus, snapshot, &mut ambiguities);
    let proposal = Proposal {
        locked_skus,
        excluded_skus,
        ..proposal
    };
    let request = build_request(&proposal, snapshot_id);
    if request.is_none() && ambiguities.is_empty() {
        ambiguities.push(ambiguity(
            "request",
            "field_unresolved",
            "The proposal is not yet a complete request. Confirm or edit every field.",
        ));
    }
    Ok(Interpretation {
        proposal,
        ambiguities,
        ignored_fields,
        request,
    })
}

/// Compare a proposal with the confirmed request it revises. Only resolved
/// fields appear, so applying the result cannot silently change anything else.
pub fn field_changes(proposal: &Proposal, current: &CurrentRequest) -> Vec<FieldChange> {
    let mut changes = Vec::new();
    let mut push = |field: &str, from: Value, to: Value| {
        if from != to {
            changes.push(FieldChange {
                field: field.to_string(),
                from,
                to,
            });
        }
    };
    if let Some(value) = proposal.budget_minor.value() {
        push(
            "budgetMinor",
            serde_json::json!(current.budget_minor),
            serde_json::json!(value),
        );
    }
    if let Some(value) = proposal.currency.value() {
        push(
            "currency",
            serde_json::json!(current.currency),
            serde_json::json!(value),
        );
    }
    if let Some(value) = proposal.use_case.value() {
        push(
            "useCase",
            serde_json::json!(current.use_case),
            serde_json::json!(value),
        );
    }
    if proposal.locked_skus.source != FieldSource::Default {
        if let Some(value) = proposal.locked_skus.value() {
            push(
                "lockedSkus",
                serde_json::json!(current.locked_skus),
                serde_json::json!(value),
            );
        }
    }
    if proposal.excluded_skus.source != FieldSource::Default {
        if let Some(value) = proposal.excluded_skus.value() {
            push(
                "excludedSkus",
                serde_json::json!(current.excluded_skus),
                serde_json::json!(value),
            );
        }
    }
    changes
}

/// The request a revision applies: each resolved field replaces the confirmed
/// value, each unresolved field keeps it.
pub fn revised_request(
    proposal: &Proposal,
    current: &CurrentRequest,
    snapshot_id: &str,
) -> Option<RecommendRequest> {
    if proposal.inventory_only.value() == Some(&false) {
        return None;
    }
    let budget_minor = proposal
        .budget_minor
        .value()
        .copied()
        .or(current.budget_minor)?;
    let currency = proposal
        .currency
        .value()
        .cloned()
        .or_else(|| current.currency.clone())?;
    let use_case = proposal
        .use_case
        .value()
        .cloned()
        .or_else(|| current.use_case.clone())?;
    Some(RecommendRequest {
        snapshot_id: snapshot_id.to_string(),
        budget_minor: Some(budget_minor),
        currency: Some(currency),
        use_case: Some(use_case),
        inventory_only: Some(true),
        locked_skus: if proposal.locked_skus.source == FieldSource::Default {
            current.locked_skus.clone()
        } else {
            proposal
                .locked_skus
                .value()
                .cloned()
                .unwrap_or_else(|| current.locked_skus.clone())
        },
        excluded_skus: if proposal.excluded_skus.source == FieldSource::Default {
            current.excluded_skus.clone()
        } else {
            proposal
                .excluded_skus
                .value()
                .cloned()
                .unwrap_or_else(|| current.excluded_skus.clone())
        },
    })
}

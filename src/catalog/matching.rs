//! Deterministic search and three-state matching, including the proposals and
//! conflicts an import commit may apply.

use std::collections::{BTreeMap, BTreeSet};

use crate::domain::{normalize_model, normalize_sku, normalize_spec_key, now_rfc3339};

use super::{
    part_provenance, spec_value, CatalogPart, Conflict, MatchCandidate, MatchResult, MatchRow,
    Proposals, StoredCatalogPart,
};

fn normalized_value(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn operator_specs(raw: &str) -> BTreeMap<String, String> {
    let mut specs = BTreeMap::new();
    for part in raw.split(';') {
        if let Some((key, value)) = part.split_once('=') {
            let key = normalize_spec_key(key);
            if !key.is_empty() {
                specs.insert(key, value.trim().to_string());
            }
        }
    }
    specs
}

fn tokens(texts: &[&str]) -> BTreeSet<String> {
    texts
        .iter()
        .flat_map(|text| text.split(|ch: char| !ch.is_ascii_alphanumeric()))
        .filter(|token| !token.is_empty())
        .map(str::to_lowercase)
        .collect()
}

fn rank_part(
    part: &CatalogPart,
    query: &str,
    query_tokens: &BTreeSet<String>,
) -> Option<(u8, usize)> {
    let normalized_query = query.trim().to_lowercase();
    let sku = normalize_sku(&part.sku).to_lowercase();
    let external = part
        .external_id
        .as_deref()
        .map(|value| normalize_sku(value).to_lowercase());
    if normalized_query == sku || external.as_deref() == Some(normalized_query.as_str()) {
        return Some((0, 0));
    }
    if part.model.to_lowercase().starts_with(&normalized_query) {
        return Some((1, 0));
    }
    let haystack = tokens(&[
        part.sku.as_str(),
        part.external_id.as_deref().unwrap_or_default(),
        part.model.as_str(),
    ]);
    let overlap = query_tokens.intersection(&haystack).count();
    (overlap > 0).then_some((2, overlap))
}

/// Deterministic search order: exact identity, model prefix, token overlap;
/// ties on SKU, model and part ID.
pub fn rank_parts<'a>(parts: &'a [StoredCatalogPart], query: &str) -> Vec<&'a StoredCatalogPart> {
    let query_tokens = tokens(&[query]);
    let mut scored = parts
        .iter()
        .filter_map(|stored| {
            rank_part(&stored.part, query, &query_tokens).map(|rank| (rank, stored))
        })
        .collect::<Vec<_>>();
    scored.sort_by(|(left_rank, left), (right_rank, right)| {
        left_rank
            .0
            .cmp(&right_rank.0)
            .then(right_rank.1.cmp(&left_rank.1))
            .then(left.part.sku.cmp(&right.part.sku))
            .then(left.part.model.cmp(&right.part.model))
            .then(left.part.id.cmp(&right.part.id))
    });
    scored.into_iter().map(|(_, stored)| stored).collect()
}

fn candidate(stored: &StoredCatalogPart) -> MatchCandidate {
    MatchCandidate {
        part: stored.part.clone(),
        dataset: stored.dataset.clone(),
        provenance: part_provenance(&stored.part, &stored.dataset),
    }
}

/// Match every row against the active catalog. Categories are already
/// normalized by the caller; parts and candidates are ordered deterministically.
pub fn match_rows(rows: &[MatchRow], parts: &[StoredCatalogPart]) -> Vec<MatchResult> {
    rows.iter()
        .map(|row| {
            let sku = normalize_sku(&row.sku);
            let mut basis = "none";
            let mut candidates = Vec::new();
            if !sku.is_empty() {
                candidates = parts
                    .iter()
                    .filter(|stored| {
                        stored.part.category == row.category
                            && normalize_sku(&stored.part.sku) == sku
                    })
                    .collect();
                if !candidates.is_empty() {
                    basis = "sku";
                }
            }
            if candidates.is_empty() && !sku.is_empty() {
                candidates = parts
                    .iter()
                    .filter(|stored| {
                        stored.part.category == row.category
                            && stored
                                .part
                                .external_id
                                .as_deref()
                                .map(normalize_sku)
                                .is_some_and(|external| external == sku)
                    })
                    .collect();
                if !candidates.is_empty() {
                    basis = "mpn";
                }
            }
            let model = normalize_model(&row.model);
            if candidates.is_empty() && !model.is_empty() {
                candidates = parts
                    .iter()
                    .filter(|stored| {
                        stored.part.category == row.category
                            && stored.part.model_normalized == model
                    })
                    .collect();
                if !candidates.is_empty() {
                    basis = "model";
                }
            }
            candidates.sort_by(|left, right| {
                left.part
                    .sku
                    .cmp(&right.part.sku)
                    .then(left.part.id.cmp(&right.part.id))
            });

            match candidates.len() {
                0 => MatchResult {
                    row: row.row,
                    status: "unmatched".to_string(),
                    basis: "none".to_string(),
                    candidates: Vec::new(),
                    proposals: Proposals::default(),
                    conflicts: Vec::new(),
                },
                1 => {
                    let stored = candidates[0];
                    let mut proposals = Proposals::default();
                    let mut conflicts = Vec::new();
                    if row.model.trim().is_empty() {
                        proposals.model = Some(stored.part.model.clone());
                    } else if normalize_model(&row.model) != stored.part.model_normalized {
                        conflicts.push(Conflict {
                            field: "model".to_string(),
                            operator_value: row.model.trim().to_string(),
                            catalog_value: stored.part.model.clone(),
                        });
                    }
                    let supplied = operator_specs(&row.specs);
                    for spec in &stored.part.specs {
                        let catalog_value = spec_value(spec);
                        match supplied.get(&spec.key) {
                            None => {
                                proposals.specs.insert(spec.key.clone(), catalog_value);
                            }
                            Some(operator_value)
                                if normalized_value(operator_value)
                                    != normalized_value(&catalog_value) =>
                            {
                                conflicts.push(Conflict {
                                    field: format!("specs.{}", spec.key),
                                    operator_value: operator_value.to_string(),
                                    catalog_value,
                                });
                            }
                            Some(_) => {}
                        }
                    }
                    MatchResult {
                        row: row.row,
                        status: "matched".to_string(),
                        basis: basis.to_string(),
                        candidates: vec![candidate(stored)],
                        proposals,
                        conflicts,
                    }
                }
                _ => MatchResult {
                    row: row.row,
                    status: "ambiguous".to_string(),
                    basis: basis.to_string(),
                    candidates: candidates.iter().map(|stored| candidate(stored)).collect(),
                    proposals: Proposals::default(),
                    conflicts: Vec::new(),
                },
            }
        })
        .collect()
}

/// The import time used when a dataset omits retrieval and verification dates.
pub fn import_timestamp() -> String {
    now_rfc3339()
}

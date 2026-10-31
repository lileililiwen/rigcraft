//! Operator-supplied catalog: the documented document, strict validation,
//! deterministic search, three-state matching and import enrichment.
//!
//! RigCraft never ships or fetches a catalog. The operator uploads a document
//! they are entitled to use; this module refuses anything it cannot store
//! honestly and never guesses a unit, a category or a value.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::domain::{Provenance, Spec};

pub const CATALOG_FORMAT: &str = "rigcraft-catalog";
pub const CATALOG_VERSION: i64 = 1;
pub const MAX_CATALOG_PARTS: usize = 50_000;
pub const MAX_MATCH_ROWS: usize = 500;
pub const DEFAULT_SEARCH_LIMIT: usize = 20;
pub const MAX_SEARCH_LIMIT: usize = 100;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CatalogDataset {
    pub id: String,
    pub name: String,
    pub source_url: Option<String>,
    pub commit: Option<String>,
    pub license: String,
    pub retrieved_at: Option<String>,
    pub imported_at: String,
    pub version: String,
    pub row_count: i64,
    pub category_counts: BTreeMap<String, i64>,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CatalogPart {
    pub id: String,
    pub dataset_id: String,
    pub external_id: Option<String>,
    pub sku: String,
    pub category: String,
    pub model: String,
    pub model_normalized: String,
    pub manufacturer: Option<String>,
    pub specs: Vec<Spec>,
    pub source_url: Option<String>,
    pub verified_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StoredCatalogPart {
    pub part: CatalogPart,
    pub dataset: CatalogDataset,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RowProblem {
    pub row: usize,
    pub field: String,
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

impl RowProblem {
    fn new(row: usize, field: &str, code: &str, message: impl Into<String>) -> Self {
        Self {
            row,
            field: field.to_string(),
            code: code.to_string(),
            message: message.into(),
            suggestion: None,
        }
    }

    fn suggest(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }
}

/// One validated part. Specifications keep the already-parsed `Spec` shape with
/// a placeholder provenance; persistence replaces it with the dataset's.
#[derive(Debug, Clone)]
pub struct ValidatedPart {
    pub external_id: Option<String>,
    pub sku: String,
    pub category: String,
    pub model: String,
    pub model_normalized: String,
    pub manufacturer: Option<String>,
    pub source_url: Option<String>,
    pub verified_at: Option<String>,
    pub specs: Vec<Spec>,
}

#[derive(Debug, Clone)]
pub struct ValidatedCatalog {
    pub name: String,
    pub source_url: Option<String>,
    pub commit: Option<String>,
    pub license: String,
    pub retrieved_at: Option<String>,
    pub parts: Vec<ValidatedPart>,
    pub category_counts: BTreeMap<String, i64>,
}

impl ValidatedCatalog {
    /// Version embedded in `catalog:<name>@<version>`: the operator's commit if
    /// recorded, else the retrieval time, else the import date.
    pub fn version_label(&self, imported_at: &str) -> String {
        self.commit
            .clone()
            .or_else(|| self.retrieved_at.clone())
            .unwrap_or_else(|| imported_at.chars().take(10).collect())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CatalogReport {
    pub accepted: bool,
    pub replaced: bool,
    pub dataset: CatalogDataset,
    pub part_count: i64,
    pub category_counts: BTreeMap<String, i64>,
    pub spec_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MatchRow {
    pub row: usize,
    pub sku: String,
    pub category: String,
    pub model: String,
    #[serde(default)]
    pub specs: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Proposals {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default)]
    pub specs: BTreeMap<String, String>,
}

impl Proposals {
    pub fn is_empty(&self) -> bool {
        self.model.is_none() && self.specs.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Conflict {
    pub field: String,
    pub operator_value: String,
    pub catalog_value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MatchCandidate {
    pub part: CatalogPart,
    pub dataset: CatalogDataset,
    pub provenance: Provenance,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MatchResult {
    pub row: usize,
    pub status: String,
    pub basis: String,
    pub candidates: Vec<MatchCandidate>,
    pub proposals: Proposals,
    pub conflicts: Vec<Conflict>,
}

/// The catalog facts a commit may add to one row. Only missing fields are used.
#[derive(Debug, Clone, PartialEq)]
pub struct CatalogApplication {
    pub part_id: String,
    pub provenance: Provenance,
    pub model: String,
    pub specs: BTreeMap<String, String>,
}

impl CatalogApplication {
    pub fn from_stored(stored: &StoredCatalogPart) -> Self {
        Self {
            part_id: stored.part.id.clone(),
            provenance: part_provenance(&stored.part, &stored.dataset),
            model: stored.part.model.clone(),
            specs: stored
                .part
                .specs
                .iter()
                .map(|spec| (spec.key.clone(), spec_value(spec)))
                .collect(),
        }
    }
}

/// The one provenance a catalog part's facts carry.
pub fn part_provenance(part: &CatalogPart, dataset: &CatalogDataset) -> Provenance {
    let reference = part
        .source_url
        .clone()
        .or_else(|| dataset.source_url.clone())
        .unwrap_or_else(|| format!("catalog:{}", dataset.name));
    let observed_at = part
        .verified_at
        .clone()
        .or_else(|| dataset.retrieved_at.clone())
        .unwrap_or_else(|| dataset.imported_at.clone());
    Provenance::catalog(
        format!("catalog:{}@{}", dataset.name, dataset.version),
        reference,
        observed_at,
    )
}

/// Every field's provenance for one stored part.
pub fn part_provenance_map(
    part: &CatalogPart,
    dataset: &CatalogDataset,
) -> BTreeMap<String, Provenance> {
    let catalog = part_provenance(part, dataset);
    let mut map = BTreeMap::new();
    for field in ["sku", "category", "model", "manufacturer", "specs"] {
        map.insert(field.to_string(), catalog.clone());
    }
    map.insert(
        "modelNormalized".to_string(),
        Provenance::normalizer("model-lexical-v1", &catalog.observed_at),
    );
    map
}

fn spec_value(spec: &Spec) -> String {
    match spec.raw.split_once('=') {
        Some((_, value)) => value.trim().to_string(),
        None => spec.text.clone().unwrap_or_default(),
    }
}

mod matching;
mod validate;

pub use matching::{import_timestamp, match_rows, rank_parts};
pub use validate::validate_catalog;

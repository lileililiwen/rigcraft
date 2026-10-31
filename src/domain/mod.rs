//! Inventory domain types, validation, and deterministic normalization.
//!
//! No third-party hardware catalog is queried. A row may declare a catalog
//! source, which is recorded as provenance and never verified; normalization is
//! lexical only, every specification carries its source tag, and a value that
//! was not supplied stays unknown rather than being guessed.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use time::format_description::well_known::Rfc3339;
use time::{Duration, OffsetDateTime};

use crate::error::ApiError;

pub const MAX_CSV_BYTES: usize = 10 * 1024 * 1024;
pub const PREVIEW_TTL_HOURS: i64 = 24;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FieldError {
    pub field: String,
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

impl FieldError {
    pub fn new(field: &str, code: &str, message: impl Into<String>) -> Self {
        Self {
            field: field.to_string(),
            code: code.to_string(),
            message: message.into(),
            suggestion: None,
        }
    }

    pub fn suggest(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Provenance {
    /// `user`, `rigcraft-normalizer` or `licensed-source`.
    pub source: String,
    pub reference: String,
    pub observed_at: String,
}

impl Provenance {
    pub fn user(reference: impl Into<String>, observed_at: impl Into<String>) -> Self {
        Self {
            source: "user".to_string(),
            reference: reference.into(),
            observed_at: observed_at.into(),
        }
    }

    pub fn normalizer(reference: impl Into<String>, observed_at: impl Into<String>) -> Self {
        Self {
            source: "rigcraft-normalizer".to_string(),
            reference: reference.into(),
            observed_at: observed_at.into(),
        }
    }

    pub fn catalog(
        source: impl Into<String>,
        reference: impl Into<String>,
        observed_at: impl Into<String>,
    ) -> Self {
        Self {
            source: source.into(),
            reference: reference.into(),
            observed_at: observed_at.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SpecStatus {
    Confirmed,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Spec {
    pub key: String,
    pub raw: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub number: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    pub status: SpecStatus,
    pub provenance: Provenance,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MatchInfo {
    /// What the normalized value was derived from, in words.
    pub basis: String,
    /// `unverified` until a licensed catalog match is recorded.
    pub confidence: String,
}

impl Default for MatchInfo {
    fn default() -> Self {
        Self {
            basis: "lexical: trimmed and whitespace collapsed; no hardware catalog consulted"
                .to_string(),
            confidence: "unverified".to_string(),
        }
    }
}

/// Raw string values for one CSV row. Corrections reuse the same shape.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RowValues {
    pub sku: String,
    pub category: String,
    pub model: String,
    pub quantity: String,
    pub price: String,
    pub currency: String,
    pub specs: String,
    /// Declared provenance for the model and specifications: empty, `user`, or
    /// `catalog:<id>`. Operator stock facts never use it.
    #[serde(default)]
    pub source: String,
    /// Per-row reference for a declared catalog source.
    #[serde(default)]
    pub source_url: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedRow {
    pub sku: Option<String>,
    pub category: Option<String>,
    pub model_normalized: Option<String>,
    pub quantity: Option<i64>,
    pub price_minor: Option<i64>,
    pub currency: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EvaluatedRow {
    pub normalized: NormalizedRow,
    pub specs: Vec<Spec>,
    pub errors: Vec<FieldError>,
    pub provenance: BTreeMap<String, Provenance>,
    pub match_info: MatchInfo,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RowStatus {
    Valid,
    Error,
    Duplicate,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PreviewRow {
    pub row: usize,
    pub status: RowStatus,
    pub values: RowValues,
    pub normalized: NormalizedRow,
    pub specs: Vec<Spec>,
    pub errors: Vec<FieldError>,
    pub duplicate_group_id: Option<String>,
    pub provenance: BTreeMap<String, Provenance>,
    #[serde(rename = "match")]
    pub match_info: MatchInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateGroup {
    pub group_id: String,
    pub sku: String,
    pub rows: Vec<usize>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PreviewSummary {
    pub total: usize,
    pub valid: usize,
    pub errors: usize,
    pub duplicate_rows: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub preview_id: String,
    pub created_at: String,
    pub expires_at: String,
    pub currency: String,
    pub filename: String,
    pub columns: Vec<String>,
    pub mapping: BTreeMap<String, String>,
    pub rows: Vec<PreviewRow>,
    pub duplicate_groups: Vec<DuplicateGroup>,
    pub summary: PreviewSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotSource {
    pub filename: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotTotals {
    pub record_count: usize,
    pub unit_count: i64,
    pub value_minor: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    pub id: String,
    pub lot: i64,
    pub sku: String,
    pub category: String,
    pub model_raw: String,
    pub model_normalized: String,
    pub quantity: i64,
    pub price_minor: i64,
    pub currency: String,
    pub specs: Vec<Spec>,
    pub provenance: BTreeMap<String, Provenance>,
    #[serde(rename = "match")]
    pub match_info: MatchInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub id: String,
    pub created_at: String,
    pub revision: i64,
    pub parent_snapshot_id: Option<String>,
    pub currency: String,
    pub source: SnapshotSource,
    pub note: Option<String>,
    pub records: Vec<Record>,
    pub totals: SnapshotTotals,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotSummary {
    pub id: String,
    pub created_at: String,
    pub revision: i64,
    pub parent_snapshot_id: Option<String>,
    pub currency: String,
    pub source_filename: String,
    pub record_count: usize,
    pub unit_count: i64,
    pub value_minor: i64,
    pub note: Option<String>,
}

/// A line total, unit count, or snapshot value that cannot be represented in a
/// signed 64-bit integer. Callers reject the write instead of saturating.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TotalsOverflow;

impl Snapshot {
    pub fn totals_for(records: &[Record]) -> Result<SnapshotTotals, TotalsOverflow> {
        let mut totals = SnapshotTotals {
            record_count: records.len(),
            ..SnapshotTotals::default()
        };
        for record in records {
            let line_total = record
                .price_minor
                .checked_mul(record.quantity)
                .ok_or(TotalsOverflow)?;
            totals.unit_count = totals
                .unit_count
                .checked_add(record.quantity)
                .ok_or(TotalsOverflow)?;
            totals.value_minor = totals
                .value_minor
                .checked_add(line_total)
                .ok_or(TotalsOverflow)?;
        }
        Ok(totals)
    }
}

/// The documented 422 for an amount that cannot be represented.
pub fn totals_overflow_error() -> ApiError {
    ApiError::unprocessable(
        "price_overflow",
        "A line total or the snapshot total is too large to store. Lower the price or quantity.",
    )
    .with_field("price")
}

pub fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

pub fn expires_at(created_at: &str) -> String {
    match OffsetDateTime::parse(created_at, &Rfc3339) {
        Ok(created) => (created + Duration::hours(PREVIEW_TTL_HOURS))
            .format(&Rfc3339)
            .unwrap_or_else(|_| created_at.to_string()),
        Err(_) => created_at.to_string(),
    }
}

pub fn preview_is_expired(created_at: &str, now: OffsetDateTime) -> bool {
    match OffsetDateTime::parse(created_at, &Rfc3339) {
        Ok(created) => created + Duration::hours(PREVIEW_TTL_HOURS) <= now,
        // An unreadable timestamp is treated as expired rather than trusted.
        Err(_) => true,
    }
}

const CATEGORY_ALIASES: &[(&str, &[&str])] = &[
    ("cpu", &["cpu", "processor"]),
    (
        "motherboard",
        &["motherboard", "mainboard", "mobo", "mb", "mother board"],
    ),
    ("ram", &["ram", "memory", "dimm", "memory kit"]),
    (
        "gpu",
        &["gpu", "graphics card", "graphics", "video card", "vga"],
    ),
    ("psu", &["psu", "power supply", "power", "psu unit"]),
    ("case", &["case", "chassis", "tower", "pc case"]),
    (
        "cooler",
        &["cooler", "cpu cooler", "heatsink", "aio", "liquid cooler"],
    ),
    ("storage", &["storage", "ssd", "hdd", "nvme", "hard drive"]),
];

pub fn normalize_category(raw: &str) -> Option<&'static str> {
    let value = raw.trim().to_lowercase();
    if value.is_empty() {
        return None;
    }
    CATEGORY_ALIASES
        .iter()
        .find(|(_, aliases)| aliases.iter().any(|alias| *alias == value))
        .map(|(canonical, _)| *canonical)
}

pub fn supported_categories() -> String {
    CATEGORY_ALIASES
        .iter()
        .map(|(canonical, _)| *canonical)
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn normalize_model(raw: &str) -> String {
    raw.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn normalize_sku(raw: &str) -> String {
    raw.trim().to_uppercase()
}

fn validate_text(field: &str, raw: &str, max_len: usize) -> Result<String, FieldError> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(FieldError::new(
            field,
            "required",
            "This field is required.",
        ));
    }
    if value.chars().count() > max_len {
        return Err(FieldError::new(
            field,
            "too_long",
            format!("Keep this field to {max_len} characters or fewer."),
        ));
    }
    if value.chars().any(|ch| ch.is_control()) {
        return Err(FieldError::new(
            field,
            "control_character",
            "Remove control characters from this field.",
        ));
    }
    Ok(value.to_string())
}

pub fn parse_currency(raw: &str) -> Result<String, FieldError> {
    let value = raw.trim().to_uppercase();
    if value.is_empty() {
        return Err(
            FieldError::new("currency", "required", "A batch currency is required.")
                .suggest("Use an ISO 4217 code such as USD, EUR, CNY or JPY."),
        );
    }
    let valid = value.len() == 3 && value.chars().all(|ch| ch.is_ascii_uppercase());
    if !valid {
        return Err(FieldError::new(
            "currency",
            "currency_format",
            "Currency must be a three-letter ISO 4217 code.",
        )
        .suggest("Use an ISO 4217 code such as USD, EUR, CNY or JPY."));
    }
    Ok(value)
}

pub fn parse_quantity(raw: &str) -> Result<i64, FieldError> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(FieldError::new(
            "quantity",
            "required",
            "Quantity is required.",
        ));
    }
    if let Some(rest) = value.strip_prefix('-') {
        let _ = rest;
        return Err(FieldError::new(
            "quantity",
            "quantity_negative",
            "Quantity cannot be negative.",
        )
        .suggest("Enter 0 for unavailable stock."));
    }
    if !value.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(FieldError::new(
            "quantity",
            "quantity_format",
            "Quantity must be a whole number.",
        )
        .suggest("Enter a whole number such as 3."));
    }
    value.parse::<i64>().map_err(|_| {
        FieldError::new(
            "quantity",
            "quantity_overflow",
            "Quantity is too large to store.",
        )
    })
}

pub fn parse_money_minor(raw: &str) -> Result<i64, FieldError> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(FieldError::new(
            "price",
            "required",
            "Sale price is required.",
        ));
    }
    if value.starts_with('-') {
        return Err(FieldError::new(
            "price",
            "price_negative",
            "Price cannot be negative.",
        ));
    }
    if value.matches('.').count() > 1 || value.starts_with('.') {
        return Err(FieldError::new(
            "price",
            "price_format",
            "Price must be a number with at most two decimals.",
        )
        .suggest("Write 1299 or 1299.50 without currency symbols or separators."));
    }
    let (whole, fraction) = match value.split_once('.') {
        Some((whole, fraction)) => (whole, fraction),
        None => (value, ""),
    };
    if whole.is_empty() || !whole.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(FieldError::new(
            "price",
            "price_format",
            "Price must be a number with at most two decimals.",
        )
        .suggest("Write 1299 or 1299.50 without currency symbols or separators."));
    }
    if fraction.len() > 2 || !fraction.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(FieldError::new(
            "price",
            "price_precision",
            "Price supports at most two decimal places.",
        )
        .suggest("Round the price to whole minor units, for example 1299.50."));
    }
    let whole_value = whole
        .parse::<i128>()
        .map_err(|_| FieldError::new("price", "price_overflow", "Price is too large to store."))?;
    let fraction_value = if fraction.is_empty() {
        0
    } else if fraction.len() == 1 {
        fraction.parse::<i128>().unwrap_or(0) * 10
    } else {
        fraction.parse::<i128>().unwrap_or(0)
    };
    let minor = whole_value
        .checked_mul(100)
        .and_then(|value| value.checked_add(fraction_value))
        .ok_or_else(|| {
            FieldError::new("price", "price_overflow", "Price is too large to store.")
        })?;
    i64::try_from(minor)
        .map_err(|_| FieldError::new("price", "price_overflow", "Price is too large to store."))
}

mod specs;
pub use specs::parse_specs;

mod provenance;
use provenance::{catalog_provenance, match_info_for};
/// Validate one row's raw values. The result is deterministic for the same
/// input, batch currency, and timestamp.
pub fn evaluate_row(
    values: &RowValues,
    batch_currency: &str,
    reference: &str,
    observed_at: &str,
) -> EvaluatedRow {
    evaluate_row_scoped(values, batch_currency, reference, observed_at, true, true)
}

/// `model_catalog` and `specs_catalog` say whether each fact still comes from the
/// row's declared catalog source. The commit path clears one of them when the
/// operator edits that field, so an edited fact is never labelled as catalog.
fn evaluate_row_scoped(
    values: &RowValues,
    batch_currency: &str,
    reference: &str,
    observed_at: &str,
    model_catalog: bool,
    specs_catalog: bool,
) -> EvaluatedRow {
    let mut errors = Vec::new();
    let mut normalized = NormalizedRow::default();
    let mut provenance = BTreeMap::new();
    let user = Provenance::user(reference, observed_at);
    let (catalog, source_errors) = catalog_provenance(values, reference, observed_at);
    errors.extend(source_errors);
    // Catalog facts are the model and the specifications. Stock facts stay the
    // operator's even when the row declares a catalog source.
    let model_facts = if model_catalog {
        catalog.as_ref().unwrap_or(&user)
    } else {
        &user
    };
    let spec_facts = if specs_catalog {
        catalog.as_ref().unwrap_or(&user)
    } else {
        &user
    };

    match validate_text("sku", &values.sku, 120) {
        Ok(sku) => {
            normalized.sku = Some(sku.clone());
            provenance.insert("sku".to_string(), user.clone());
        }
        Err(error) => errors.push(error),
    }

    match normalize_category(&values.category) {
        Some(category) => {
            normalized.category = Some(category.to_string());
            provenance.insert(
                "category".to_string(),
                Provenance::normalizer("category-alias-v1", observed_at),
            );
        }
        None if values.category.trim().is_empty() => errors.push(
            FieldError::new("category", "required", "Category is required.")
                .suggest(format!("Use one of: {}.", supported_categories())),
        ),
        None => errors.push(
            FieldError::new(
                "category",
                "category_unsupported",
                format!("`{}` is not a supported category.", values.category.trim()),
            )
            .suggest(format!("Use one of: {}.", supported_categories())),
        ),
    }

    match validate_text("model", &values.model, 200) {
        Ok(model) => {
            normalized.model_normalized = Some(normalize_model(&model));
            provenance.insert("model".to_string(), model_facts.clone());
            provenance.insert(
                "modelNormalized".to_string(),
                Provenance::normalizer("model-lexical-v1", observed_at),
            );
        }
        Err(error) => errors.push(error),
    }

    match parse_quantity(&values.quantity) {
        Ok(quantity) => {
            normalized.quantity = Some(quantity);
            provenance.insert("quantity".to_string(), user.clone());
        }
        Err(error) => errors.push(error),
    }

    match parse_money_minor(&values.price) {
        Ok(price_minor) => {
            normalized.price_minor = Some(price_minor);
            provenance.insert("price".to_string(), user.clone());
        }
        Err(error) => errors.push(error),
    }

    let row_currency = values.currency.trim();
    let batch = batch_currency.trim().to_uppercase();
    if row_currency.is_empty() {
        normalized.currency = Some(batch.clone());
        provenance.insert("currency".to_string(), user.clone());
    } else {
        match parse_currency(row_currency) {
            Ok(currency) if currency == batch => {
                normalized.currency = Some(currency);
                provenance.insert("currency".to_string(), user.clone());
            }
            Ok(currency) => errors.push(
                FieldError::new(
                    "currency",
                    "currency_mismatch",
                    format!("Row currency {currency} does not match batch currency {batch}."),
                )
                .suggest("Remove the row currency or import one currency per batch."),
            ),
            Err(error) => errors.push(error),
        }
    }

    let specs = match parse_specs(&values.specs, spec_facts) {
        Ok(specs) => {
            if !values.specs.trim().is_empty() {
                provenance.insert("specs".to_string(), spec_facts.clone());
            }
            specs
        }
        Err(error) => {
            errors.push(error);
            Vec::new()
        }
    };

    EvaluatedRow {
        normalized,
        specs,
        errors,
        provenance,
        match_info: match_info_for(catalog.as_ref().filter(|_| model_catalog || specs_catalog)),
    }
}

pub fn make_record(row: &PreviewRow, lot: i64, record_id: String) -> Record {
    let normalized = &row.normalized;
    Record {
        id: record_id,
        lot,
        sku: normalized
            .sku
            .clone()
            .unwrap_or_else(|| row.values.sku.trim().to_string()),
        category: normalized.category.clone().unwrap_or_default(),
        model_raw: row.values.model.trim().to_string(),
        model_normalized: normalized.model_normalized.clone().unwrap_or_default(),
        quantity: normalized.quantity.unwrap_or(0),
        price_minor: normalized.price_minor.unwrap_or(0),
        currency: normalized
            .currency
            .clone()
            .unwrap_or_else(|| row.values.currency.to_uppercase()),
        specs: row.specs.clone(),
        provenance: row.provenance.clone(),
        match_info: row.match_info.clone(),
    }
}

/// Used by the commit path to reject a row whose corrected values are still
/// invalid, with the same field codes the preview reports.
pub fn row_from_values(
    row: usize,
    values: RowValues,
    batch_currency: &str,
    reference: &str,
    observed_at: &str,
    model_catalog: bool,
    specs_catalog: bool,
) -> PreviewRow {
    let evaluated = evaluate_row_scoped(
        &values,
        batch_currency,
        reference,
        observed_at,
        model_catalog,
        specs_catalog,
    );
    PreviewRow {
        row,
        status: if evaluated.errors.is_empty() {
            RowStatus::Valid
        } else {
            RowStatus::Error
        },
        values,
        normalized: evaluated.normalized,
        specs: evaluated.specs,
        errors: evaluated.errors,
        duplicate_group_id: None,
        provenance: evaluated.provenance,
        match_info: evaluated.match_info,
    }
}

pub fn csv_checksum(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

pub fn reject_oversize(len: usize) -> Result<(), ApiError> {
    if len > MAX_CSV_BYTES {
        return Err(ApiError::bad_request(
            "file_too_large",
            format!(
                "CSV files must be 10 MiB or smaller; this file is {} bytes.",
                len
            ),
        )
        .with_field("file"));
    }
    Ok(())
}

#[cfg(test)]
mod tests;

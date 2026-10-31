//! Deterministic, budget- and stock-constrained build recommendation.
//!
//! The search only ever selects parts from one immutable inventory snapshot.
//! Every candidate has passed the compatibility engine and is a complete build;
//! a build the checker would reject is never presented. Results are ordered by
//! published weights, then total price, then SKU list, so an identical request
//! against an unchanged snapshot and rule set yields the same order, parts and
//! totals. Only identifiers and timestamps differ because each report is a new
//! persisted record.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::compatibility::{CheckReport, CheckSummary, CompletenessStatus, Verdict};
use crate::domain::{now_rfc3339, parse_currency, Snapshot};
use crate::error::ApiError;

mod candidate;
mod explain;
mod export;
mod score;
mod search;

pub use export::{export_json, export_markdown};
use search::{build_input, search_builds, SearchInput};

pub const SCORING_VERSION: &str = "recommend-score-v1";
pub const EXPORT_VERSION: &str = "recommend-export-v1";
/// Complete combinations stop after this many expanded search nodes. The
/// limit is stated in every report so a shortened search is never presented as
/// a global optimum.
pub const SEARCH_WORK_LIMIT: u64 = 100_000;
pub const MAX_CANDIDATES: usize = 3;
const USE_CASES: &[&str] = &["gaming", "office", "creation"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecommendRequest {
    #[serde(default)]
    pub snapshot_id: String,
    #[serde(default)]
    pub budget_minor: Option<i64>,
    #[serde(default)]
    pub currency: Option<String>,
    #[serde(default)]
    pub use_case: Option<String>,
    #[serde(default)]
    pub inventory_only: Option<bool>,
    #[serde(default)]
    pub locked_skus: Vec<String>,
    #[serde(default)]
    pub excluded_skus: Vec<String>,
}

/// Validated request, kept in the report so an export can explain itself.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecommendationRequestEcho {
    pub budget_minor: i64,
    pub currency: String,
    pub use_case: String,
    pub inventory_only: bool,
    pub locked_skus: Vec<String>,
    pub excluded_skus: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct Settings {
    pub budget_minor: i64,
    pub currency: String,
    pub use_case: String,
    pub locked_skus: Vec<String>,
    pub excluded_skus: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CandidatePart {
    pub record_id: String,
    pub sku: String,
    pub category: String,
    pub model: String,
    pub quantity: i64,
    /// An owned part is included at zero incremental price and takes no stock.
    pub owned: bool,
    pub unit_price_minor: i64,
    pub line_total_minor: i64,
    pub available: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StockLine {
    pub sku: String,
    pub category: String,
    pub used: i64,
    pub available: i64,
    pub remaining: i64,
    pub owned: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ScoreDimension {
    pub key: String,
    pub label: String,
    pub available: bool,
    /// `None` when the evidence is missing; never a silent zero.
    pub value: Option<f64>,
    pub weight: f64,
    pub basis: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ScoreBreakdown {
    pub total: f64,
    /// `full` when every dimension has sourced evidence, `partial` otherwise.
    pub confidence: String,
    pub dimensions: Vec<ScoreDimension>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AlternativePart {
    pub category: String,
    pub sku: String,
    pub model: String,
    pub line_total_minor: i64,
    /// `higher_cost` or `different_build`.
    pub reason: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub rank: usize,
    /// `best_overall`, `lowest_cost`, `highest_performance` or `alternative`.
    pub objective: String,
    pub label: String,
    pub parts: Vec<CandidatePart>,
    pub total_minor: i64,
    pub budget_delta_minor: i64,
    pub stock: Vec<StockLine>,
    pub units_from_stock: i64,
    pub owned_units: i64,
    pub overall_status: Verdict,
    pub completeness: CompletenessStatus,
    pub check_report_id: String,
    pub check_summary: CheckSummary,
    pub unknown_follow_ups: Vec<String>,
    pub scores: ScoreBreakdown,
    pub reasons: Vec<String>,
    pub alternatives: Vec<AlternativePart>,
    /// The full persisted report is embedded so the comparison view, export and
    /// a later re-read all cite the same rule evidence.
    pub check: CheckReport,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Exclusion {
    pub sku: String,
    pub category: String,
    pub model: String,
    /// `requested_excluded`, `out_of_stock`, `over_budget`, `currency_mismatch`
    /// or `ambiguous_sku`.
    pub reason: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Blocker {
    pub code: String,
    pub category: Option<String>,
    pub sku: Option<String>,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NoSolution {
    /// `budget`, `stock`, `compatibility` or `search_limit`.
    pub binding: String,
    pub headline: String,
    pub explanation: String,
    pub blockers: Vec<Blocker>,
    pub adjustable: Vec<String>,
    pub minimum_build_minor: Option<i64>,
    pub budget_shortfall_minor: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SearchStats {
    pub limit: u64,
    pub nodes_examined: u64,
    pub leaves_evaluated: u64,
    pub valid_builds: u64,
    pub limited: bool,
    pub method: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RecommendStatus {
    Ok,
    NoSolution,
    SearchLimited,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecommendationReport {
    pub id: String,
    pub created_at: String,
    pub snapshot_id: String,
    pub snapshot_revision: i64,
    pub rule_set_version: String,
    pub scoring_version: String,
    pub status: RecommendStatus,
    pub request: RecommendationRequestEcho,
    pub currency: String,
    pub budget_minor: i64,
    pub candidates: Vec<Candidate>,
    pub no_solution: Option<NoSolution>,
    pub search: SearchStats,
    pub exclusions: Vec<Exclusion>,
    #[serde(default)]
    pub stale: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_snapshot_revision: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stale_reason: Option<String>,
}

pub(crate) struct SearchOutcome {
    pub candidates: Vec<Candidate>,
    pub no_solution: Option<NoSolution>,
    pub search: SearchStats,
    pub exclusions: Vec<Exclusion>,
}

fn field_error(code: &str, message: impl Into<String>, field: &str) -> ApiError {
    ApiError::bad_request(code, message).with_field(field)
}

fn normalise_sku_list(
    list: &[String],
    field: &str,
    error_code: &str,
) -> Result<Vec<String>, ApiError> {
    let mut seen = Vec::new();
    for raw in list {
        let sku = raw.trim();
        if sku.is_empty() {
            return Err(field_error(
                error_code,
                "Every listed SKU must be a non-empty value.",
                field,
            ));
        }
        if seen.iter().any(|existing| existing == sku) {
            return Err(field_error(
                "duplicate_sku",
                format!("SKU {sku} is listed more than once."),
                field,
            )
            .with_details(serde_json::json!({ "sku": sku })));
        }
        seen.push(sku.to_string());
    }
    Ok(seen)
}

fn validate(snapshot: &Snapshot, request: &RecommendRequest) -> Result<Settings, ApiError> {
    if request.snapshot_id.trim().is_empty() {
        return Err(field_error(
            "snapshot_required",
            "Choose an inventory snapshot first.",
            "snapshotId",
        ));
    }
    let Some(budget) = request.budget_minor else {
        return Err(field_error(
            "budget_required",
            "Enter a total budget in minor units.",
            "budgetMinor",
        ));
    };
    if budget <= 0 {
        return Err(field_error(
            "budget_invalid",
            "The budget must be greater than zero.",
            "budgetMinor",
        ));
    }
    let Some(raw_currency) = request.currency.as_deref() else {
        return Err(field_error(
            "currency_required",
            "Choose the currency for this budget.",
            "currency",
        ));
    };
    let currency = parse_currency(raw_currency)
        .map_err(|error| field_error(&error.code, error.message, "currency"))?;
    if currency != snapshot.currency {
        return Err(field_error(
            "currency_mismatch",
            format!(
                "The snapshot uses {}; the request uses {currency}. One currency per inventory batch.",
                snapshot.currency
            ),
            "currency",
        )
        .with_details(serde_json::json!({ "snapshotCurrency": snapshot.currency, "requestCurrency": currency })));
    }
    let Some(raw_use_case) = request.use_case.as_deref() else {
        return Err(field_error(
            "use_case_required",
            "Choose what the build is for.",
            "useCase",
        ));
    };
    let use_case = raw_use_case.trim().to_lowercase();
    if !USE_CASES.contains(&use_case.as_str()) {
        return Err(field_error(
            "use_case_unsupported",
            format!(
                "`{}` is not a supported use case. Use gaming, office or creation.",
                raw_use_case.trim()
            ),
            "useCase",
        ));
    }
    if request.inventory_only == Some(false) {
        return Err(field_error(
            "catalog_unavailable",
            "Only parts in your inventory can be recommended. No licensed hardware catalog is configured.",
            "inventoryOnly",
        ));
    }
    let locked_skus = normalise_sku_list(&request.locked_skus, "lockedSkus", "locked_sku_invalid")?;
    let excluded_skus = normalise_sku_list(
        &request.excluded_skus,
        "excludedSkus",
        "excluded_sku_invalid",
    )?;
    if let Some(sku) = locked_skus.iter().find(|sku| excluded_skus.contains(sku)) {
        return Err(field_error(
            "locked_excluded_conflict",
            format!("SKU {sku} is both locked and excluded."),
            "lockedSkus",
        )
        .with_details(serde_json::json!({ "sku": sku })));
    }
    for sku in &locked_skus {
        if !snapshot.records.iter().any(|record| &record.sku == sku) {
            return Err(ApiError::unprocessable(
                "locked_sku_not_found",
                format!("Locked SKU {sku} is not present in this snapshot."),
            )
            .with_field("lockedSkus")
            .with_details(serde_json::json!({ "sku": sku })));
        }
    }
    for sku in &excluded_skus {
        if !snapshot.records.iter().any(|record| &record.sku == sku) {
            return Err(ApiError::unprocessable(
                "excluded_sku_not_found",
                format!("Excluded SKU {sku} is not present in this snapshot."),
            )
            .with_field("excludedSkus")
            .with_details(serde_json::json!({ "sku": sku })));
        }
    }
    let mut locked_categories: BTreeMap<String, String> = BTreeMap::new();
    for sku in &locked_skus {
        let record = snapshot
            .records
            .iter()
            .find(|record| &record.sku == sku)
            .expect("locked SKUs were checked above");
        if let Some(existing) = locked_categories.insert(record.category.clone(), sku.clone()) {
            return Err(field_error(
                "locked_category_conflict",
                format!(
                    "A build can contain only one {}; both {existing} and {sku} are locked.",
                    record.category
                ),
                "lockedSkus",
            )
            .with_details(
                serde_json::json!({ "category": record.category, "skus": [existing, sku] }),
            ));
        }
    }
    Ok(Settings {
        budget_minor: budget,
        currency,
        use_case,
        locked_skus,
        excluded_skus,
    })
}

/// Run the search and assemble a persisted-shape report. The caller persists it
/// and its candidate check reports; nothing here mutates the snapshot or stock.
pub fn recommend(
    snapshot: &Snapshot,
    request: &RecommendRequest,
) -> Result<RecommendationReport, ApiError> {
    let settings = validate(snapshot, request)?;
    let SearchInput { pools, exclusions } = build_input(snapshot, &settings)?;
    let outcome = search_builds(snapshot, &settings, &pools, exclusions);
    let status = if outcome.search.limited {
        RecommendStatus::SearchLimited
    } else if outcome.candidates.is_empty() {
        RecommendStatus::NoSolution
    } else {
        RecommendStatus::Ok
    };
    Ok(RecommendationReport {
        id: Uuid::new_v4().to_string(),
        created_at: now_rfc3339(),
        snapshot_id: snapshot.id.clone(),
        snapshot_revision: snapshot.revision,
        rule_set_version: crate::compatibility::RULE_SET_VERSION.to_string(),
        scoring_version: SCORING_VERSION.to_string(),
        status,
        request: RecommendationRequestEcho {
            budget_minor: settings.budget_minor,
            currency: settings.currency.clone(),
            use_case: settings.use_case.clone(),
            inventory_only: true,
            locked_skus: settings.locked_skus.clone(),
            excluded_skus: settings.excluded_skus.clone(),
        },
        currency: snapshot.currency.clone(),
        budget_minor: settings.budget_minor,
        candidates: outcome.candidates,
        no_solution: outcome.no_solution,
        search: outcome.search,
        exclusions: outcome.exclusions,
        stale: false,
        current_snapshot_revision: Some(snapshot.revision),
        stale_reason: None,
    })
}

pub(crate) fn format_money(minor: i64, currency: &str) -> String {
    let negative = minor < 0;
    let value = minor.unsigned_abs();
    let whole = value / 100;
    let cents = value % 100;
    let digits = whole.to_string();
    let mut grouped = String::new();
    for (index, ch) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    if negative {
        grouped.insert(0, '-');
    }
    format!("{grouped}.{cents:02} {currency}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{MatchInfo, Provenance, Record, SnapshotSource};

    fn record(sku: &str, category: &str, price_minor: i64, quantity: i64, specs: &str) -> Record {
        let user = Provenance::user("csv:test.csv", "2026-10-31T00:00:00Z");
        Record {
            id: format!("id-{sku}"),
            lot: 1,
            sku: sku.to_string(),
            category: category.to_string(),
            model_raw: format!("Fixture {sku}"),
            model_normalized: format!("Fixture {sku}"),
            quantity,
            price_minor,
            currency: "USD".to_string(),
            specs: crate::domain::parse_specs(specs, &user).unwrap(),
            provenance: BTreeMap::from([("model".to_string(), user)]),
            match_info: MatchInfo::default(),
        }
    }

    fn snapshot() -> Snapshot {
        let records = vec![record("CPU", "cpu", 20000, 1, "socket=AM5")];
        Snapshot {
            id: "snapshot-1".to_string(),
            created_at: "2026-10-31T00:00:00Z".to_string(),
            revision: 1,
            parent_snapshot_id: None,
            currency: "USD".to_string(),
            source: SnapshotSource {
                filename: "test.csv".to_string(),
                sha256: "abc".to_string(),
            },
            note: None,
            totals: Snapshot::totals_for(&records).unwrap(),
            records,
        }
    }

    fn request() -> RecommendRequest {
        RecommendRequest {
            snapshot_id: "snapshot-1".to_string(),
            budget_minor: Some(100_000),
            currency: Some("USD".to_string()),
            use_case: Some("gaming".to_string()),
            inventory_only: Some(true),
            locked_skus: Vec::new(),
            excluded_skus: Vec::new(),
        }
    }

    #[test]
    fn money_is_grouped_without_a_locale() {
        assert_eq!(format_money(0, "USD"), "0.00 USD");
        assert_eq!(format_money(129_950, "USD"), "1,299.50 USD");
        assert_eq!(format_money(1_234_567, "EUR"), "12,345.67 EUR");
        assert_eq!(format_money(-500, "USD"), "-5.00 USD");
    }

    #[test]
    fn budget_and_currency_are_validated_with_field_codes() {
        let snapshot = snapshot();
        let mut missing_budget = request();
        missing_budget.budget_minor = None;
        let error = recommend(&snapshot, &missing_budget).unwrap_err();
        assert_eq!(error.code, "budget_required");
        assert_eq!(error.field.as_deref(), Some("budgetMinor"));

        let mut zero = request();
        zero.budget_minor = Some(0);
        assert_eq!(
            recommend(&snapshot, &zero).unwrap_err().code,
            "budget_invalid"
        );

        let mut wrong_currency = request();
        wrong_currency.currency = Some("EUR".to_string());
        let error = recommend(&snapshot, &wrong_currency).unwrap_err();
        assert_eq!(error.code, "currency_mismatch");

        let mut bad_case = request();
        bad_case.use_case = Some("mining".to_string());
        assert_eq!(
            recommend(&snapshot, &bad_case).unwrap_err().code,
            "use_case_unsupported"
        );

        let mut catalog = request();
        catalog.inventory_only = Some(false);
        assert_eq!(
            recommend(&snapshot, &catalog).unwrap_err().code,
            "catalog_unavailable"
        );
    }

    #[test]
    fn locked_and_excluded_lists_are_checked_against_the_snapshot() {
        let snapshot = snapshot();
        let mut both = request();
        both.locked_skus = vec!["CPU".to_string()];
        both.excluded_skus = vec!["CPU".to_string()];
        assert_eq!(
            recommend(&snapshot, &both).unwrap_err().code,
            "locked_excluded_conflict"
        );

        let mut missing = request();
        missing.locked_skus = vec!["NOPE".to_string()];
        let error = recommend(&snapshot, &missing).unwrap_err();
        assert_eq!(error.code, "locked_sku_not_found");
        assert_eq!(error.status, axum::http::StatusCode::UNPROCESSABLE_ENTITY);

        let mut duplicate = request();
        duplicate.excluded_skus = vec!["CPU".to_string(), "CPU".to_string()];
        assert_eq!(
            recommend(&snapshot, &duplicate).unwrap_err().code,
            "duplicate_sku"
        );
    }
}

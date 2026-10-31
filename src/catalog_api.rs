//! HTTP contract for operator-supplied catalog ingestion, registry, search and
//! import matching.

use std::collections::{BTreeMap, BTreeSet};

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::Value;

use crate::api::{storage_failure, AppState};
use crate::catalog::{
    match_rows, part_provenance_map, rank_parts, validate_catalog, CatalogApplication,
    CatalogDataset, CatalogReport, MatchResult, MatchRow, StoredCatalogPart, DEFAULT_SEARCH_LIMIT,
    MAX_MATCH_ROWS, MAX_SEARCH_LIMIT,
};
use crate::commit::CommitRequest;
use crate::domain::{
    normalize_category, now_rfc3339, parse_specs, supported_categories, Provenance,
};
use crate::error::{ApiError, ApiJson};
use crate::store::StoreError;

fn category_error(raw: &str) -> ApiError {
    ApiError::bad_request(
        "category_unsupported",
        format!("`{}` is not a supported category.", raw.trim()),
    )
    .with_field("category")
    .with_details(serde_json::json!({ "accepted": supported_categories() }))
}

fn resolve_category(raw: Option<&String>) -> Result<Option<String>, ApiError> {
    match raw {
        None => Ok(None),
        Some(value) if value.trim().is_empty() => Ok(None),
        Some(value) => normalize_category(value)
            .map(|category| Some(category.to_string()))
            .ok_or_else(|| category_error(value)),
    }
}

pub(crate) async fn upload_catalog(
    State(state): State<AppState>,
    ApiJson(body): ApiJson<Value>,
) -> Result<Json<CatalogReport>, ApiError> {
    if body.get("acknowledged").and_then(Value::as_bool) != Some(true) {
        return Err(ApiError::bad_request(
            "catalog_acknowledgement_required",
            "Confirm that you have the right to use this catalog data before importing it.",
        )
        .with_field("acknowledged"));
    }
    let Some(document) = body.get("catalog") else {
        return Err(ApiError::bad_request(
            "catalog_required",
            "The request must carry a `catalog` JSON object.",
        )
        .with_field("catalog"));
    };
    let catalog = validate_catalog(document)?;
    let outcome = state
        .store
        .import_catalog(&catalog, &now_rfc3339())
        .map_err(storage_failure)?;
    Ok(Json(CatalogReport {
        accepted: true,
        replaced: outcome.replaced,
        part_count: outcome.dataset.row_count,
        category_counts: outcome.dataset.category_counts.clone(),
        dataset: outcome.dataset,
        spec_keys: crate::domain::supported_spec_keys()
            .into_iter()
            .map(str::to_string)
            .collect(),
    }))
}

pub(crate) async fn list_datasets(
    State(state): State<AppState>,
) -> Result<Json<Vec<CatalogDataset>>, ApiError> {
    state
        .store
        .list_catalog_datasets()
        .map(Json)
        .map_err(storage_failure)
}

#[derive(Debug, Deserialize)]
pub(crate) struct ActiveRequest {
    pub active: bool,
}

pub(crate) async fn patch_dataset(
    State(state): State<AppState>,
    Path(dataset_id): Path<String>,
    ApiJson(request): ApiJson<ActiveRequest>,
) -> Result<Json<CatalogDataset>, ApiError> {
    state
        .store
        .set_catalog_dataset_active(&dataset_id, request.active)
        .map(Json)
        .map_err(|error| match error {
            StoreError::NotFound => ApiError::not_found(
                "catalog_dataset_not_found",
                "No catalog dataset has this ID.",
            ),
            other => storage_failure(other),
        })
}

#[derive(Debug, Deserialize)]
pub(crate) struct ConfirmQuery {
    pub confirm: Option<String>,
}

pub(crate) async fn delete_dataset(
    State(state): State<AppState>,
    Path(dataset_id): Path<String>,
    Query(query): Query<ConfirmQuery>,
) -> Result<Json<Value>, ApiError> {
    if query.confirm.as_deref() != Some("remove") {
        return Err(ApiError::bad_request(
            "catalog_confirm_required",
            "Removing a dataset needs `confirm=remove`. Inventory and snapshots are never affected.",
        )
        .with_field("confirm"));
    }
    state
        .store
        .delete_catalog_dataset(&dataset_id)
        .map_err(|error| match error {
            StoreError::NotFound => ApiError::not_found(
                "catalog_dataset_not_found",
                "No catalog dataset has this ID.",
            ),
            other => storage_failure(other),
        })?;
    Ok(Json(
        serde_json::json!({ "removed": true, "datasetId": dataset_id }),
    ))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SearchQuery {
    pub q: Option<String>,
    pub category: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

pub(crate) async fn search_catalog(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<Value>, ApiError> {
    let Some(raw_query) = query.q.as_deref() else {
        return Err(ApiError::bad_request(
            "catalog_query_required",
            "Enter a SKU, model or word to search the catalog.",
        )
        .with_field("q"));
    };
    let text = raw_query.trim();
    if text.is_empty() || text.chars().count() > 200 {
        return Err(ApiError::bad_request(
            "catalog_query_required",
            "The search needs 1–200 characters.",
        )
        .with_field("q"));
    }
    let limit = query.limit.unwrap_or(DEFAULT_SEARCH_LIMIT);
    if limit == 0 || limit > MAX_SEARCH_LIMIT {
        return Err(ApiError::bad_request(
            "catalog_limit_invalid",
            format!("`limit` must be between 1 and {MAX_SEARCH_LIMIT}."),
        )
        .with_field("limit"));
    }
    let offset = query.offset.unwrap_or(0);
    let category = resolve_category(query.category.as_ref())?;
    let parts = state
        .store
        .active_catalog_parts(category.as_deref())
        .map_err(storage_failure)?;
    let ranked = rank_parts(&parts, text);
    let total = ranked.len();
    let hits = ranked
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect::<Vec<&StoredCatalogPart>>();
    Ok(Json(serde_json::json!({
        "query": text,
        "category": category,
        "limit": limit,
        "offset": offset,
        "total": total,
        "hits": hits,
    })))
}

pub(crate) async fn get_part(
    State(state): State<AppState>,
    Path(part_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let stored = state
        .store
        .get_catalog_part(&part_id)
        .map_err(|error| match error {
            StoreError::NotFound => ApiError::not_found(
                "catalog_part_not_found",
                "No stored catalog part has this ID.",
            ),
            other => storage_failure(other),
        })?;
    Ok(Json(serde_json::json!({
        "part": stored.part,
        "dataset": stored.dataset,
        "provenance": part_provenance_map(&stored.part, &stored.dataset),
    })))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MatchRequest {
    pub rows: Vec<MatchRow>,
}

pub(crate) async fn match_import_rows(
    State(state): State<AppState>,
    ApiJson(request): ApiJson<MatchRequest>,
) -> Result<Json<Value>, ApiError> {
    if request.rows.is_empty() || request.rows.len() > MAX_MATCH_ROWS {
        return Err(ApiError::bad_request(
            "catalog_rows_invalid",
            format!("Send between 1 and {MAX_MATCH_ROWS} rows."),
        )
        .with_field("rows"));
    }
    let mut problems = Vec::new();
    let mut rows = Vec::with_capacity(request.rows.len());
    for row in request.rows {
        let Some(category) = normalize_category(&row.category) else {
            problems.push(serde_json::json!({
                "row": row.row,
                "field": "category",
                "code": "category_unsupported",
                "message": format!("`{}` is not a supported category.", row.category.trim()),
                "suggestion": format!("Use one of: {}.", supported_categories()),
            }));
            continue;
        };
        if !row.specs.trim().is_empty() {
            if let Err(error) = parse_specs(
                &row.specs,
                &Provenance::user("preview", "1970-01-01T00:00:00Z"),
            ) {
                problems.push(serde_json::json!({
                    "row": row.row,
                    "field": "specs",
                    "code": error.code,
                    "message": error.message,
                }));
                continue;
            }
        }
        rows.push(MatchRow {
            category: category.to_string(),
            ..row
        });
    }
    if !problems.is_empty() {
        return Err(ApiError::unprocessable(
            "catalog_match_invalid",
            "A row sent for catalog matching is invalid; nothing was matched.",
        )
        .with_details(serde_json::json!({ "rows": problems })));
    }
    let parts = state
        .store
        .active_catalog_parts(None)
        .map_err(storage_failure)?;
    let matches: Vec<MatchResult> = match_rows(&rows, &parts);
    Ok(Json(serde_json::json!({ "matches": matches })))
}

/// Load the parts named by per-row commit opt-ins. A missing part refuses the
/// commit before anything is written.
pub(crate) fn catalog_applications(
    state: &AppState,
    request: &CommitRequest,
) -> Result<BTreeMap<String, CatalogApplication>, ApiError> {
    let ids = request
        .decisions
        .iter()
        .filter_map(|decision| {
            decision
                .catalog
                .as_ref()
                .map(|catalog| catalog.part_id.clone())
        })
        .collect::<BTreeSet<_>>();
    if ids.is_empty() {
        return Ok(BTreeMap::new());
    }
    let mut applications = BTreeMap::new();
    for id in ids {
        let part =
            state.store.get_catalog_part(&id).map_err(|error| {
                match error {
            StoreError::NotFound => ApiError::unprocessable(
                "catalog_part_not_found",
                "A commit opted into a catalog part that no longer exists. Match the rows again.",
            )
            .with_field("catalog")
            .with_details(serde_json::json!({ "partId": id })),
            other => storage_failure(other),
        }
            })?;
        applications.insert(part.part.id.clone(), CatalogApplication::from_stored(&part));
    }
    Ok(applications)
}

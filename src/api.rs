//! HTTP contract for inventory preview, commit, snapshots, and corrections.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::{DefaultBodyLimit, Multipart, Path as AxumPath, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{any, get, post};
use axum::{Json, Router};
use time::OffsetDateTime;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use crate::commit::{
    build_correction, build_records, decision_hash, snapshot_from_preview, CommitRequest,
    CommitResponse, CorrectionRequest,
};
use crate::csv_import::{preview_csv, SAMPLE_CSV};
use crate::domain::{preview_is_expired, Preview, Snapshot, SnapshotSummary};
use crate::error::ApiError;
use crate::store::{CommitOutcome, Store, StoreError};

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<Store>,
    pub web_dir: PathBuf,
}

impl AppState {
    pub fn new(store: Arc<Store>, web_dir: PathBuf) -> Self {
        Self { store, web_dir }
    }
}

fn storage_failure(error: StoreError) -> ApiError {
    tracing::error!(error = %error, "inventory storage operation failed");
    ApiError::internal("The inventory store could not complete this request.")
}

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok", "service": "rigcraft-inventory" }))
}

async fn sample_csv() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"rigcraft-inventory-sample.csv\"".to_string(),
            ),
        ],
        SAMPLE_CSV,
    )
}

async fn preview_import(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<Preview>, ApiError> {
    let mut file: Option<(String, Vec<u8>)> = None;
    let mut currency: Option<String> = None;
    let mut mapping: BTreeMap<String, String> = BTreeMap::new();
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|error| ApiError::bad_request("multipart_error", error.to_string()))?
    {
        let name = field.name().unwrap_or_default().to_string();
        match name.as_str() {
            "file" => {
                if file.is_some() {
                    return Err(ApiError::bad_request(
                        "duplicate_field",
                        "Send exactly one CSV file.",
                    )
                    .with_field("file"));
                }
                let filename = field
                    .file_name()
                    .map(str::to_string)
                    .unwrap_or_else(|| "inventory.csv".to_string());
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|error| ApiError::bad_request("file_unreadable", error.to_string()))?;
                file = Some((filename, bytes.to_vec()));
            }
            "currency" => {
                currency = Some(field.text().await.map_err(|error| {
                    ApiError::bad_request("multipart_error", error.to_string())
                })?);
            }
            "mapping" => {
                let text = field
                    .text()
                    .await
                    .map_err(|error| ApiError::bad_request("multipart_error", error.to_string()))?;
                if !text.trim().is_empty() {
                    mapping = serde_json::from_str(&text).map_err(|_| {
                        ApiError::bad_request(
                            "mapping_invalid",
                            "Column mapping must be a JSON object of logical field to column name.",
                        )
                        .with_field("mapping")
                    })?;
                }
            }
            _ => {
                // Consume unknown parts so the multipart reader stays aligned.
                let _ = field.bytes().await;
            }
        }
    }
    let Some((filename, bytes)) = file else {
        return Err(
            ApiError::bad_request("file_required", "Choose a UTF-8 CSV file to import.")
                .with_field("file"),
        );
    };
    let Some(currency) = currency else {
        return Err(ApiError::bad_request(
            "currency_required",
            "Choose the single currency for this import batch.",
        )
        .with_field("currency"));
    };
    let parsed = preview_csv(
        &bytes,
        &filename,
        &currency,
        &mapping,
        Uuid::new_v4().to_string(),
    )?;
    state
        .store
        .insert_preview(&parsed.preview, &parsed.sha256)
        .map_err(storage_failure)?;
    Ok(Json(parsed.preview))
}

async fn commit_import(
    State(state): State<AppState>,
    AxumPath(preview_id): AxumPath<String>,
    Json(request): Json<CommitRequest>,
) -> Result<Json<CommitResponse>, ApiError> {
    let stored = state
        .store
        .get_preview(&preview_id)
        .map_err(|error| match error {
            StoreError::NotFound => ApiError::not_found(
                "preview_not_found",
                "This import preview no longer exists. Upload the file again.",
            ),
            other => storage_failure(other),
        })?;
    let hash = decision_hash(&request);
    if let Some(previous_hash) = &stored.commit_hash {
        if previous_hash == &hash {
            if let Some(snapshot_id) = &stored.snapshot_id {
                let snapshot = state
                    .store
                    .get_snapshot(snapshot_id)
                    .map_err(storage_failure)?;
                return Ok(Json(CommitResponse {
                    snapshot_id: snapshot_id.clone(),
                    snapshot,
                }));
            }
        }
        return Err(ApiError::conflict(
            "preview_already_committed",
            "This preview was already committed with a different decision set.",
        ));
    }
    if preview_is_expired(&stored.created_at, OffsetDateTime::now_utc()) {
        return Err(ApiError::gone(
            "preview_expired",
            "This import preview is older than 24 hours. Upload the file again.",
        ));
    }
    let records = build_records(&stored.preview, &request)?;
    let candidate = snapshot_from_preview(&stored.preview, records, &stored.sha256);
    match state.store.commit_preview(&preview_id, &hash, &candidate) {
        Ok(CommitOutcome::Created) => Ok(Json(CommitResponse {
            snapshot_id: candidate.id.clone(),
            snapshot: candidate,
        })),
        Ok(CommitOutcome::Existing(snapshot_id)) => {
            let snapshot = state
                .store
                .get_snapshot(&snapshot_id)
                .map_err(storage_failure)?;
            Ok(Json(CommitResponse {
                snapshot_id,
                snapshot,
            }))
        }
        Err(StoreError::Conflict) => Err(ApiError::conflict(
            "preview_already_committed",
            "This preview was already committed with a different decision set.",
        )),
        Err(error) => Err(storage_failure(error)),
    }
}

async fn list_snapshots(
    State(state): State<AppState>,
) -> Result<Json<Vec<SnapshotSummary>>, ApiError> {
    state
        .store
        .list_snapshots()
        .map(Json)
        .map_err(storage_failure)
}

async fn get_snapshot(
    State(state): State<AppState>,
    AxumPath(snapshot_id): AxumPath<String>,
) -> Result<Json<Snapshot>, ApiError> {
    state
        .store
        .get_snapshot(&snapshot_id)
        .map(Json)
        .map_err(|error| match error {
            StoreError::NotFound => {
                ApiError::not_found("snapshot_not_found", "No inventory snapshot has this ID.")
            }
            other => storage_failure(other),
        })
}

async fn correct_snapshot(
    State(state): State<AppState>,
    AxumPath(snapshot_id): AxumPath<String>,
    Json(request): Json<CorrectionRequest>,
) -> Result<Json<Snapshot>, ApiError> {
    let snapshot = state
        .store
        .get_snapshot(&snapshot_id)
        .map_err(|error| match error {
            StoreError::NotFound => {
                ApiError::not_found("snapshot_not_found", "No inventory snapshot has this ID.")
            }
            other => storage_failure(other),
        })?;
    let corrected = build_correction(&snapshot, &request)?;
    state
        .store
        .insert_correction(&corrected)
        .map_err(storage_failure)?;
    Ok(Json(corrected))
}

async fn api_not_found() -> Response {
    ApiError::not_found("route_not_found", "This API route does not exist.").into_response()
}

async fn web_not_built() -> Response {
    (
        StatusCode::NOT_FOUND,
        "The Web build is missing. Run `npm run build` before opening the UI.",
    )
        .into_response()
}

pub fn build_router(state: AppState) -> Router {
    let web_dir = state.web_dir.clone();
    let router = Router::new()
        .route("/api/health", get(health))
        .route("/api/imports/sample", get(sample_csv))
        .route("/api/imports", post(preview_import))
        .route("/api/imports/{preview_id}/commit", post(commit_import))
        .route("/api/inventory/snapshots", get(list_snapshots))
        .route("/api/inventory/snapshots/{snapshot_id}", get(get_snapshot))
        .route(
            "/api/inventory/snapshots/{snapshot_id}/corrections",
            post(correct_snapshot),
        )
        .route("/api/{*rest}", any(api_not_found))
        .layer(DefaultBodyLimit::max(11 * 1024 * 1024))
        .layer(TraceLayer::new_for_http())
        .with_state(state);
    let index = web_dir.join("index.html");
    if index.is_file() {
        router.fallback_service(ServeDir::new(web_dir).not_found_service(ServeFile::new(index)))
    } else {
        router.fallback(web_not_built)
    }
}

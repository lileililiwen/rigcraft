//! HTTP contract for inventory preview, commit, snapshots, and corrections.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::{DefaultBodyLimit, Multipart, Path as AxumPath, Query, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{any, get, post};
use axum::{Json, Router};
use time::OffsetDateTime;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use crate::ai::{
    build_prompt, field_changes, interpret, provider, require_key, revised_request,
    validate_provider_url, validate_text, AiProposalResponse, BuiltPrompt, ParseRequest,
    ProviderAdapter, ProviderIdentity, ReviseRequest,
};
use crate::commit::{
    build_correction, build_records, decision_hash, snapshot_from_preview, CommitRequest,
    CommitResponse, CorrectionRequest,
};
use crate::compatibility::{check_build, CheckReport, CheckRequest};
use crate::csv_import::{preview_csv, SAMPLE_CSV};
use crate::diagnosis::{
    diagnose, parse_list, DiagnoseRequest, DiagnosisReport, ParseListRequest, ParsedList,
};
use crate::domain::{preview_is_expired, Preview, Snapshot, SnapshotSummary};
use crate::error::ApiError;
use crate::recommend::{
    export_json, export_markdown, recommend, RecommendRequest, RecommendationReport,
};
use crate::store::{CommitOutcome, Store, StoreError};

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<Store>,
    pub web_dir: PathBuf,
    /// The single provider seam. `AppState::new` installs the real
    /// OpenAI-compatible adapter; tests and offline evidence replace it.
    pub provider: Arc<dyn ProviderAdapter>,
}

impl AppState {
    pub fn new(store: Arc<Store>, web_dir: PathBuf) -> Self {
        Self {
            store,
            web_dir,
            provider: Arc::new(provider::OpenAiCompatibleProvider),
        }
    }

    pub fn with_provider(mut self, provider: Arc<dyn ProviderAdapter>) -> Self {
        self.provider = provider;
        self
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

async fn check_compatibility(
    State(state): State<AppState>,
    Json(request): Json<CheckRequest>,
) -> Result<Json<CheckReport>, ApiError> {
    if request.snapshot_id.trim().is_empty() {
        return Err(ApiError::bad_request(
            "snapshot_required",
            "Choose an inventory snapshot first.",
        )
        .with_field("snapshotId"));
    }
    let snapshot = state
        .store
        .get_snapshot(&request.snapshot_id)
        .map_err(|error| match error {
            StoreError::NotFound => {
                ApiError::not_found("snapshot_not_found", "No inventory snapshot has this ID.")
            }
            other => storage_failure(other),
        })?;
    let report = check_build(&snapshot, &request)?;
    state
        .store
        .insert_check_report(&report)
        .map_err(storage_failure)?;
    Ok(Json(report))
}

async fn get_check_report(
    State(state): State<AppState>,
    AxumPath(check_id): AxumPath<String>,
) -> Result<Json<CheckReport>, ApiError> {
    let mut report = state
        .store
        .get_check_report(&check_id)
        .map_err(|error| match error {
            StoreError::NotFound => {
                ApiError::not_found("check_not_found", "No compatibility check has this ID.")
            }
            other => storage_failure(other),
        })?;
    match state
        .store
        .latest_chain_revision(&report.snapshot_id)
        .map_err(storage_failure)?
    {
        Some(revision) => {
            report.current_snapshot_revision = Some(revision);
            if revision != report.snapshot_revision {
                report.stale = true;
                report.stale_reason = Some(format!(
                    "The snapshot chain is now at revision {revision}; this report was checked against revision {}.",
                    report.snapshot_revision
                ));
            }
        }
        None => {
            report.stale = true;
            report.current_snapshot_revision = None;
            report.stale_reason =
                Some("The snapshot this report referenced no longer exists.".to_string());
        }
    }
    Ok(Json(report))
}

#[derive(Debug, serde::Deserialize)]
struct ExportQuery {
    format: Option<String>,
}

fn load_snapshot(state: &AppState, snapshot_id: &str) -> Result<Snapshot, ApiError> {
    state
        .store
        .get_snapshot(snapshot_id)
        .map_err(|error| match error {
            StoreError::NotFound => {
                ApiError::not_found("snapshot_not_found", "No inventory snapshot has this ID.")
            }
            other => storage_failure(other),
        })
}

fn with_staleness(
    mut report: RecommendationReport,
    state: &AppState,
) -> Result<RecommendationReport, ApiError> {
    match state
        .store
        .latest_chain_revision(&report.snapshot_id)
        .map_err(storage_failure)?
    {
        Some(revision) => {
            report.current_snapshot_revision = Some(revision);
            if revision != report.snapshot_revision {
                report.stale = true;
                report.stale_reason = Some(format!(
                    "The snapshot chain is now at revision {revision}; this report was recommended against revision {}.",
                    report.snapshot_revision
                ));
            }
        }
        None => {
            report.stale = true;
            report.current_snapshot_revision = None;
            report.stale_reason =
                Some("The snapshot this report referenced no longer exists.".to_string());
        }
    }
    Ok(report)
}

async fn recommend_build(
    State(state): State<AppState>,
    Json(request): Json<RecommendRequest>,
) -> Result<Json<RecommendationReport>, ApiError> {
    if request.snapshot_id.trim().is_empty() {
        return Err(ApiError::bad_request(
            "snapshot_required",
            "Choose an inventory snapshot first.",
        )
        .with_field("snapshotId"));
    }
    let snapshot = load_snapshot(&state, &request.snapshot_id)?;
    let report = recommend(&snapshot, &request)?;
    state
        .store
        .insert_recommendation(&report)
        .map_err(storage_failure)?;
    Ok(Json(report))
}

async fn get_recommendation_report(
    State(state): State<AppState>,
    AxumPath(recommendation_id): AxumPath<String>,
) -> Result<Json<RecommendationReport>, ApiError> {
    let report =
        state
            .store
            .get_recommendation(&recommendation_id)
            .map_err(|error| match error {
                StoreError::NotFound => ApiError::not_found(
                    "recommendation_not_found",
                    "No recommendation has this ID.",
                ),
                other => storage_failure(other),
            })?;
    with_staleness(report, &state).map(Json)
}

async fn export_recommendation(
    State(state): State<AppState>,
    AxumPath(recommendation_id): AxumPath<String>,
    Query(query): Query<ExportQuery>,
) -> Result<Response, ApiError> {
    let report =
        state
            .store
            .get_recommendation(&recommendation_id)
            .map_err(|error| match error {
                StoreError::NotFound => ApiError::not_found(
                    "recommendation_not_found",
                    "No recommendation has this ID.",
                ),
                other => storage_failure(other),
            })?;
    let report = with_staleness(report, &state)?;
    let key: String = report.id.chars().take(8).collect();
    let format = query
        .format
        .as_deref()
        .map(|value| value.trim().to_lowercase())
        .unwrap_or_else(|| "json".to_string());
    let (content_type, extension, body) = if matches!(format.as_str(), "markdown" | "md") {
        (
            "text/markdown; charset=utf-8",
            "md",
            export_markdown(&report),
        )
    } else if format == "json" {
        let snapshot = load_snapshot(&state, &report.snapshot_id)?;
        let value = export_json(&report, &snapshot);
        let body = serde_json::to_string_pretty(&value)
            .map_err(|_| ApiError::internal("The export could not be serialized."))?;
        ("application/json", "json", body)
    } else {
        return Err(ApiError::bad_request(
            "export_format_unsupported",
            "Use format=json or format=markdown.",
        )
        .with_field("format"));
    };
    let mut response = body.into_response();
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    if let Ok(disposition) = HeaderValue::from_str(&format!(
        "attachment; filename=\"rigcraft-recommendation-{key}.{extension}\""
    )) {
        response
            .headers_mut()
            .insert(header::CONTENT_DISPOSITION, disposition);
    }
    Ok(response)
}

async fn parse_diagnosis_list(
    State(state): State<AppState>,
    Json(request): Json<ParseListRequest>,
) -> Result<Json<ParsedList>, ApiError> {
    let snapshot = if request.snapshot_id.trim().is_empty() {
        None
    } else {
        Some(load_snapshot(&state, request.snapshot_id.trim())?)
    };
    Ok(Json(parse_list(snapshot.as_ref(), &request)?))
}

async fn diagnose_existing_build(
    State(state): State<AppState>,
    Json(request): Json<DiagnoseRequest>,
) -> Result<Json<DiagnosisReport>, ApiError> {
    if request.snapshot_id.trim().is_empty() {
        return Err(ApiError::bad_request(
            "snapshot_required",
            "Choose an inventory snapshot first.",
        )
        .with_field("snapshotId"));
    }
    let snapshot = load_snapshot(&state, request.snapshot_id.trim())?;
    let report = diagnose(&snapshot, &request)?;
    state
        .store
        .insert_diagnosis(&report)
        .map_err(storage_failure)?;
    Ok(Json(report))
}

async fn get_diagnosis_report(
    State(state): State<AppState>,
    AxumPath(diagnosis_id): AxumPath<String>,
) -> Result<Json<DiagnosisReport>, ApiError> {
    let mut report = state
        .store
        .get_diagnosis(&diagnosis_id)
        .map_err(|error| match error {
            StoreError::NotFound => {
                ApiError::not_found("diagnosis_not_found", "No build diagnosis has this ID.")
            }
            other => storage_failure(other),
        })?;
    match state
        .store
        .latest_chain_revision(&report.snapshot_id)
        .map_err(storage_failure)?
    {
        Some(revision) => {
            report.current_snapshot_revision = Some(revision);
            if revision != report.snapshot_revision {
                report.stale = true;
                report.stale_reason = Some(format!(
                    "The snapshot chain is now at revision {revision}; this diagnosis was made against revision {}.",
                    report.snapshot_revision
                ));
            }
        }
        None => {
            report.stale = true;
            report.current_snapshot_revision = None;
            report.stale_reason =
                Some("The snapshot this diagnosis referenced no longer exists.".to_string());
        }
    }
    Ok(Json(report))
}

struct Prepared {
    snapshot: Option<Snapshot>,
    snapshot_id: String,
    text: String,
    prompt: BuiltPrompt,
    completion: provider::CompletionRequest,
    identity: ProviderIdentity,
}

/// Validate the transport fields, load the snapshot and build the bounded
/// prompt. Nothing leaves the machine here.
fn prepare(state: &AppState, request: &ParseRequest) -> Result<Prepared, ApiError> {
    let text = validate_text(&request.text)?;
    let api_key = require_key(&request.api_key)?;
    let base_url = validate_provider_url(&request.base_url)?;
    let model = request.model.trim();
    if model.is_empty() {
        return Err(ApiError::bad_request(
            "ai_model_required",
            "Enter the provider model name, or use the deterministic form below.",
        )
        .with_field("model"));
    }
    let snapshot = if request.snapshot_id.trim().is_empty() {
        None
    } else {
        Some(load_snapshot(state, request.snapshot_id.trim())?)
    };
    let prompt = build_prompt(
        &text,
        snapshot.as_ref(),
        request.include_inventory_summary,
        model,
    )?;
    let endpoint = format!("{base_url}/chat/completions");
    let identity = ProviderIdentity {
        kind: state.provider.kind().to_string(),
        base_url,
        model: model.to_string(),
    };
    Ok(Prepared {
        snapshot_id: request.snapshot_id.trim().to_string(),
        snapshot,
        text,
        completion: provider::CompletionRequest {
            endpoint,
            model: model.to_string(),
            api_key,
            body: prompt.body.clone(),
        },
        prompt,
        identity,
    })
}

async fn call_provider(
    state: &AppState,
    completion: provider::CompletionRequest,
) -> Result<String, ApiError> {
    let adapter = state.provider.clone();
    let joined = tokio::task::spawn_blocking(move || adapter.complete(&completion))
        .await
        .map_err(|_| ApiError::internal("The provider call could not be scheduled."))?;
    joined.map_err(provider::ProviderFailure::into_api_error)
}

fn interpretation_response(
    prepared: &Prepared,
    interpretation: crate::ai::Interpretation,
    diff: Option<Vec<crate::ai::FieldChange>>,
    request: Option<RecommendRequest>,
) -> AiProposalResponse {
    AiProposalResponse {
        mode: "provider".to_string(),
        prompt_version: crate::ai::PROMPT_VERSION.to_string(),
        provider: prepared.identity.clone(),
        proposal: interpretation.proposal,
        ambiguities: interpretation.ambiguities,
        ignored_fields: interpretation.ignored_fields,
        sent: prepared.prompt.scope.clone(),
        confirm_required: true,
        request,
        diff,
    }
}

async fn ai_parse(
    State(state): State<AppState>,
    Json(request): Json<ParseRequest>,
) -> Result<Json<AiProposalResponse>, ApiError> {
    let prepared = prepare(&state, &request)?;
    let reply = call_provider(&state, prepared.completion.clone()).await?;
    let content =
        provider::extract_content(&reply).map_err(provider::ProviderFailure::into_api_error)?;
    let interpretation = interpret(
        &content,
        &prepared.text,
        prepared.snapshot.as_ref(),
        &prepared.snapshot_id,
        &prepared.prompt.included_skus,
    )?;
    // A parse never runs a search, mutates a snapshot or persists anything.
    tracing::info!(
        operation = "ai_parse",
        inventory_records = prepared.prompt.scope.inventory_records,
        "ai interpretation requested"
    );
    let proposed = interpretation.request.clone();
    Ok(Json(interpretation_response(
        &prepared,
        interpretation,
        None,
        proposed,
    )))
}

async fn ai_revise(
    State(state): State<AppState>,
    Json(request): Json<ReviseRequest>,
) -> Result<Json<AiProposalResponse>, ApiError> {
    let parse = request.as_parse();
    let prepared = prepare(&state, &parse)?;
    let reply = call_provider(&state, prepared.completion.clone()).await?;
    let content =
        provider::extract_content(&reply).map_err(provider::ProviderFailure::into_api_error)?;
    let interpretation = interpret(
        &content,
        &prepared.text,
        prepared.snapshot.as_ref(),
        &prepared.snapshot_id,
        &prepared.prompt.included_skus,
    )?;
    let diff = field_changes(&interpretation.proposal, &request.current);
    let applied = revised_request(
        &interpretation.proposal,
        &request.current,
        &prepared.snapshot_id,
    );
    tracing::info!(
        operation = "ai_revise",
        inventory_records = prepared.prompt.scope.inventory_records,
        changed = diff.len(),
        "ai interpretation requested"
    );
    Ok(Json(interpretation_response(
        &prepared,
        interpretation,
        Some(diff),
        applied,
    )))
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
        .route("/api/builds/check", post(check_compatibility))
        .route("/api/builds/checks/{check_id}", get(get_check_report))
        .route("/api/recommendations", post(recommend_build))
        .route(
            "/api/recommendations/{recommendation_id}",
            get(get_recommendation_report),
        )
        .route(
            "/api/recommendations/{recommendation_id}/export",
            get(export_recommendation),
        )
        .route("/api/diagnoses/parse", post(parse_diagnosis_list))
        .route("/api/diagnoses", post(diagnose_existing_build))
        .route("/api/diagnoses/{diagnosis_id}", get(get_diagnosis_report))
        .route("/api/ai/parse", post(ai_parse))
        .route("/api/ai/revise", post(ai_revise))
        .route("/api/{*rest}", any(api_not_found))
        .layer(DefaultBodyLimit::max(11 * 1024 * 1024))
        .layer(TraceLayer::new_for_http())
        .with_state(state);
    let index = web_dir.join("index.html");
    match std::fs::read_to_string(&index) {
        Ok(index_body) => {
            // A client-side route such as `/builds/check` is not a file on
            // disk. Serve the SPA shell with 200 rather than the 404 status a
            // bare file fallback would attach to an existing index file.
            let spa = move || {
                let index_body = index_body.clone();
                async move { Html(index_body) }
            };
            let spa_service = Router::new().fallback(spa);
            router.fallback_service(ServeDir::new(web_dir).fallback(spa_service))
        }
        Err(_) => router.fallback(web_not_built),
    }
}

//! Snapshot export, whole-database backup and guarded restore handlers.

use std::path::{Path, PathBuf};

use axum::extract::multipart::MultipartRejection;
use axum::extract::{Multipart, Path as AxumPath, Query, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use uuid::Uuid;

use crate::api::{load_snapshot, storage_failure, AppState, ExportQuery};
use crate::error::ApiError;
use crate::portability::{self, BackupInspection, BackupProblem, RestoreSummary, CSV_LOSSY_NOTE};
use crate::store::{RestoreFailure, SCHEMA_VERSION};

pub(crate) async fn export_snapshot(
    State(state): State<AppState>,
    AxumPath(snapshot_id): AxumPath<String>,
    Query(query): Query<ExportQuery>,
) -> Result<Response, ApiError> {
    let snapshot = load_snapshot(&state, &snapshot_id)?;
    let format = query
        .format
        .as_deref()
        .map(|value| value.trim().to_lowercase());
    let (content_type, extension, body, lossy) = match format.as_deref() {
        Some("csv") => (
            "text/csv; charset=utf-8",
            "csv",
            portability::snapshot_csv(&snapshot)
                .map_err(|_| ApiError::internal("The snapshot could not be exported as CSV."))?,
            true,
        ),
        Some("json") => (
            "application/json",
            "json",
            portability::snapshot_json(&snapshot)
                .map_err(|_| ApiError::internal("The snapshot could not be exported as JSON."))?,
            false,
        ),
        _ => {
            return Err(ApiError::bad_request(
                "export_format_unsupported",
                "Use format=csv or format=json.",
            )
            .with_field("format"))
        }
    };
    let filename = portability::export_filename(&snapshot, extension);
    let mut response = body.into_response();
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    if let Ok(disposition) = HeaderValue::from_str(&format!("attachment; filename=\"{filename}\""))
    {
        headers.insert(header::CONTENT_DISPOSITION, disposition);
    }
    if lossy {
        headers.insert("x-rigcraft-export-lossy", HeaderValue::from_static("true"));
        headers.insert(
            "x-rigcraft-export-note",
            HeaderValue::from_static(CSV_LOSSY_NOTE),
        );
    }
    Ok(response)
}

struct ScratchFile(PathBuf);

impl ScratchFile {
    fn create(state: &AppState, bytes: &[u8]) -> Result<Self, ApiError> {
        let path = state
            .store
            .db_path()
            .with_file_name(format!("restore-{}.sqlite3", Uuid::new_v4()));
        std::fs::write(&path, bytes).map_err(|error| {
            tracing::error!(error = %error, "backup upload could not be staged");
            ApiError::internal("The uploaded backup could not be staged for validation.")
        })?;
        Ok(Self(path))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for ScratchFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

struct BackupUpload {
    file_name: String,
    bytes: Vec<u8>,
}

async fn read_backup_upload(
    multipart: Result<Multipart, MultipartRejection>,
    require_confirm: bool,
) -> Result<BackupUpload, ApiError> {
    let mut multipart = multipart.map_err(|rejection| {
        ApiError::bad_request(
            "invalid_body",
            "Send this request as multipart/form-data with a valid boundary.",
        )
        .with_details(serde_json::json!({ "reason": rejection.body_text() }))
    })?;
    let mut file: Option<(String, Vec<u8>)> = None;
    let mut confirm: Option<String> = None;
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
                        "Send exactly one SQLite backup file.",
                    )
                    .with_field("file"));
                }
                let filename = field
                    .file_name()
                    .map(str::to_string)
                    .unwrap_or_else(|| "inventory.sqlite3".to_string());
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|error| ApiError::bad_request("file_unreadable", error.to_string()))?;
                file = Some((filename, bytes.to_vec()));
            }
            "confirm" => {
                confirm = Some(field.text().await.map_err(|error| {
                    ApiError::bad_request("multipart_error", error.to_string())
                })?);
            }
            _ => {
                let _ = field.bytes().await;
            }
        }
    }
    let Some((file_name, bytes)) = file else {
        return Err(ApiError::bad_request(
            "backup_file_required",
            "Choose a SQLite backup file to inspect or restore.",
        )
        .with_field("file"));
    };
    if require_confirm && confirm.as_deref().map(str::trim) != Some("restore") {
        return Err(ApiError::bad_request(
            "backup_confirm_required",
            "Type restore to confirm replacing the live inventory.",
        )
        .with_field("confirm"));
    }
    Ok(BackupUpload { file_name, bytes })
}

fn backup_problem(problem: BackupProblem) -> ApiError {
    match problem {
        BackupProblem::Empty => ApiError::bad_request(
            "backup_empty",
            "The uploaded backup is empty. Choose a SQLite backup file.",
        )
        .with_field("file"),
        BackupProblem::NotSqlite => ApiError::bad_request(
            "backup_not_sqlite",
            "The uploaded file is not a SQLite database.",
        )
        .with_field("file"),
        BackupProblem::Corrupt => ApiError::unprocessable(
            "backup_corrupt",
            "The uploaded database failed SQLite's integrity check and was not restored.",
        )
        .with_field("file"),
        BackupProblem::SchemaTooNew(version) => ApiError::unprocessable(
            "backup_schema_too_new",
            "This backup was written by a newer RigCraft schema than this build supports.",
        )
        .with_field("file")
        .with_details(serde_json::json!({
            "schemaVersion": version,
            "supportedSchemaVersion": SCHEMA_VERSION,
        })),
        BackupProblem::WrongShape(detail) => ApiError::unprocessable(
            "backup_wrong_shape",
            "This database is not a RigCraft inventory catalog.",
        )
        .with_field("file")
        .with_details(serde_json::json!({ "reason": detail })),
    }
}

fn restore_failure(failure: RestoreFailure) -> ApiError {
    match failure {
        RestoreFailure::Problem(problem) => backup_problem(problem),
        RestoreFailure::PreRestoreCopyFailed => ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "backup_pre_restore_failed",
            "The safety copy of the current inventory could not be written, so nothing was restored.",
        ),
        RestoreFailure::RestoreFailed { pre_restore_path } => ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "backup_restore_failed",
            "The restore did not complete. The previous inventory is in the safety copy named below.",
        )
        .with_details(serde_json::json!({ "preRestorePath": pre_restore_path })),
    }
}

pub(crate) async fn download_backup(State(state): State<AppState>) -> Result<Response, ApiError> {
    let temporary = state
        .store
        .db_path()
        .with_file_name(format!("backup-{}.sqlite3.tmp", Uuid::new_v4()));
    let backup = state.store.backup_to(&temporary);
    if let Err(error) = backup {
        let _ = std::fs::remove_file(&temporary);
        return Err(storage_failure(error));
    }
    let bytes = match std::fs::read(&temporary) {
        Ok(bytes) => bytes,
        Err(error) => {
            let _ = std::fs::remove_file(&temporary);
            tracing::error!(error = %error, "backup file could not be read");
            return Err(ApiError::internal("The database backup could not be read."));
        }
    };
    let _ = std::fs::remove_file(&temporary);
    let filename = portability::backup_filename();
    let mut response = bytes.into_response();
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/vnd.sqlite3"),
    );
    if let Ok(disposition) = HeaderValue::from_str(&format!("attachment; filename=\"{filename}\""))
    {
        headers.insert(header::CONTENT_DISPOSITION, disposition);
    }
    Ok(response)
}

pub(crate) async fn inspect_backup(
    State(state): State<AppState>,
    multipart: Result<Multipart, MultipartRejection>,
) -> Result<Json<BackupInspection>, ApiError> {
    let upload = read_backup_upload(multipart, false).await?;
    let scratch = ScratchFile::create(&state, &upload.bytes)?;
    let inspection =
        portability::inspect_sqlite(scratch.path(), &upload.file_name, upload.bytes.len() as u64)
            .map_err(backup_problem)?;
    Ok(Json(inspection))
}

pub(crate) async fn restore_backup(
    State(state): State<AppState>,
    multipart: Result<Multipart, MultipartRejection>,
) -> Result<Json<RestoreSummary>, ApiError> {
    let upload = read_backup_upload(multipart, true).await?;
    let scratch = ScratchFile::create(&state, &upload.bytes)?;
    portability::inspect_sqlite(scratch.path(), &upload.file_name, upload.bytes.len() as u64)
        .map_err(backup_problem)?;
    let summary = state
        .store
        .restore_from(scratch.path())
        .map_err(restore_failure)?;
    tracing::info!(
        operation = "backup_restore",
        snapshots = summary.counts.snapshot_count,
        records = summary.counts.record_count,
        "inventory restored"
    );
    Ok(Json(summary))
}

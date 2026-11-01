//! Snapshot export contracts and read-only backup inspection.
//!
//! The CSV export deliberately uses exactly the columns `POST /api/imports`
//! accepts, so a shop can open it in a spreadsheet and feed it back. That makes
//! it lossy: lot identity and per-field provenance references do not survive.
//! The JSON export is the authoritative artifact.

use std::path::Path;

use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::domain::{Record, Snapshot};
use crate::store::SCHEMA_VERSION;

/// Columns the importer accepts, in the importer's documented order.
pub const CSV_COLUMNS: [&str; 9] = [
    "sku",
    "category",
    "model",
    "quantity",
    "price",
    "currency",
    "specs",
    "source",
    "sourceUrl",
];

pub const EXPORT_FORMAT: &str = "rigcraft.inventory.snapshot";
pub const EXPORT_FORMAT_VERSION: u32 = 1;
pub const CSV_LOSSY_NOTE: &str =
    "CSV export is lossy: it omits lot identity, snapshot metadata and per-field provenance references. Use format=json for the authoritative artifact.";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotExport {
    pub format: String,
    pub format_version: u32,
    pub authoritative: bool,
    pub lossy: bool,
    pub snapshot: Snapshot,
}

/// Render one CSV row per lot, using the importer's own column set.
pub fn snapshot_csv(snapshot: &Snapshot) -> Result<String, csv::Error> {
    let mut writer = csv::WriterBuilder::new()
        .quote_style(csv::QuoteStyle::Necessary)
        .from_writer(Vec::new());
    writer.write_record(CSV_COLUMNS)?;
    for record in &snapshot.records {
        let (source, source_url) = declared_source(record);
        writer.write_record([
            neutralise(&record.sku),
            neutralise(&record.category),
            neutralise(&record.model_raw),
            neutralise(&record.quantity.to_string()),
            neutralise(&money(record.price_minor)),
            neutralise(&record.currency),
            neutralise(&specs_text(record)),
            neutralise(&source),
            neutralise(&source_url),
        ])?;
    }
    let bytes = writer.into_inner().map_err(|error| error.into_error())?;
    String::from_utf8(bytes).map_err(|error| {
        csv::Error::from(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            error.to_string(),
        ))
    })
}

/// Characters a spreadsheet treats as the start of a formula.
const FORMULA_TRIGGERS: [char; 6] = ['=', '+', '-', '@', '\t', '\r'];

/// Stop a spreadsheet from evaluating an exported cell.
///
/// A cell beginning with `=`, `+`, `-`, `@`, a tab or a carriage return is a
/// formula to Excel and Sheets, so third-party model text could execute in an
/// operator's spreadsheet. One leading apostrophe makes the cell text. A value
/// that begins with a quote and then a trigger is quoted by the writer and read
/// as a formula, so it is neutralised too.
///
/// This is lossy: re-importing the file keeps the apostrophe. The JSON export is
/// the authoritative artifact and keeps the raw value.
fn neutralise(value: &str) -> String {
    let mut chars = value.chars();
    let dangerous = match chars.next() {
        Some(first) if FORMULA_TRIGGERS.contains(&first) => true,
        Some('\'') | Some('"') => chars
            .next()
            .is_some_and(|next| FORMULA_TRIGGERS.contains(&next)),
        _ => false,
    };
    if dangerous {
        format!("'{value}")
    } else {
        value.to_string()
    }
}

/// The full-fidelity artifact. It embeds the immutable snapshot, so any field
/// the API exposes is present, including each field's provenance.
pub fn snapshot_json(snapshot: &Snapshot) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&SnapshotExport {
        format: EXPORT_FORMAT.to_string(),
        format_version: EXPORT_FORMAT_VERSION,
        authoritative: true,
        lossy: false,
        snapshot: snapshot.clone(),
    })
}

pub fn export_filename(snapshot: &Snapshot, extension: &str) -> String {
    let short: String = snapshot.id.chars().take(8).collect();
    format!(
        "rigcraft-inventory-snapshot-r{}-{}.{}",
        snapshot.revision, short, extension
    )
}

/// `YYYYMMDDTHHMMSSffffffZ`, safe in a filename, ordered by time, and unique
/// enough that two restores in the same millisecond cannot share a safety copy.
pub fn utc_file_stamp() -> String {
    let now = OffsetDateTime::now_utc();
    format!(
        "{:04}{:02}{:02}T{:02}{:02}{:02}{:06}Z",
        now.year(),
        u8::from(now.month()),
        now.day(),
        now.hour(),
        now.minute(),
        now.second(),
        now.microsecond()
    )
}

pub fn backup_filename() -> String {
    format!("rigcraft-backup-{}.sqlite3", utc_file_stamp())
}

pub fn pre_restore_filename() -> String {
    format!("inventory.sqlite3.pre-restore-{}", utc_file_stamp())
}

fn money(minor: i64) -> String {
    let sign = if minor < 0 { "-" } else { "" };
    let absolute = minor.unsigned_abs();
    format!("{sign}{}.{:02}", absolute / 100, absolute % 100)
}

fn specs_text(record: &Record) -> String {
    record
        .specs
        .iter()
        .map(|spec| spec.raw.as_str())
        .collect::<Vec<_>>()
        .join(";")
}

fn is_http_url(value: &str) -> bool {
    value.starts_with("http://") || value.starts_with("https://")
}

/// The declared catalog source belongs to the model and its specifications.
/// Operator stock facts never carry one, so the model's provenance is the row's
/// declaration. A `user` model means the exported row declares no catalog.
fn declared_source(record: &Record) -> (String, String) {
    match record.provenance.get("model") {
        Some(provenance) if provenance.source.starts_with("catalog:") => (
            provenance.source.clone(),
            if is_http_url(&provenance.reference) {
                provenance.reference.clone()
            } else {
                String::new()
            },
        ),
        _ => ("user".to_string(), String::new()),
    }
}

/// Counts an operator can compare before and after a restore.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseCounts {
    pub schema_version: i64,
    pub snapshot_count: i64,
    pub latest_revision: Option<i64>,
    pub created_at: Option<String>,
    pub record_count: i64,
    pub lot_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BackupInspection {
    pub file_name: String,
    pub size_bytes: u64,
    pub integrity: String,
    pub counts: DatabaseCounts,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RestoreSummary {
    pub restored: bool,
    pub source_schema_version: i64,
    pub pre_restore_path: String,
    pub counts: DatabaseCounts,
}

/// Why an upload is not a restorable RigCraft catalog. Each variant has one
/// documented API code; none of them implies any write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackupProblem {
    Empty,
    NotSqlite,
    Corrupt,
    SchemaTooNew(i64),
    WrongShape(String),
}

impl BackupProblem {
    pub fn code(&self) -> &'static str {
        match self {
            BackupProblem::Empty => "backup_empty",
            BackupProblem::NotSqlite => "backup_not_sqlite",
            BackupProblem::Corrupt => "backup_corrupt",
            BackupProblem::SchemaTooNew(_) => "backup_schema_too_new",
            BackupProblem::WrongShape(_) => "backup_wrong_shape",
        }
    }
}

const REQUIRED_TABLES: [&str; 3] = ["snapshots", "records", "previews"];

const SHAPE_PROBES: [&str; 3] = [
    "SELECT id, created_at, revision, parent_snapshot_id, currency, source_filename,
            source_sha256, note, unit_count, value_minor FROM snapshots LIMIT 0",
    "SELECT id, snapshot_id, position, lot, sku, category, model_raw, model_normalized,
            quantity, price_minor, currency, specs_json, provenance_json, match_json
     FROM records LIMIT 0",
    "SELECT id, created_at, sha256, preview_json, commit_hash, snapshot_id
     FROM previews LIMIT 0",
];

/// Open an upload read-only and validate it without writing anything.
pub fn inspect_sqlite(
    path: &Path,
    file_name: &str,
    size_bytes: u64,
) -> Result<BackupInspection, BackupProblem> {
    if size_bytes == 0 {
        return Err(BackupProblem::Empty);
    }
    let mut header = [0_u8; 16];
    match std::fs::read(path) {
        Ok(bytes) if bytes.len() >= 16 => header.copy_from_slice(&bytes[..16]),
        Ok(_) => return Err(BackupProblem::NotSqlite),
        Err(_) => return Err(BackupProblem::NotSqlite),
    }
    if &header != b"SQLite format 3\0" {
        return Err(BackupProblem::NotSqlite);
    }
    let connection =
        Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|error| {
            match error {
                rusqlite::Error::SqliteFailure(failure, _)
                    if failure.code == rusqlite::ErrorCode::NotADatabase =>
                {
                    BackupProblem::NotSqlite
                }
                _ => BackupProblem::Corrupt,
            }
        })?;
    let integrity: String = connection
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|_| BackupProblem::Corrupt)?;
    if integrity != "ok" {
        return Err(BackupProblem::Corrupt);
    }
    let schema_version: i64 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|_| BackupProblem::Corrupt)?;
    if schema_version > SCHEMA_VERSION {
        return Err(BackupProblem::SchemaTooNew(schema_version));
    }
    if schema_version < 1 {
        return Err(BackupProblem::WrongShape(
            "user_version is below 1; this is not a RigCraft catalog".to_string(),
        ));
    }
    let mut statement = connection
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table'")
        .map_err(|_| BackupProblem::WrongShape("schema could not be read".to_string()))?;
    let tables = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|_| BackupProblem::WrongShape("schema could not be read".to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| BackupProblem::WrongShape("schema could not be read".to_string()))?;
    let missing = REQUIRED_TABLES
        .iter()
        .filter(|table| !tables.iter().any(|name| name == *table))
        .copied()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(BackupProblem::WrongShape(format!(
            "missing {}: {}",
            crate::text::plural(missing.len(), "table", "tables"),
            missing.join(", ")
        )));
    }
    for probe in SHAPE_PROBES {
        if connection.prepare(probe).is_err() {
            return Err(BackupProblem::WrongShape(
                "a required table does not have the expected columns".to_string(),
            ));
        }
    }
    let counts = counts_from(&connection, schema_version)
        .map_err(|_| BackupProblem::WrongShape("counts could not be read".to_string()))?;
    Ok(BackupInspection {
        file_name: file_name.to_string(),
        size_bytes,
        integrity,
        counts,
    })
}

pub fn counts_from(
    connection: &Connection,
    schema_version: i64,
) -> Result<DatabaseCounts, rusqlite::Error> {
    let snapshot_count: i64 =
        connection.query_row("SELECT COUNT(*) FROM snapshots", [], |row| row.get(0))?;
    let record_count: i64 =
        connection.query_row("SELECT COUNT(*) FROM records", [], |row| row.get(0))?;
    let latest = connection
        .query_row(
            "SELECT id, created_at, revision FROM snapshots
             ORDER BY created_at DESC, rowid DESC LIMIT 1",
            [],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )
        .ok();
    let (latest_revision, created_at, lot_count) = match latest {
        Some((id, created_at, revision)) => {
            let lots: i64 = connection.query_row(
                "SELECT COUNT(*) FROM records WHERE snapshot_id = ?1",
                [id],
                |row| row.get(0),
            )?;
            (Some(revision), Some(created_at), lots)
        }
        None => (None, None, 0),
    };
    Ok(DatabaseCounts {
        schema_version,
        snapshot_count,
        latest_revision,
        created_at,
        record_count,
        lot_count,
    })
}

#[cfg(test)]
mod tests {
    use super::neutralise;

    #[test]
    fn every_formula_trigger_is_neutralised() {
        for value in [
            "=1+1", "+1+1", "-1+1", "-", "@SUM(A1)", "\t=1+1", "\r=1+1", "'=1+1", "\"=1+1",
        ] {
            assert_eq!(neutralise(value), format!("'{value}"), "{value:?}");
        }
    }

    #[test]
    fn ordinary_values_keep_their_bytes() {
        for value in [
            "AMD Ryzen 7 7800X3D",
            "449.00",
            "1",
            "USD",
            "socket=AM5;tdp=120W",
            "https://example.invalid/cpu",
            "",
            "'quoted text",
            "\"quoted text",
        ] {
            assert_eq!(neutralise(value), value, "{value:?}");
        }
    }
}

//! Document validation. A header problem is a 400; any row problem refuses the
//! whole document with 422 and names the row index and field.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::domain::{
    normalize_category, normalize_model, normalize_sku, normalize_spec_key, parse_specs,
    supported_categories, supported_spec_keys, Provenance,
};
use crate::error::ApiError;

use super::{
    RowProblem, ValidatedCatalog, ValidatedPart, CATALOG_FORMAT, CATALOG_VERSION, MAX_CATALOG_PARTS,
};

fn placeholder_provenance() -> Provenance {
    Provenance::catalog(
        "catalog:validation@pending",
        "validation",
        "1970-01-01T00:00:00Z",
    )
}

fn header_error(field: &str, code: &str, message: impl Into<String>) -> ApiError {
    ApiError::bad_request(code, message).with_field(field)
}

fn required_text(
    object: &Map<String, Value>,
    field: &str,
    path: &str,
    max: usize,
    missing_code: &str,
) -> Result<String, ApiError> {
    let Some(value) = object.get(field) else {
        return Err(header_error(
            path,
            missing_code,
            format!("`{path}` is required."),
        ));
    };
    let Some(text) = value.as_str() else {
        return Err(header_error(
            path,
            missing_code,
            format!("`{path}` must be a string."),
        ));
    };
    let text = text.trim();
    if text.is_empty() || text.chars().count() > max || text.chars().any(char::is_control) {
        return Err(header_error(
            path,
            missing_code,
            format!("`{path}` must be 1–{max} characters with no control characters."),
        ));
    }
    Ok(text.to_string())
}

fn optional_text(
    object: &Map<String, Value>,
    field: &str,
    path: &str,
    max: usize,
    code: &str,
) -> Result<Option<String>, ApiError> {
    match object.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => {
            let text = text.trim();
            if text.is_empty() {
                return Ok(None);
            }
            if text.chars().count() > max || text.chars().any(char::is_control) {
                return Err(header_error(
                    path,
                    code,
                    format!(
                        "`{path}` must be at most {max} characters with no control characters."
                    ),
                ));
            }
            Ok(Some(text.to_string()))
        }
        Some(_) => Err(header_error(
            path,
            code,
            format!("`{path}` must be a string."),
        )),
    }
}

fn valid_http_url(value: &str) -> bool {
    value.chars().count() <= 500
        && !value
            .chars()
            .any(|ch| ch.is_whitespace() || ch.is_control())
        && (value.starts_with("https://") || value.starts_with("http://"))
        && value.len() > "https://".len()
}

fn valid_timestamp(value: &str) -> bool {
    OffsetDateTime::parse(value, &Rfc3339).is_ok()
}

fn valid_dataset_name(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_alphanumeric()
        && value.chars().count() <= 120
        && chars
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '@' | '/' | '+' | '-'))
}

fn row_problem(row: usize, field: &str, code: &str, message: impl Into<String>) -> RowProblem {
    RowProblem::new(row, field, code, message)
}

fn part_text(
    object: &Map<String, Value>,
    index: usize,
    field: &str,
    max: usize,
    required: bool,
    problems: &mut Vec<RowProblem>,
) -> Option<String> {
    match object.get(field) {
        None | Some(Value::Null) => {
            if required {
                problems.push(row_problem(
                    index,
                    field,
                    "required",
                    format!("`{field}` is required."),
                ));
            }
            None
        }
        Some(Value::String(raw)) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                if required {
                    problems.push(row_problem(
                        index,
                        field,
                        "required",
                        format!("`{field}` is required."),
                    ));
                }
                None
            } else if trimmed.chars().count() > max || trimmed.chars().any(char::is_control) {
                problems.push(row_problem(
                    index,
                    field,
                    "too_long",
                    format!("Keep `{field}` to {max} characters with no control characters."),
                ));
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        Some(_) => {
            problems.push(row_problem(
                index,
                field,
                "catalog_field_type",
                format!("`{field}` must be a string."),
            ));
            None
        }
    }
}

fn validate_part(
    index: usize,
    value: &Value,
    problems: &mut Vec<RowProblem>,
) -> Option<ValidatedPart> {
    let Some(object) = value.as_object() else {
        problems.push(row_problem(
            index,
            "parts",
            "catalog_row_type",
            "Each part must be a JSON object.",
        ));
        return None;
    };
    let mut invalid = false;

    let sku = part_text(object, index, "sku", 120, true, problems);
    let model = part_text(object, index, "model", 200, true, problems);
    let external_id = part_text(object, index, "externalId", 120, false, problems);
    let manufacturer = part_text(object, index, "manufacturer", 200, false, problems);
    let source_url = part_text(object, index, "sourceUrl", 500, false, problems);
    let verified_at = part_text(object, index, "verifiedAt", 40, false, problems);

    let category = match object.get("category") {
        Some(Value::String(raw)) => match normalize_category(raw) {
            Some(category) => Some(category.to_string()),
            None => {
                problems.push(
                    row_problem(
                        index,
                        "category",
                        "category_unsupported",
                        format!("`{}` is not a supported category.", raw.trim()),
                    )
                    .suggest(format!("Use one of: {}.", supported_categories())),
                );
                invalid = true;
                None
            }
        },
        Some(_) => {
            problems.push(row_problem(
                index,
                "category",
                "catalog_field_type",
                "`category` must be a string.",
            ));
            invalid = true;
            None
        }
        None => {
            problems.push(row_problem(
                index,
                "category",
                "required",
                "`category` is required.",
            ));
            invalid = true;
            None
        }
    };

    if let Some(url) = &source_url {
        if !valid_http_url(url) {
            problems.push(row_problem(
                index,
                "sourceUrl",
                "source_url_format",
                "`sourceUrl` must be an absolute http:// or https:// URL.",
            ));
            invalid = true;
        }
    }
    if let Some(stamp) = &verified_at {
        if !valid_timestamp(stamp) {
            problems.push(row_problem(
                index,
                "verifiedAt",
                "catalog_timestamp_invalid",
                "`verifiedAt` must be an RFC 3339 timestamp.",
            ));
            invalid = true;
        }
    }

    let mut specs = Vec::new();
    match object.get("specs") {
        None | Some(Value::Null) => {}
        Some(Value::Object(values)) => {
            let accepted = supported_spec_keys();
            let mut seen = BTreeSet::new();
            for (raw_key, raw_value) in values {
                let key = normalize_spec_key(raw_key);
                if key.is_empty() {
                    problems.push(row_problem(
                        index,
                        "specs",
                        "catalog_spec_key_unknown",
                        "A specification key cannot be empty.",
                    ));
                    invalid = true;
                    continue;
                }
                if !accepted.contains(&key.as_str()) {
                    problems.push(row_problem(
                        index,
                        &format!("specs.{key}"),
                        "catalog_spec_key_unknown",
                        format!(
                            "`{key}` is not an accepted specification key. Accepted keys: {}.",
                            accepted.join(", ")
                        ),
                    ));
                    invalid = true;
                    continue;
                }
                if !seen.insert(key.clone()) {
                    problems.push(row_problem(
                        index,
                        &format!("specs.{key}"),
                        "catalog_spec_duplicate",
                        format!("Specification `{key}` is listed more than once."),
                    ));
                    invalid = true;
                    continue;
                }
                let Some(raw_value) = raw_value.as_str() else {
                    problems.push(row_problem(
                        index,
                        &format!("specs.{key}"),
                        "catalog_field_type",
                        format!("Specification `{key}` must have a string value."),
                    ));
                    invalid = true;
                    continue;
                };
                let raw_value = raw_value.trim();
                if raw_value.is_empty() {
                    problems.push(row_problem(
                        index,
                        &format!("specs.{key}"),
                        "catalog_spec_empty",
                        format!("Specification `{key}` needs a value."),
                    ));
                    invalid = true;
                    continue;
                }
                match parse_specs(&format!("{key}={raw_value}"), &placeholder_provenance()) {
                    Ok(parsed) => specs.extend(parsed),
                    Err(error) => {
                        problems.push(
                            row_problem(
                                index,
                                &format!("specs.{key}"),
                                &error.code,
                                error.message.clone(),
                            )
                            .suggest(
                                error.suggestion.clone().unwrap_or_else(|| {
                                    "Record the value exactly as the source states it.".to_string()
                                }),
                            ),
                        );
                        invalid = true;
                    }
                }
            }
        }
        Some(_) => {
            problems.push(row_problem(
                index,
                "specs",
                "catalog_field_type",
                "`specs` must be an object of accepted key to string value.",
            ));
            invalid = true;
        }
    }

    if invalid || sku.is_none() || model.is_none() || category.is_none() {
        return None;
    }
    let model = model.expect("checked above");
    Some(ValidatedPart {
        external_id,
        sku: sku.expect("checked above"),
        category: category.expect("checked above"),
        model_normalized: normalize_model(&model),
        model,
        manufacturer,
        source_url,
        verified_at,
        specs,
    })
}

/// Validate a whole document. Header errors are 400; any row defect refuses the
/// document with 422 and lists every problem found.
pub fn validate_catalog(document: &Value) -> Result<ValidatedCatalog, ApiError> {
    let Some(object) = document.as_object() else {
        return Err(header_error(
            "catalog",
            "catalog_required",
            "The request must carry a `catalog` JSON object.",
        ));
    };

    match object.get("format").and_then(Value::as_str) {
        Some(CATALOG_FORMAT) => {}
        _ => {
            return Err(header_error(
                "format",
                "catalog_format_unsupported",
                format!(
                "This is not a RigCraft catalog document. Expected `format` = `{CATALOG_FORMAT}`."
            ),
            ))
        }
    }
    match object.get("version").and_then(Value::as_i64) {
        Some(CATALOG_VERSION) => {}
        Some(other) => {
            return Err(header_error(
                "version",
                "catalog_version_unsupported",
                format!("Catalog version {other} is not supported; this build reads version {CATALOG_VERSION}."),
            ))
        }
        None => {
            return Err(header_error(
                "version",
                "catalog_version_unsupported",
                format!("`version` must be the number {CATALOG_VERSION}."),
            ))
        }
    }

    let Some(dataset) = object.get("dataset").and_then(Value::as_object) else {
        return Err(header_error(
            "dataset",
            "catalog_dataset_required",
            "`dataset` must be an object naming the data set.",
        ));
    };
    let name = required_text(dataset, "name", "dataset.name", 120, "catalog_name_invalid")?;
    if !valid_dataset_name(&name) {
        return Err(header_error(
            "dataset.name",
            "catalog_name_invalid",
            "`dataset.name` is the identifier used in provenance. Letters, digits and `. _ @ / + -` only, starting with a letter or digit.",
        ));
    }
    let license = required_text(
        dataset,
        "license",
        "dataset.license",
        200,
        "catalog_license_required",
    )?;
    let source_url = optional_text(
        dataset,
        "sourceUrl",
        "dataset.sourceUrl",
        500,
        "catalog_source_url_invalid",
    )?;
    if let Some(url) = &source_url {
        if !valid_http_url(url) {
            return Err(header_error(
                "dataset.sourceUrl",
                "catalog_source_url_invalid",
                "`dataset.sourceUrl` must be an absolute http:// or https:// URL.",
            ));
        }
    }
    let commit = optional_text(
        dataset,
        "commit",
        "dataset.commit",
        120,
        "catalog_commit_invalid",
    )?;
    let retrieved_at = optional_text(
        dataset,
        "retrievedAt",
        "dataset.retrievedAt",
        40,
        "catalog_timestamp_invalid",
    )?;
    if let Some(stamp) = &retrieved_at {
        if !valid_timestamp(stamp) {
            return Err(header_error(
                "dataset.retrievedAt",
                "catalog_timestamp_invalid",
                "`dataset.retrievedAt` must be an RFC 3339 timestamp.",
            ));
        }
    }

    let Some(raw_parts) = object.get("parts").and_then(Value::as_array) else {
        return Err(header_error(
            "parts",
            "catalog_parts_required",
            "`parts` must be a non-empty array.",
        ));
    };
    if raw_parts.is_empty() {
        return Err(header_error(
            "parts",
            "catalog_empty",
            "The document has no parts, so there is nothing to store.",
        ));
    }
    if raw_parts.len() > MAX_CATALOG_PARTS {
        return Err(header_error(
            "parts",
            "catalog_too_large",
            format!("A catalog may carry at most {MAX_CATALOG_PARTS} parts."),
        ));
    }

    let mut problems = Vec::new();
    let mut parts = Vec::new();
    for (index, raw) in raw_parts.iter().enumerate() {
        if let Some(part) = validate_part(index, raw, &mut problems) {
            parts.push(part);
        }
    }

    // Duplicates are refused, naming both row indexes. A catalog is a claim
    // about distinct parts; merging or dropping one would be a silent decision.
    let mut by_sku: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    let mut by_external: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (index, part) in parts.iter().enumerate() {
        by_sku
            .entry(normalize_sku(&part.sku))
            .or_default()
            .push(index);
        if let Some(external) = &part.external_id {
            by_external
                .entry(normalize_sku(external))
                .or_default()
                .push(index);
        }
    }
    for (sku, rows) in &by_sku {
        if rows.len() > 1 {
            for row in rows {
                let others = rows
                    .iter()
                    .filter(|other| *other != row)
                    .map(usize::to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                problems.push(
                    row_problem(
                        *row,
                        "sku",
                        "catalog_duplicate_sku",
                        format!("SKU `{sku}` also appears at row index {others}."),
                    )
                    .suggest("Keep one part per SKU; a catalog is refused rather than merged."),
                );
            }
        }
    }
    for (external, rows) in &by_external {
        if rows.len() > 1 {
            for row in rows {
                let others = rows
                    .iter()
                    .filter(|other| *other != row)
                    .map(usize::to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                problems.push(row_problem(
                    *row,
                    "externalId",
                    "catalog_duplicate_external_id",
                    format!("External ID `{external}` also appears at row index {others}."),
                ));
            }
        }
    }

    if !problems.is_empty() {
        let first = problems.first().map(|problem| problem.row).unwrap_or(0);
        return Err(ApiError::unprocessable(
            "catalog_invalid",
            "The catalog document was refused. Nothing was stored; correct the named rows or fix the file.",
        )
        .with_row(first)
        .with_details(serde_json::json!({
            "accepted": false,
            "rows": problems,
            "specKeys": supported_spec_keys(),
        })));
    }

    let mut category_counts: BTreeMap<String, i64> = BTreeMap::new();
    for part in &parts {
        *category_counts.entry(part.category.clone()).or_default() += 1;
    }

    Ok(ValidatedCatalog {
        name,
        source_url,
        commit,
        license,
        retrieved_at,
        parts,
        category_counts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document() -> Value {
        serde_json::json!({
            "format": "rigcraft-catalog",
            "version": 1,
            "dataset": {
                "name": "fixture-catalog",
                "license": "CC0-1.0",
                "commit": "abc1234"
            },
            "parts": [
                {
                    "externalId": "MPN-1",
                    "sku": "CPU-1",
                    "category": "cpu",
                    "model": "Synthetic CPU",
                    "specs": { "socket": "AM5", "tdp": "120W" }
                },
                {
                    "sku": "GPU-1",
                    "category": "gpu",
                    "model": "Synthetic GPU",
                    "specs": { "length": "250mm" }
                }
            ]
        })
    }

    #[test]
    fn a_valid_document_counts_categories_and_keeps_specs() {
        let catalog = validate_catalog(&document()).unwrap();
        assert_eq!(catalog.name, "fixture-catalog");
        assert_eq!(catalog.parts.len(), 2);
        assert_eq!(catalog.category_counts["cpu"], 1);
        assert_eq!(catalog.version_label("2026-11-01T00:00:00Z"), "abc1234");
        assert_eq!(catalog.parts[0].specs.len(), 2);
    }

    #[test]
    fn unknown_format_and_version_are_refused() {
        let mut wrong_format = document();
        wrong_format["format"] = Value::String("open-db".to_string());
        assert_eq!(
            validate_catalog(&wrong_format).unwrap_err().code,
            "catalog_format_unsupported"
        );
        let mut wrong_version = document();
        wrong_version["version"] = Value::from(2);
        assert_eq!(
            validate_catalog(&wrong_version).unwrap_err().code,
            "catalog_version_unsupported"
        );
    }

    #[test]
    fn an_invalid_row_names_its_index_and_field() {
        let mut bad = document();
        bad["parts"][1]["model"] = Value::String(String::new());
        let error = validate_catalog(&bad).unwrap_err();
        assert_eq!(error.code, "catalog_invalid");
        let details = error.details.expect("details");
        let rows = details["rows"].as_array().unwrap();
        assert_eq!(rows[0]["row"], 1);
        assert_eq!(rows[0]["field"], "model");
    }

    #[test]
    fn duplicate_skus_name_both_rows() {
        let mut duplicate = document();
        duplicate["parts"][1]["sku"] = Value::String("CPU-1".to_string());
        let error = validate_catalog(&duplicate).unwrap_err();
        let details = error.details.expect("details");
        let rows = details["rows"].as_array().unwrap();
        assert!(rows.iter().any(|row| row["row"] == 0));
        assert!(rows.iter().any(|row| row["row"] == 1));
    }

    #[test]
    fn an_unknown_spec_key_lists_the_accepted_keys() {
        let mut bad = document();
        bad["parts"][0]["specs"] = serde_json::json!({ "socket_size": "AM5" });
        let error = validate_catalog(&bad).unwrap_err();
        let details = error.details.expect("details");
        let message = details["rows"][0]["message"].as_str().unwrap();
        assert!(message.contains("socket"));
        assert!(message.contains("tdp"));
        assert_eq!(details["rows"][0]["field"], "specs.socket_size");
    }

    #[test]
    fn a_unitless_numeric_spec_is_refused_instead_of_guessed() {
        let mut bad = document();
        bad["parts"][0]["specs"] = serde_json::json!({ "tdp": "120" });
        let error = validate_catalog(&bad).unwrap_err();
        let details = error.details.expect("details");
        assert_eq!(details["rows"][0]["code"], "spec_unit");
    }
}

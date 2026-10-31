//! Optional catalog provenance for the model and specifications.

use super::*;

const SOURCE_MAX_LEN: usize = 120;
const SOURCE_URL_MAX_LEN: usize = 500;

/// Validate the optional `source` column. Empty and `user` mean the operator
/// asserted the facts; `catalog:<id>` names a dataset revision. Anything else is
/// rejected rather than recorded as an ambiguous claim.
pub(super) fn parse_catalog_source(raw: &str) -> Result<Option<String>, FieldError> {
    let value = raw.trim();
    if value.is_empty() || value.eq_ignore_ascii_case("user") {
        return Ok(None);
    }
    let invalid = || {
        FieldError::new(
            "source",
            "source_format",
            "Source must be `user` or `catalog:<id>`.",
        )
        .suggest("Use `catalog:buildcores@<commit>` or leave the column empty.")
    };
    if value.chars().count() > SOURCE_MAX_LEN || value.chars().any(|ch| ch.is_control()) {
        return Err(invalid());
    }
    let Some(id) = value.strip_prefix("catalog:") else {
        return Err(invalid());
    };
    let id_valid = !id.is_empty()
        && id.chars().all(|ch| {
            ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '@' | '/' | '+' | '-')
        });
    if !id_valid {
        return Err(invalid());
    }
    Ok(Some(value.to_string()))
}

pub(super) fn parse_source_url(raw: &str) -> Result<Option<String>, FieldError> {
    let value = raw.trim();
    if value.is_empty() {
        return Ok(None);
    }
    let valid = value.chars().count() <= SOURCE_URL_MAX_LEN
        && !value
            .chars()
            .any(|ch| ch.is_whitespace() || ch.is_control())
        && (value.starts_with("https://") || value.starts_with("http://"))
        && value.len() > "https://".len();
    if !valid {
        return Err(FieldError::new(
            "sourceUrl",
            "source_url_format",
            "Source URL must be an absolute http:// or https:// URL.",
        )
        .suggest("Cite the manufacturer page as https://… or leave the column empty."));
    }
    Ok(Some(value.to_string()))
}

/// Resolve the provenance that the model and specifications inherit. Errors are
/// collected so the row reports every invalid value at once.
pub(super) fn catalog_provenance(
    values: &RowValues,
    reference: &str,
    observed_at: &str,
) -> (Option<Provenance>, Vec<FieldError>) {
    let mut errors = Vec::new();
    let source = match parse_catalog_source(&values.source) {
        Ok(source) => source,
        Err(error) => {
            errors.push(error);
            None
        }
    };
    let url = match parse_source_url(&values.source_url) {
        Ok(url) => url,
        Err(error) => {
            errors.push(error);
            None
        }
    };
    match source {
        Some(source) => (
            Some(Provenance::catalog(
                source,
                url.unwrap_or_else(|| reference.to_string()),
                observed_at,
            )),
            errors,
        ),
        None => (None, errors),
    }
}

pub(super) fn match_info_for(catalog: Option<&Provenance>) -> MatchInfo {
    match catalog {
        Some(provenance) => MatchInfo {
            basis: format!(
                "declared catalog facts from {}; RigCraft did not query or verify the source",
                provenance.source
            ),
            confidence: "catalog-declared".to_string(),
        },
        None => MatchInfo::default(),
    }
}

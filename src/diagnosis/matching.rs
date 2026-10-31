//! Bounded pasted-list parsing and honest part matching.
//!
//! A line is matched against the records of one snapshot. There is no licensed
//! catalog in this package, so a line that cannot be matched is reported as
//! unmatched with the evidence that would identify it, never as the nearest
//! SKU. A line whose tokens are contained in more than one record is reported
//! as ambiguous and no candidate is preselected.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::domain::{
    normalize_category, parse_currency, parse_money_minor, Provenance, Record, Snapshot,
};
use crate::error::ApiError;

use super::{MATCH_VERSION, MAX_LINES, MAX_TEXT_BYTES};

pub const MAX_CANDIDATES: usize = 5;
/// A candidate must contain at least this share of the pasted model tokens.
/// Below it the line is unmatched rather than nudged to a nearby part.
const MIN_COVERAGE: f64 = 0.6;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MatchStatus {
    Matched,
    Ambiguous,
    Unmatched,
    Invalid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LineIssue {
    pub code: String,
    pub message: String,
}

impl LineIssue {
    fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MatchCandidate {
    pub sku: String,
    pub category: String,
    pub model: String,
    pub price_minor: i64,
    pub currency: String,
    pub available: i64,
    /// Share of the pasted model tokens this record contains, 0.0–1.0.
    pub coverage: f64,
    /// True only when the record's token set equals the pasted set, or the
    /// pasted text is the record's SKU.
    pub exact: bool,
    pub shared_tokens: Vec<String>,
    /// Tokens the record adds beyond the pasted model; these are the concrete
    /// differences a disambiguation can name.
    pub extra_tokens: Vec<String>,
    /// Pasted tokens the record does not contain.
    pub missing_tokens: Vec<String>,
    pub provenance: Option<Provenance>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ParsedLine {
    pub line: usize,
    pub raw: String,
    pub status: MatchStatus,
    pub category: Option<String>,
    pub model: String,
    pub quantity: i64,
    pub price_minor: Option<i64>,
    pub issues: Vec<LineIssue>,
    pub candidates: Vec<MatchCandidate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disambiguation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duplicate_of: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ParseLimits {
    pub max_lines: usize,
    pub max_bytes: usize,
    pub max_candidates: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ParsedList {
    pub match_version: String,
    pub snapshot_id: String,
    pub currency: String,
    pub line_count: usize,
    pub lines: Vec<ParsedLine>,
    pub limits: ParseLimits,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ParseListRequest {
    #[serde(default)]
    pub snapshot_id: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub currency: Option<String>,
}

/// Normalise a model into comparable tokens. Non-alphanumeric characters
/// separate tokens; a letter/digit transition splits a run only when both sides
/// keep at least two characters, so `RTX4070` yields `rtx` and `4070` while
/// `B650` stays one token.
fn tokens(value: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for run in value
        .split(|ch: char| !ch.is_alphanumeric())
        .filter(|run| !run.is_empty())
    {
        let chars: Vec<char> = run.to_lowercase().chars().collect();
        let mut start = 0;
        for index in 1..chars.len() {
            let boundary = chars[index - 1].is_ascii_digit() != chars[index].is_ascii_digit();
            if boundary && index >= 2 && chars.len() - index >= 2 {
                push_token(&mut out, &chars[start..index]);
                start = index;
            }
        }
        push_token(&mut out, &chars[start..]);
    }
    out
}

fn push_token(out: &mut BTreeSet<String>, chars: &[char]) {
    if chars.len() >= 2 || (chars.len() == 1 && chars[0].is_ascii_digit()) {
        out.insert(chars.iter().collect());
    }
}

struct RawLine {
    category: Option<String>,
    model: String,
    quantity: i64,
    price_minor: Option<i64>,
    issues: Vec<LineIssue>,
}

fn parse_price(raw: &str) -> (Option<i64>, Option<LineIssue>) {
    match parse_money_minor(raw) {
        Ok(value) => (Some(value), None),
        Err(error) => (
            None,
            Some(LineIssue::new(
                &error.code,
                format!(
                    "The price `{raw}` is not readable. {}",
                    error.suggestion.unwrap_or_default()
                )
                .trim()
                .to_string(),
            )),
        ),
    }
}

fn parse_line(raw: &str) -> RawLine {
    let mut rest = raw.trim().to_string();
    let mut issues = Vec::new();
    let mut category = None;

    if let Some((head, tail)) = rest.split_once(':') {
        if let Some(normalized) = normalize_category(head) {
            category = Some(normalized.to_string());
            rest = tail.trim().to_string();
        }
    }
    if category.is_none() {
        if let Some((head, tail)) = rest.split_once(char::is_whitespace) {
            if let Some(normalized) = normalize_category(head) {
                category = Some(normalized.to_string());
                rest = tail.trim().to_string();
            }
        }
    }

    let mut price_minor = None;
    if let Some(index) = rest.rfind(" @") {
        let price_text = rest[index + 2..].trim().to_string();
        rest = rest[..index].trim().to_string();
        if price_text.is_empty() {
            issues.push(LineIssue::new(
                "price_required",
                "The `@` needs a price after it, for example @349.00.",
            ));
        } else {
            let (value, issue) = parse_price(&price_text);
            price_minor = value;
            if let Some(issue) = issue {
                issues.push(issue);
            }
        }
    }

    let mut quantity = 1;
    if let Some(split) = rest.rfind(char::is_whitespace) {
        let candidate = rest[split..].trim();
        let lowered = candidate.to_lowercase();
        if let Some(digits) = lowered.strip_prefix('x') {
            if !digits.is_empty() && digits.chars().all(|ch| ch.is_ascii_digit()) {
                match digits.parse::<i64>() {
                    Ok(value) if value > 0 && value <= 1000 => {
                        quantity = value;
                        rest = rest[..split].trim().to_string();
                    }
                    _ => issues.push(LineIssue::new(
                        "quantity_invalid",
                        "Quantity after `x` must be a whole number from 1 to 1000.",
                    )),
                }
            }
        }
    }

    let model = rest.trim().to_string();
    if model.is_empty() {
        issues.push(LineIssue::new(
            "model_required",
            "A line needs a model after the optional category, quantity and price.",
        ));
    } else if model.chars().count() > 200 {
        issues.push(LineIssue::new(
            "model_too_long",
            "Keep the model to 200 characters or fewer.",
        ));
    } else if model.chars().any(char::is_control) {
        issues.push(LineIssue::new(
            "model_control_character",
            "Remove control characters from the model.",
        ));
    }

    RawLine {
        category,
        model,
        quantity,
        price_minor,
        issues,
    }
}

fn first_by_sku(snapshot: &Snapshot) -> Vec<(String, Record, i64)> {
    let mut grouped: BTreeMap<String, (Record, i64)> = BTreeMap::new();
    for record in &snapshot.records {
        grouped
            .entry(record.sku.clone())
            .and_modify(|entry| entry.1 += record.quantity)
            .or_insert_with(|| (record.clone(), record.quantity));
    }
    grouped
        .into_iter()
        .map(|(sku, (record, available))| (sku, record, available))
        .collect()
}

fn candidates(pool: &[(String, Record, i64)], raw: &RawLine) -> Vec<MatchCandidate> {
    let input = tokens(&raw.model);
    if input.is_empty() {
        return Vec::new();
    }
    let mut scored: Vec<MatchCandidate> = Vec::new();
    for (sku, record, available) in pool {
        if raw
            .category
            .as_deref()
            .is_some_and(|want| want != record.category)
        {
            continue;
        }
        let record_tokens = tokens(if record.model_normalized.is_empty() {
            &record.model_raw
        } else {
            &record.model_normalized
        });
        let sku_match = raw.model.trim().eq_ignore_ascii_case(sku);
        let shared: BTreeSet<String> = input.intersection(&record_tokens).cloned().collect();
        let coverage = if sku_match {
            1.0
        } else {
            shared.len() as f64 / input.len() as f64
        };
        if !sku_match && (coverage < MIN_COVERAGE || shared.is_empty()) {
            continue;
        }
        let extra: BTreeSet<String> = record_tokens.difference(&input).cloned().collect();
        let missing: BTreeSet<String> = input.difference(&record_tokens).cloned().collect();
        let exact = sku_match
            || (shared.len() == input.len()
                && record_tokens.len() == input.len()
                && missing.is_empty()
                && extra.is_empty());
        scored.push(MatchCandidate {
            sku: sku.clone(),
            category: record.category.clone(),
            model: if record.model_normalized.is_empty() {
                record.model_raw.clone()
            } else {
                record.model_normalized.clone()
            },
            price_minor: record.price_minor,
            currency: record.currency.clone(),
            available: *available,
            coverage,
            exact,
            shared_tokens: shared.into_iter().collect(),
            extra_tokens: extra.into_iter().collect(),
            missing_tokens: missing.into_iter().collect(),
            provenance: record.provenance.get("model").cloned(),
        });
    }
    scored.sort_by(|left, right| {
        right
            .exact
            .cmp(&left.exact)
            .then(
                right
                    .coverage
                    .partial_cmp(&left.coverage)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
            .then(left.sku.cmp(&right.sku))
    });
    scored.truncate(MAX_CANDIDATES);
    scored
}

fn disambiguate(candidates: &[MatchCandidate], raw: &RawLine) -> Option<String> {
    let extras: Vec<String> = candidates
        .iter()
        .flat_map(|candidate| candidate.extra_tokens.iter().cloned())
        .collect::<BTreeSet<String>>()
        .into_iter()
        .collect();
    let missing: Vec<String> = candidates
        .iter()
        .flat_map(|candidate| candidate.missing_tokens.iter().cloned())
        .collect::<BTreeSet<String>>()
        .into_iter()
        .collect();
    if candidates.is_empty() {
        return Some(if raw.category.is_some() {
            "No snapshot record of this category shares enough of the pasted tokens. Paste the full model or the SKU, or describe the part by hand with its specifications."
                .to_string()
        } else {
            "No snapshot record shares enough of the pasted tokens. Add the category or the full model, paste the SKU, or describe the part by hand with its specifications."
                .to_string()
        });
    }
    if candidates
        .iter()
        .filter(|candidate| candidate.exact)
        .count()
        > 1
    {
        return Some(format!(
            "{} snapshot records have exactly these model tokens. Paste the SKU to choose one.",
            candidates
                .iter()
                .filter(|candidate| candidate.exact)
                .count()
        ));
    }
    match (extras.is_empty(), missing.is_empty()) {
        (false, _) => Some(format!(
            "The closest records add tokens you did not paste: {}. Confirm the full model, add one of those tokens, or paste the SKU.",
            extras.join(", ")
        )),
        (true, false) => Some(format!(
            "The closest records do not contain: {}. Confirm the exact model or paste the SKU.",
            missing.join(", ")
        )),
        _ => Some(
            "More than one record contains the pasted tokens. Paste the SKU or add a distinguishing token."
                .to_string(),
        ),
    }
}

fn duplicate_key(line: &ParsedLine) -> Option<String> {
    let category = line.category.as_ref()?;
    let text = if line.model.is_empty() {
        None
    } else {
        Some(line.model.to_lowercase())
    };
    text.map(|model| format!("{category}|{model}"))
}

/// Parse and match a bounded pasted list. Nothing is persisted.
pub fn parse_list(
    snapshot: Option<&Snapshot>,
    request: &ParseListRequest,
) -> Result<ParsedList, ApiError> {
    if request.text.len() > MAX_TEXT_BYTES {
        return Err(ApiError::bad_request(
            "text_too_large",
            format!("Paste at most {MAX_TEXT_BYTES} bytes of parts."),
        )
        .with_field("text")
        .with_details(serde_json::json!({ "limitBytes": MAX_TEXT_BYTES })));
    }
    if request.text.trim().is_empty() {
        return Err(ApiError::bad_request(
            "text_required",
            "Paste one part per line, for example `gpu: Fixture RTX 4070`.",
        )
        .with_field("text"));
    }
    let raw_currency = request.currency.as_deref().ok_or_else(|| {
        ApiError::bad_request(
            "currency_required",
            "Choose the currency for any price in the pasted list.",
        )
        .with_field("currency")
    })?;
    let currency = parse_currency(raw_currency).map_err(|error| {
        ApiError::bad_request(&error.code, error.message).with_field("currency")
    })?;
    if let Some(snapshot) = snapshot {
        if currency != snapshot.currency {
            return Err(ApiError::bad_request(
                "currency_mismatch",
                format!(
                    "The snapshot uses {}; the pasted list uses {currency}. One currency per build.",
                    snapshot.currency
                ),
            )
            .with_field("currency")
            .with_details(serde_json::json!({
                "snapshotCurrency": snapshot.currency,
                "requestCurrency": currency,
            })));
        }
    }
    let numbered: Vec<(usize, &str)> = request
        .text
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(index, line)| (index + 1, line.trim()))
        .collect();
    if numbered.len() > MAX_LINES {
        return Err(ApiError::bad_request(
            "line_limit_exceeded",
            format!(
                "Paste at most {MAX_LINES} parts; this list has {}.",
                numbered.len()
            ),
        )
        .with_field("text")
        .with_details(serde_json::json!({ "limit": MAX_LINES, "lines": numbered.len() })));
    }
    let pool = snapshot.map(first_by_sku).unwrap_or_default();
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    let mut lines = Vec::with_capacity(numbered.len());
    for (line_number, raw) in numbered {
        let parsed = parse_line(raw);
        let candidates = if parsed.issues.is_empty() {
            candidates(&pool, &parsed)
        } else {
            Vec::new()
        };
        let exact: Vec<&MatchCandidate> = candidates
            .iter()
            .filter(|candidate| candidate.exact)
            .collect();
        let status = if !parsed.issues.is_empty() {
            MatchStatus::Invalid
        } else if exact.len() == 1 {
            MatchStatus::Matched
        } else if candidates.is_empty() {
            MatchStatus::Unmatched
        } else {
            MatchStatus::Ambiguous
        };
        let mut line = ParsedLine {
            line: line_number,
            raw: raw.to_string(),
            status,
            category: parsed.category.clone(),
            model: parsed.model.clone(),
            quantity: parsed.quantity,
            price_minor: parsed.price_minor,
            issues: parsed.issues.clone(),
            candidates,
            disambiguation: None,
            duplicate_of: None,
        };
        if status == MatchStatus::Ambiguous {
            line.disambiguation = disambiguate(&line.candidates, &parsed);
        } else if status == MatchStatus::Unmatched {
            line.disambiguation = disambiguate(&[], &parsed);
        } else if status == MatchStatus::Invalid {
            line.disambiguation = line.issues.first().map(|issue| issue.message.clone());
        }
        if let Some(key) = duplicate_key(&line) {
            if let Some(first) = seen.get(&key) {
                line.duplicate_of = Some(*first);
                line.issues.push(LineIssue::new(
                    "duplicate_row",
                    format!("Line {first} already describes this part; resolve one of them."),
                ));
            } else {
                seen.insert(key, line_number);
            }
        }
        lines.push(line);
    }
    Ok(ParsedList {
        match_version: MATCH_VERSION.to_string(),
        snapshot_id: snapshot
            .map(|snapshot| snapshot.id.clone())
            .unwrap_or_default(),
        currency,
        line_count: lines.len(),
        lines,
        limits: ParseLimits {
            max_lines: MAX_LINES,
            max_bytes: MAX_TEXT_BYTES,
            max_candidates: MAX_CANDIDATES,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_split_letter_digit_runs_only_when_both_sides_are_long() {
        assert_eq!(
            tokens("Fixture RTX4070 Ti").into_iter().collect::<Vec<_>>(),
            vec!["4070", "fixture", "rtx", "ti"]
        );
        assert_eq!(
            tokens("B650 AM5").into_iter().collect::<Vec<_>>(),
            vec!["am5", "b650"]
        );
    }

    #[test]
    fn a_line_keeps_its_category_quantity_and_price() {
        let parsed = parse_line("gpu: Fixture RTX 4070 x2 @349.00");
        assert_eq!(parsed.category.as_deref(), Some("gpu"));
        assert_eq!(parsed.model, "Fixture RTX 4070");
        assert_eq!(parsed.quantity, 2);
        assert_eq!(parsed.price_minor, Some(34900));
        assert!(parsed.issues.is_empty());
    }

    #[test]
    fn an_unreadable_price_is_reported_not_dropped() {
        let parsed = parse_line("cpu: Fixture CPU @12.345");
        assert_eq!(parsed.price_minor, None);
        assert_eq!(parsed.issues.len(), 1);
        assert_eq!(parsed.issues[0].code, "price_precision");
    }
}

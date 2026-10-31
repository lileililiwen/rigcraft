//! Optional BYOK natural-language interpretation.
//!
//! The layer is an interpreter only. It turns one sentence into a *proposal*
//! whose every field carries its source, and it never runs a search, writes a
//! price, invents a specification, or records a verdict. The deterministic
//! recommendation contract stays the only path that produces results.

use serde::{Deserialize, Serialize};

use crate::domain::Snapshot;
use crate::error::ApiError;
use crate::recommend::RecommendRequest;

pub mod interpret;
pub mod provider;

pub use interpret::{field_changes, interpret, revised_request, FieldChange, Interpretation};
pub use provider::{
    CompletionRequest, MockProvider, OpenAiCompatibleProvider, ProviderAdapter, ProviderFailure,
    SecretKey,
};

/// Version of the interpretation contract, echoed in every response.
pub const PROMPT_VERSION: &str = "ai-parse-v1";
/// Hard cap for the sentence and for the built prompt.
pub const MAX_PROMPT_BYTES: usize = 8 * 1024;
/// Opt-in part summary: at most this many records, and only these fields.
pub const MAX_INVENTORY_LINES: usize = 50;
pub const INVENTORY_SUMMARY_FIELDS: [&str; 5] =
    ["sku", "category", "model", "priceMinor", "quantity"];
pub const USE_CASES: [&str; 3] = ["gaming", "office", "creation"];

const SYSTEM_PROMPT: &str = "You convert one buyer sentence into a JSON object for a PC build request. \
Reply with JSON only: no prose and no code fences. \
Allowed keys: budgetMinor, currency, useCase, inventoryOnly, lockedSkus, excludedSkus. \
Each key maps to {\"value\": <value>, \"excerpt\": <text copied character for character from the sentence>}. \
budgetMinor is a positive integer in minor units (cents). currency is ISO 4217. \
useCase is gaming, office or creation. inventoryOnly is true only when the sentence asks for it. \
lockedSkus are SKUs the buyer already owns. excludedSkus are SKUs to avoid. \
Omit a key the sentence does not state. Never guess. \
Never output prices, specifications, stock, performance numbers or compatibility claims.";

/// A request to interpret a sentence. It is never serialized, so the key
/// cannot be echoed, and it is never logged.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParseRequest {
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub snapshot_id: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub include_inventory_summary: bool,
}

/// The confirmed request a revision starts from.
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CurrentRequest {
    #[serde(default)]
    pub budget_minor: Option<i64>,
    #[serde(default)]
    pub currency: Option<String>,
    #[serde(default)]
    pub use_case: Option<String>,
    #[serde(default)]
    pub locked_skus: Vec<String>,
    #[serde(default)]
    pub excluded_skus: Vec<String>,
}

impl CurrentRequest {
    pub fn from_request(request: &RecommendRequest) -> Self {
        Self {
            budget_minor: request.budget_minor,
            currency: request.currency.clone(),
            use_case: request.use_case.clone(),
            locked_skus: request.locked_skus.clone(),
            excluded_skus: request.excluded_skus.clone(),
        }
    }
}

/// A revision request. It is never serialized either.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviseRequest {
    #[serde(default)]
    pub instruction: String,
    #[serde(default)]
    pub current: CurrentRequest,
    #[serde(default)]
    pub snapshot_id: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub include_inventory_summary: bool,
}

impl ReviseRequest {
    /// The same transport fields as a parse request, with the instruction as text.
    pub fn as_parse(&self) -> ParseRequest {
        ParseRequest {
            text: self.instruction.clone(),
            snapshot_id: self.snapshot_id.clone(),
            api_key: self.api_key.clone(),
            base_url: self.base_url.clone(),
            model: self.model.clone(),
            include_inventory_summary: self.include_inventory_summary,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldSource {
    /// Copied from the sentence; the excerpt is a literal substring.
    Sentence,
    /// Derived from the selected snapshot, such as its currency or an existing SKU.
    Snapshot,
    /// A stated default, such as inventory-only mode.
    Default,
    /// The value could not be established and needs explicit confirmation.
    Unresolved,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProposedField<T> {
    pub value: Option<T>,
    pub source: FieldSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub excerpt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl<T> ProposedField<T> {
    pub fn unresolved(note: impl Into<String>) -> Self {
        Self {
            value: None,
            source: FieldSource::Unresolved,
            excerpt: None,
            note: Some(note.into()),
        }
    }

    pub fn value(&self) -> Option<&T> {
        self.value.as_ref()
    }

    pub fn is_resolved(&self) -> bool {
        self.source != FieldSource::Unresolved && self.value.is_some()
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Ambiguity {
    pub field: String,
    /// `ungrounded_value`, `unknown_sku`, `catalog_unavailable`,
    /// `currency_mismatch`, `locked_excluded_conflict`,
    /// `locked_category_conflict` or `field_unresolved`.
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Proposal {
    pub budget_minor: ProposedField<i64>,
    pub currency: ProposedField<String>,
    pub use_case: ProposedField<String>,
    pub inventory_only: ProposedField<bool>,
    pub locked_skus: ProposedField<Vec<String>>,
    pub excluded_skus: ProposedField<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderIdentity {
    /// `openai-compatible` or `mock`.
    pub kind: String,
    pub base_url: String,
    pub model: String,
}

/// Exactly what left the machine for this interpretation.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PromptScope {
    pub sentence: bool,
    pub currency: bool,
    pub use_cases: Vec<String>,
    pub inventory_fields: Vec<String>,
    pub inventory_records: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AiProposalResponse {
    /// `provider` on a real interpretation.
    pub mode: String,
    pub prompt_version: String,
    pub provider: ProviderIdentity,
    pub proposal: Proposal,
    pub ambiguities: Vec<Ambiguity>,
    /// Model output that was outside the allowed field set and was discarded.
    pub ignored_fields: Vec<String>,
    pub sent: PromptScope,
    pub confirm_required: bool,
    /// Present only when every hard field is resolved; still a proposal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request: Option<RecommendRequest>,
    /// Field-level changes for a revision; absent for a first parse.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<Vec<FieldChange>>,
}

fn field_error(code: &str, message: impl Into<String>, field: &str) -> ApiError {
    ApiError::bad_request(code, message).with_field(field)
}

/// Reject an oversized sentence before any prompt is built or network call made.
pub fn validate_text(text: &str) -> Result<String, ApiError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(field_error(
            "ai_text_required",
            "Describe the build in one sentence, or use the form below.",
            "text",
        ));
    }
    if trimmed.len() > MAX_PROMPT_BYTES {
        return Err(field_error(
            "ai_text_too_large",
            format!("The sentence must be at most {MAX_PROMPT_BYTES} bytes."),
            "text",
        ));
    }
    Ok(trimmed.to_string())
}

pub fn require_key(raw: &str) -> Result<SecretKey, ApiError> {
    let key = SecretKey::new(raw);
    if key.is_empty() {
        return Err(field_error(
            "ai_key_required",
            "No provider key was supplied. Enter your own key for the assistant, or use the deterministic form below — it needs no key.",
            "apiKey",
        ));
    }
    Ok(key)
}

/// HTTPS base URL, or plain HTTP only for a loopback model. Credentials, query
/// strings, fragments, private-network targets and redirects are refused.
pub fn validate_provider_url(raw: &str) -> Result<String, ApiError> {
    let refuse = || {
        field_error(
            "ai_provider_url_invalid",
            "Use an HTTPS base URL such as https://provider.example/v1. Plain HTTP is accepted only for a loopback address such as http://127.0.0.1:11434/v1. Credentials, query strings, fragments and private-network targets are refused.",
            "baseUrl",
        )
    };
    let trimmed = raw.trim();
    let (scheme, rest) = if let Some(rest) = trimmed.strip_prefix("https://") {
        ("https", rest)
    } else if let Some(rest) = trimmed.strip_prefix("http://") {
        ("http", rest)
    } else {
        return Err(refuse());
    };
    if rest.is_empty() || rest.contains('?') || rest.contains('#') || rest.contains('@') {
        return Err(refuse());
    }
    let (authority, path) = match rest.find('/') {
        Some(index) => (&rest[..index], &rest[index..]),
        None => (rest, ""),
    };
    if authority.is_empty() {
        return Err(refuse());
    }
    let (host, port) = split_authority(authority).ok_or_else(refuse)?;
    let bare = host.trim_start_matches('[').trim_end_matches(']');
    if bare.is_empty() {
        return Err(refuse());
    }
    let loopback = is_loopback(bare);
    if scheme == "http" && !loopback {
        return Err(refuse());
    }
    if scheme == "https" && is_private_address(bare) && !loopback {
        return Err(refuse());
    }
    let port = match port {
        Some(value) if value.parse::<u16>().is_ok() => format!(":{value}"),
        Some(_) => return Err(refuse()),
        None => String::new(),
    };
    let base = format!("{scheme}://{host}{port}{path}");
    Ok(base.trim_end_matches('/').to_string())
}

fn split_authority(authority: &str) -> Option<(&str, Option<&str>)> {
    if let Some(stripped) = authority.strip_prefix('[') {
        // IPv6 literal: [::1] or [::1]:8080
        let end = stripped.find(']')?;
        let host = &authority[..=end + 1];
        let tail = &stripped[end + 1..];
        if tail.is_empty() {
            return Some((host, None));
        }
        return Some((host, Some(tail.strip_prefix(':')?)));
    }
    match authority.rsplit_once(':') {
        Some((host, port)) if !port.is_empty() && port.chars().all(|ch| ch.is_ascii_digit()) => {
            Some((host, Some(port)))
        }
        Some(_) => None,
        None => Some((authority, None)),
    }
}

fn is_loopback(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost") || host == "::1" || host.starts_with("127.")
}

fn is_private_address(host: &str) -> bool {
    let octets: Vec<u8> = host
        .split('.')
        .filter_map(|part| part.parse().ok())
        .collect();
    if octets.len() == 4 {
        return matches!(octets[0], 10 | 127)
            || (octets[0] == 172 && (16..=31).contains(&octets[1]))
            || (octets[0] == 192 && octets[1] == 168)
            || (octets[0] == 169 && octets[1] == 254)
            || (octets[0] == 100 && (64..=127).contains(&octets[1]))
            || octets[0] == 0;
    }
    let lowered = host.to_ascii_lowercase();
    lowered.starts_with("fc")
        || lowered.starts_with("fd")
        || lowered.starts_with("fe80")
        || lowered == "::1"
}

/// A prepared prompt and the exact scope it disclosed.
pub struct BuiltPrompt {
    pub body: String,
    pub scope: PromptScope,
    /// SKUs that were actually placed in the prompt, for part grounding.
    pub included_skus: Vec<String>,
}

/// Build the single bounded prompt. The inventory is included only on opt-in,
/// only these five fields, and never more than the cap.
pub fn build_prompt(
    text: &str,
    snapshot: Option<&Snapshot>,
    include_inventory_summary: bool,
    model: &str,
) -> Result<BuiltPrompt, ApiError> {
    let use_cases: Vec<String> = USE_CASES.iter().map(|value| (*value).to_string()).collect();
    let header = match snapshot {
        Some(snapshot) => format!(
            "Sentence:\n{text}\n\nSnapshot currency: {}\nAllowed use cases: {}",
            snapshot.currency,
            use_cases.join(", ")
        ),
        None => format!(
            "Sentence:\n{text}\n\nNo inventory snapshot is selected; neither the currency nor any part name can be checked.\nAllowed use cases: {}",
            use_cases.join(", ")
        ),
    };

    let mut included = 0usize;
    let mut included_skus: Vec<String> = Vec::new();
    let mut body = header.clone();
    if include_inventory_summary {
        if let Some(snapshot) = snapshot {
            let mut lines = String::from(
                "\n\nInventory parts (SKU | category | model | priceMinor | quantity). \
                 Only use a SKU that appears here, and only when the sentence asks for it:",
            );
            for record in snapshot.records.iter().take(MAX_INVENTORY_LINES) {
                let line = format!(
                    "\n{} | {} | {} | {} | {}",
                    record.sku,
                    record.category,
                    record.model_normalized,
                    record.price_minor,
                    record.quantity
                );
                if body.len() + lines.len() + line.len() > MAX_PROMPT_BYTES.saturating_sub(256) {
                    break;
                }
                lines.push_str(&line);
                included_skus.push(record.sku.clone());
            }
            included = included_skus.len();
            body.push_str(&lines);
        }
    }

    let payload = serde_json::json!({
        "model": model,
        "temperature": 0,
        "messages": [
            { "role": "system", "content": SYSTEM_PROMPT },
            { "role": "user", "content": body },
        ],
    });
    let serialized = payload.to_string();
    if serialized.len() > MAX_PROMPT_BYTES {
        return Err(field_error(
            "ai_prompt_too_large",
            format!("The prompt would exceed {MAX_PROMPT_BYTES} bytes. Shorten the sentence."),
            "text",
        ));
    }
    Ok(BuiltPrompt {
        body: serialized,
        scope: PromptScope {
            sentence: true,
            currency: snapshot.is_some(),
            use_cases,
            inventory_fields: if included > 0 {
                INVENTORY_SUMMARY_FIELDS
                    .iter()
                    .map(|field| (*field).to_string())
                    .collect()
            } else {
                Vec::new()
            },
            inventory_records: included,
        },
        included_skus,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{MatchInfo, Provenance, Record, SnapshotSource};
    use std::collections::BTreeMap;

    fn record(sku: &str, category: &str, price_minor: i64) -> Record {
        let user = Provenance::user("csv:ai.csv", "2026-10-31T00:00:00Z");
        Record {
            id: format!("id-{sku}"),
            lot: 1,
            sku: sku.to_string(),
            category: category.to_string(),
            model_raw: format!("Fixture {sku}"),
            model_normalized: format!("Fixture {sku}"),
            quantity: 1,
            price_minor,
            currency: "USD".to_string(),
            specs: Vec::new(),
            provenance: BTreeMap::from([("model".to_string(), user)]),
            match_info: MatchInfo::default(),
        }
    }

    fn snapshot() -> Snapshot {
        let records = vec![
            record("FIX-CPU", "cpu", 20000),
            record("FIX-GPU", "gpu", 30000),
        ];
        Snapshot {
            id: "snapshot-1".to_string(),
            created_at: "2026-10-31T00:00:00Z".to_string(),
            revision: 1,
            parent_snapshot_id: None,
            currency: "USD".to_string(),
            source: SnapshotSource {
                filename: "ai.csv".to_string(),
                sha256: "abc".to_string(),
            },
            note: None,
            totals: Snapshot::totals_for(&records).unwrap(),
            records,
        }
    }

    #[test]
    fn a_secret_key_never_prints_its_value() {
        let key = SecretKey::new("sk-sentinel-never-print");
        assert_eq!(format!("{key:?}"), "[redacted]");
        assert!(!format!("{key:?}").contains("sk-sentinel"));
        assert!(SecretKey::new("   ").is_empty());
        assert_eq!(key.expose_to_provider(), "sk-sentinel-never-print");
    }

    #[test]
    fn provider_urls_are_https_or_loopback_http() {
        assert_eq!(
            validate_provider_url("https://provider.example/v1/").unwrap(),
            "https://provider.example/v1"
        );
        assert_eq!(
            validate_provider_url("http://127.0.0.1:11434/v1").unwrap(),
            "http://127.0.0.1:11434/v1"
        );
        assert!(validate_provider_url("http://localhost:8080/v1").is_ok());
        assert!(validate_provider_url("http://[::1]:8080/v1").is_ok());
        for refused in [
            "http://provider.example/v1",
            "http://198.51.100.7/v1",
            "https://192.168.1.20/v1",
            "https://10.0.0.5/v1",
            "https://user:pass@provider.example/v1",
            "https://provider.example/v1?key=abc",
            "https://provider.example/v1#fragment",
            "ftp://provider.example/v1",
            "https://provider.example:notaport/v1",
        ] {
            assert!(
                validate_provider_url(refused).is_err(),
                "{refused} should be refused"
            );
        }
    }

    #[test]
    fn an_oversized_or_empty_sentence_is_refused() {
        assert_eq!(validate_text("   ").unwrap_err().code, "ai_text_required");
        let long = "a".repeat(MAX_PROMPT_BYTES + 1);
        assert_eq!(validate_text(&long).unwrap_err().code, "ai_text_too_large");
    }

    #[test]
    fn a_missing_key_names_the_deterministic_flow() {
        let error = require_key("  ").unwrap_err();
        assert_eq!(error.code, "ai_key_required");
        assert_eq!(error.field.as_deref(), Some("apiKey"));
        assert!(error.message.contains("deterministic form"));
    }

    #[test]
    fn the_prompt_omits_the_inventory_unless_the_operator_opts_in() {
        let snapshot = snapshot();
        let plain =
            build_prompt("a 1200 gaming build", Some(&snapshot), false, "test-model").unwrap();
        assert_eq!(plain.scope.inventory_records, 0);
        assert!(plain.scope.inventory_fields.is_empty());
        assert!(!plain.body.contains("FIX-CPU"));
        assert!(!plain.body.contains("20000"));
        assert!(plain.body.contains("Snapshot currency: USD"));

        let opted_in =
            build_prompt("a 1200 gaming build", Some(&snapshot), true, "test-model").unwrap();
        assert_eq!(opted_in.scope.inventory_records, 2);
        assert_eq!(opted_in.included_skus, vec!["FIX-CPU", "FIX-GPU"]);
        assert!(opted_in.body.contains("FIX-CPU | cpu"));
        assert!(opted_in.body.len() <= MAX_PROMPT_BYTES);
    }

    #[test]
    fn an_unknown_snapshot_does_not_claim_a_currency() {
        let prompt = build_prompt("a 1200 gaming build", None, false, "test-model").unwrap();
        assert!(!prompt.scope.currency);
        assert!(prompt.body.contains("No inventory snapshot is selected"));
    }
}

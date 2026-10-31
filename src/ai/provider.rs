//! Provider adapters for the optional BYOK interpretation layer.
//!
//! Exactly one outbound call is made per interpretation, to the URL the
//! operator entered, only after they submitted a parse or revision request.
//! Redirects are not followed, the timeout is fixed, there is no retry, and the
//! credential is read from [`SecretKey`] in one place.

use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use crate::error::ApiError;

/// Fixed provider timeout. There is no automatic retry.
pub const PROVIDER_TIMEOUT_SECS: u64 = 20;

/// A provider credential that must never be logged, serialized or persisted.
///
/// The value is readable only through [`SecretKey::expose_to_provider`], which
/// exists so that no other code path needs to hold or print it.
#[derive(Clone, Default)]
pub struct SecretKey(String);

impl SecretKey {
    pub fn new(raw: &str) -> Self {
        Self(raw.trim().to_string())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The single place the raw credential is read. Callers must put it
    /// straight into an outbound provider request header.
    pub fn expose_to_provider(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("[redacted]")
    }
}

/// One outbound completion request. It is never serialized as a whole.
#[derive(Clone)]
pub struct CompletionRequest {
    pub endpoint: String,
    pub model: String,
    pub api_key: SecretKey,
    pub body: String,
}

impl fmt::Debug for CompletionRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CompletionRequest")
            .field("endpoint", &self.endpoint)
            .field("model", &self.model)
            .field("api_key", &self.api_key)
            .finish()
    }
}

/// A provider failure with a stable code and a message safe to return.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderFailure {
    pub code: &'static str,
    pub status: u16,
    pub field: Option<&'static str>,
    pub message: String,
}

impl ProviderFailure {
    fn new(code: &'static str, status: u16, message: impl Into<String>) -> Self {
        Self {
            code,
            status,
            field: None,
            message: message.into(),
        }
    }

    pub fn into_api_error(self) -> ApiError {
        let error = ApiError::new(
            axum::http::StatusCode::from_u16(self.status)
                .unwrap_or(axum::http::StatusCode::BAD_GATEWAY),
            self.code,
            self.message,
        );
        match self.field {
            Some(field) => error.with_field(field),
            None => error,
        }
    }
}

/// The seam the API uses. Tests and offline evidence inject [`MockProvider`].
pub trait ProviderAdapter: Send + Sync {
    /// The adapter name echoed in a proposal. It is never inferred from input.
    fn kind(&self) -> &'static str {
        "openai-compatible"
    }

    /// Returns the raw response body. It is parsed by [`extract_content`] so the
    /// mock and the real adapter share one parsing path.
    fn complete(&self, request: &CompletionRequest) -> Result<String, ProviderFailure>;
}

/// OpenAI-compatible chat completions over HTTPS, or loopback HTTP for a local
/// model. The transport refuses redirects and applies the fixed timeout.
#[derive(Debug, Default, Clone, Copy)]
pub struct OpenAiCompatibleProvider;

impl ProviderAdapter for OpenAiCompatibleProvider {
    fn complete(&self, request: &CompletionRequest) -> Result<String, ProviderFailure> {
        let config = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(PROVIDER_TIMEOUT_SECS)))
            .max_redirects(0)
            .build();
        let agent: ureq::Agent = config.into();
        let sent = agent
            .post(&request.endpoint)
            .header("content-type", "application/json")
            .header(
                "authorization",
                format!("Bearer {}", request.api_key.expose_to_provider()),
            )
            .send(request.body.as_str());
        match sent {
            Ok(mut response) => {
                let status = response.status().as_u16();
                if (300..400).contains(&status) {
                    return Err(ProviderFailure::new(
                        "ai_provider_error",
                        502,
                        "The provider answered with a redirect. Redirects are not followed; use the final provider URL.",
                    ));
                }
                if !response.status().is_success() {
                    return Err(ProviderFailure::new(
                        "ai_provider_error",
                        502,
                        format!("The provider returned HTTP {status}."),
                    ));
                }
                response
                    .body_mut()
                    .read_to_string()
                    .map_err(|_| unreadable_body())
            }
            Err(error) => Err(transport_failure(error)),
        }
    }
}

fn unreadable_body() -> ProviderFailure {
    ProviderFailure::new(
        "ai_provider_error",
        502,
        "The provider response could not be read.",
    )
}

fn transport_failure(error: ureq::Error) -> ProviderFailure {
    match error {
        ureq::Error::Timeout(_) => ProviderFailure::new(
            "ai_provider_timeout",
            504,
            format!(
                "The provider did not answer within {PROVIDER_TIMEOUT_SECS} seconds. Nothing was saved; retry or use the deterministic form."
            ),
        ),
        ureq::Error::StatusCode(status) => ProviderFailure::new(
            "ai_provider_error",
            502,
            format!("The provider returned HTTP {status}."),
        ),
        ureq::Error::Io(_) | ureq::Error::ConnectionFailed => ProviderFailure::new(
            "ai_provider_unreachable",
            502,
            "The provider could not be reached. Check the URL, the network and the model name.",
        ),
        _ => ProviderFailure::new(
            "ai_provider_unreachable",
            502,
            "The provider could not be reached. Check the URL, the network and the model name.",
        ),
    }
}

/// Extract the assistant content from an OpenAI-compatible chat completion.
pub fn extract_content(body: &str) -> Result<String, ProviderFailure> {
    let value: serde_json::Value = serde_json::from_str(body).map_err(|_| {
        ProviderFailure::new(
            "ai_response_invalid",
            422,
            "The provider reply was not JSON in the OpenAI chat-completions shape.",
        )
    })?;
    let content = value
        .get("choices")
        .and_then(|choices| choices.get(0))
        .and_then(|choice| choice.get("message"))
        .and_then(|message| message.get("content"))
        .and_then(|content| content.as_str())
        .ok_or_else(|| {
            ProviderFailure::new(
                "ai_response_invalid",
                422,
                "The provider reply carried no assistant message content.",
            )
        })?;
    Ok(content.to_string())
}

/// One captured outbound request, for tests and offline evidence only.
#[derive(Clone, PartialEq, Eq)]
pub struct CapturedCall {
    pub endpoint: String,
    pub model: String,
    pub authorization: String,
    pub body: String,
}

/// A deterministic in-process provider used by tests and offline evidence.
///
/// It performs no network I/O and is never installed by `AppState::new`, so it
/// cannot answer for a production request.
pub struct MockProvider {
    reply: Mutex<Result<String, ProviderFailure>>,
    calls: AtomicUsize,
    last: Mutex<Option<CapturedCall>>,
}

impl MockProvider {
    /// Replies with an OpenAI-shaped envelope carrying `content`.
    pub fn with_completion(content: &str) -> Self {
        let envelope = serde_json::json!({
            "choices": [{ "message": { "role": "assistant", "content": content } }]
        });
        Self::with_raw_body(&envelope.to_string())
    }

    pub fn with_raw_body(body: &str) -> Self {
        Self {
            reply: Mutex::new(Ok(body.to_string())),
            calls: AtomicUsize::new(0),
            last: Mutex::new(None),
        }
    }

    pub fn failing(failure: ProviderFailure) -> Self {
        Self {
            reply: Mutex::new(Err(failure)),
            calls: AtomicUsize::new(0),
            last: Mutex::new(None),
        }
    }

    pub fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }

    pub fn last_call(&self) -> Option<CapturedCall> {
        self.last
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }
}

impl ProviderAdapter for MockProvider {
    fn kind(&self) -> &'static str {
        "mock"
    }

    fn complete(&self, request: &CompletionRequest) -> Result<String, ProviderFailure> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        *self
            .last
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(CapturedCall {
            endpoint: request.endpoint.clone(),
            model: request.model.clone(),
            authorization: format!("Bearer {}", request.api_key.expose_to_provider()),
            body: request.body.clone(),
        });
        self.reply
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }
}

use axum::extract::rejection::JsonRejection;
use axum::extract::{FromRequest, Request};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::{json, Value};

/// Structured API failure. Every code is stable and consumed by the Web flow.
#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub code: String,
    pub message: String,
    pub field: Option<String>,
    pub row: Option<usize>,
    pub details: Option<Box<Value>>,
}

impl ApiError {
    pub fn new(status: StatusCode, code: &str, message: impl Into<String>) -> Self {
        Self {
            status,
            code: code.to_string(),
            message: message.into(),
            field: None,
            row: None,
            details: None,
        }
    }

    pub fn bad_request(code: &str, message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, code, message)
    }

    pub fn unprocessable(code: &str, message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNPROCESSABLE_ENTITY, code, message)
    }

    pub fn not_found(code: &str, message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, code, message)
    }

    pub fn conflict(code: &str, message: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, code, message)
    }

    pub fn gone(code: &str, message: impl Into<String>) -> Self {
        Self::new(StatusCode::GONE, code, message)
    }

    pub fn with_field(mut self, field: &str) -> Self {
        self.field = Some(field.to_string());
        self
    }

    pub fn with_row(mut self, row: usize) -> Self {
        self.row = Some(row);
        self
    }

    pub fn with_details(mut self, details: Value) -> Self {
        self.details = Some(Box::new(details));
        self
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "internal_error", message)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let mut error = json!({
            "code": self.code,
            "message": self.message,
        });
        if let Some(field) = self.field {
            error["field"] = Value::String(field);
        }
        if let Some(row) = self.row {
            error["row"] = Value::from(row);
        }
        if let Some(details) = self.details {
            error["details"] = *details;
        }
        (self.status, Json(json!({ "error": error }))).into_response()
    }
}

/// JSON body extractor that converts an axum rejection into the documented
/// error envelope instead of the plain-text body the built-in extractor returns.
pub struct ApiJson<T>(pub T);

impl<T, S> FromRequest<S> for ApiJson<T>
where
    Json<T>: FromRequest<S, Rejection = JsonRejection>,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        match Json::<T>::from_request(request, state).await {
            Ok(Json(value)) => Ok(ApiJson(value)),
            Err(rejection) => Err(json_rejection(rejection)),
        }
    }
}

fn json_rejection(rejection: JsonRejection) -> ApiError {
    let status = rejection.status();
    let reason = rejection.body_text();
    let error = match &rejection {
        JsonRejection::MissingJsonContentType(_) => {
            ApiError::bad_request("invalid_body", "Send this request as application/json.")
        }
        JsonRejection::BytesRejection(_) => ApiError::new(
            status,
            "invalid_body",
            "The request body could not be read.",
        ),
        JsonRejection::JsonDataError(_) | JsonRejection::JsonSyntaxError(_) => {
            ApiError::bad_request(
                "invalid_json",
                "The JSON request body could not be parsed. Check its syntax and field types.",
            )
        }
        _ => ApiError::bad_request(
            "invalid_json",
            "The JSON request body could not be parsed. Check its syntax and field types.",
        ),
    };
    error.with_details(json!({ "reason": reason }))
}

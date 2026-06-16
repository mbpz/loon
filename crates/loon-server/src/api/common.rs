//! Common HTTP types for the `loon-server` API surface.
//!
//! Provides:
//! - [`ApiError`] / [`ApiErrorBody`] — typed error responses with
//!   HTTP status-code mapping and machine-readable `code`.
//! - [`ApiResponse`] / [`ApiListResponse`] — uniform success
//!   envelopes so every endpoint serialises the same shape.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use loon_persistence::PaginatedResult;
use serde::Serialize;

/// JSON body returned for every error response.
#[derive(Debug, Serialize)]
pub struct ApiErrorBody {
    pub error: String,
    pub detail: Option<String>,
    pub code: String,
}

/// Typed error returned by every handler. Each variant carries a
/// machine-readable `code` and maps to an HTTP status code via
/// [`ApiError::status_code`].
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("not found: {0}")]
    NotFound(String, String),
    #[error("invalid argument: {0}")]
    InvalidArgument(String, String),
    #[error("conflict: {0}")]
    Conflict(String, String),
    #[error("rate limited")]
    RateLimited(String),
    #[error("upstream error: {0}")]
    Upstream(String),
    #[error("internal error: {0}")]
    Internal(String),
}

impl ApiError {
    /// HTTP status code corresponding to this error.
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::NotFound(_, _) => StatusCode::NOT_FOUND,
            Self::InvalidArgument(_, _) => StatusCode::BAD_REQUEST,
            Self::Conflict(_, _) => StatusCode::CONFLICT,
            Self::RateLimited(_) => StatusCode::TOO_MANY_REQUESTS,
            Self::Upstream(_) => StatusCode::BAD_GATEWAY,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Machine-readable error code returned to clients.
    pub fn code(&self) -> String {
        match self {
            Self::NotFound(_, c) | Self::InvalidArgument(_, c) | Self::Conflict(_, c) => c.clone(),
            Self::RateLimited(_) => "RATE_LIMITED".into(),
            Self::Upstream(_) => "UPSTREAM_ERROR".into(),
            Self::Internal(_) => "INTERNAL".into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = ApiErrorBody {
            error: self.to_string(),
            detail: None,
            code: self.code(),
        };
        (self.status_code(), Json(body)).into_response()
    }
}

impl From<loon_sdk::SdkError> for ApiError {
    fn from(e: loon_sdk::SdkError) -> Self {
        match e {
            loon_sdk::SdkError::AgentNotFound(_) => {
                ApiError::NotFound(e.to_string(), "AGENT_NOT_FOUND".into())
            }
            loon_sdk::SdkError::GuidelineNotFound(_) => {
                ApiError::NotFound(e.to_string(), "GUIDELINE_NOT_FOUND".into())
            }
            loon_sdk::SdkError::SessionNotFound(_) => {
                ApiError::NotFound(e.to_string(), "SESSION_NOT_FOUND".into())
            }
            loon_sdk::SdkError::ToolNotFound(_) => {
                ApiError::NotFound(e.to_string(), "TOOL_NOT_FOUND".into())
            }
            loon_sdk::SdkError::CustomerNotFound(_) => {
                ApiError::NotFound(e.to_string(), "CUSTOMER_NOT_FOUND".into())
            }
            loon_sdk::SdkError::JourneyNotFound(_) => {
                ApiError::NotFound(e.to_string(), "JOURNEY_NOT_FOUND".into())
            }
            _ => ApiError::Internal(e.to_string()),
        }
    }
}

/// Success envelope returned by single-resource endpoints.
#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub data: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<ApiMeta>,
}

/// Pagination / result-set metadata returned alongside
/// [`ApiResponse`] payloads.
#[derive(Debug, Serialize)]
pub struct ApiMeta {
    pub total: Option<usize>,
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}

/// Success envelope returned by collection endpoints. Mirrors the
/// shape of [`loon_persistence::PaginatedResult`] but without
/// exposing `offset` / `limit` (clients compute them from the
/// request).
#[derive(Debug, Serialize)]
pub struct ApiListResponse<T: Serialize> {
    pub items: Vec<T>,
    pub total: usize,
}

impl<T: Serialize> From<PaginatedResult<T>> for ApiListResponse<T> {
    fn from(p: PaginatedResult<T>) -> Self {
        Self {
            items: p.items,
            total: p.total,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use loon_core::{AgentId, CustomerId, GuidelineId, JourneyId, SessionId, ToolId};
    use loon_sdk::SdkError;

    #[test]
    fn status_code_mapping() {
        assert_eq!(
            ApiError::NotFound("x".into(), "FOO".into()).status_code(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            ApiError::InvalidArgument("x".into(), "FOO".into()).status_code(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            ApiError::Conflict("x".into(), "FOO".into()).status_code(),
            StatusCode::CONFLICT
        );
        assert_eq!(
            ApiError::RateLimited("x".into()).status_code(),
            StatusCode::TOO_MANY_REQUESTS
        );
        assert_eq!(
            ApiError::Upstream("x".into()).status_code(),
            StatusCode::BAD_GATEWAY
        );
        assert_eq!(
            ApiError::Internal("x".into()).status_code(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn sdk_error_conversion() {
        let agent_err = ApiError::from(SdkError::AgentNotFound(AgentId::new()));
        assert!(matches!(agent_err, ApiError::NotFound(_, ref c) if c == "AGENT_NOT_FOUND"));

        let guideline_err = ApiError::from(SdkError::GuidelineNotFound(GuidelineId::new()));
        assert!(
            matches!(guideline_err, ApiError::NotFound(_, ref c) if c == "GUIDELINE_NOT_FOUND")
        );

        let session_err = ApiError::from(SdkError::SessionNotFound(SessionId::new()));
        assert!(matches!(session_err, ApiError::NotFound(_, ref c) if c == "SESSION_NOT_FOUND"));

        let tool_err = ApiError::from(SdkError::ToolNotFound(ToolId::new()));
        assert!(matches!(tool_err, ApiError::NotFound(_, ref c) if c == "TOOL_NOT_FOUND"));

        let customer_err = ApiError::from(SdkError::CustomerNotFound(CustomerId::new()));
        assert!(matches!(customer_err, ApiError::NotFound(_, ref c) if c == "CUSTOMER_NOT_FOUND"));

        let journey_err = ApiError::from(SdkError::JourneyNotFound(JourneyId::new()));
        assert!(matches!(journey_err, ApiError::NotFound(_, ref c) if c == "JOURNEY_NOT_FOUND"));

        let validation_err = ApiError::from(SdkError::Validation("bad".into()));
        assert!(matches!(validation_err, ApiError::Internal(_)));
    }
}

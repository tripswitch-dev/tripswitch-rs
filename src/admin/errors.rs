use std::fmt;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ApiError {
    #[serde(default)]
    pub status: u16,
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub request_id: Option<String>,
    #[serde(skip)]
    pub body: Option<String>,
    #[serde(skip)]
    pub retry_after: Option<u64>,
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}: {}", self.status, self.code, self.message)
    }
}

impl std::error::Error for ApiError {}

#[derive(Debug, thiserror::Error)]
pub enum AdminError {
    #[error("not found: {0}")]
    NotFound(ApiError),

    #[error("unauthorized: {0}")]
    Unauthorized(ApiError),

    #[error("forbidden: {0}")]
    Forbidden(ApiError),

    #[error("rate limited: {0}")]
    RateLimited(ApiError),

    #[error("conflict: {0}")]
    Conflict(ApiError),

    #[error("validation error: {0}")]
    Validation(ApiError),

    #[error("server error: {0}")]
    ServerFault(ApiError),

    #[error("transport error: {0}")]
    Transport(#[from] reqwest::Error),
}

impl AdminError {
    pub fn is_not_found(&self) -> bool {
        matches!(self, AdminError::NotFound(_))
    }

    pub fn is_unauthorized(&self) -> bool {
        matches!(self, AdminError::Unauthorized(_))
    }

    pub fn is_forbidden(&self) -> bool {
        matches!(self, AdminError::Forbidden(_))
    }

    pub fn is_rate_limited(&self) -> bool {
        matches!(self, AdminError::RateLimited(_))
    }

    pub fn is_conflict(&self) -> bool {
        matches!(self, AdminError::Conflict(_))
    }

    pub fn is_validation(&self) -> bool {
        matches!(self, AdminError::Validation(_))
    }

    pub fn is_server_fault(&self) -> bool {
        matches!(self, AdminError::ServerFault(_))
    }

    pub fn is_transport(&self) -> bool {
        matches!(self, AdminError::Transport(_))
    }

    pub fn api_error(&self) -> Option<&ApiError> {
        match self {
            AdminError::NotFound(e)
            | AdminError::Unauthorized(e)
            | AdminError::Forbidden(e)
            | AdminError::RateLimited(e)
            | AdminError::Conflict(e)
            | AdminError::Validation(e)
            | AdminError::ServerFault(e) => Some(e),
            AdminError::Transport(_) => None,
        }
    }

    pub(crate) fn from_api_error(err: ApiError) -> Self {
        match err.status {
            401 => AdminError::Unauthorized(err),
            403 => AdminError::Forbidden(err),
            404 => AdminError::NotFound(err),
            409 => AdminError::Conflict(err),
            422 => AdminError::Validation(err),
            429 => AdminError::RateLimited(err),
            500..=599 => AdminError::ServerFault(err),
            _ => AdminError::ServerFault(err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_api_error(status: u16) -> ApiError {
        ApiError {
            status,
            code: "test_code".to_string(),
            message: "test message".to_string(),
            request_id: Some("req_123".to_string()),
            body: Some("raw body".to_string()),
            retry_after: None,
        }
    }

    // ── from_api_error classification ──────────────────────────────

    #[test]
    fn from_api_error_401_unauthorized() {
        let err = AdminError::from_api_error(make_api_error(401));
        assert!(err.is_unauthorized());
        assert!(!err.is_not_found());
    }

    #[test]
    fn from_api_error_403_forbidden() {
        let err = AdminError::from_api_error(make_api_error(403));
        assert!(err.is_forbidden());
        assert!(!err.is_unauthorized());
    }

    #[test]
    fn from_api_error_404_not_found() {
        let err = AdminError::from_api_error(make_api_error(404));
        assert!(err.is_not_found());
        assert!(!err.is_server_fault());
    }

    #[test]
    fn from_api_error_409_conflict() {
        let err = AdminError::from_api_error(make_api_error(409));
        assert!(err.is_conflict());
    }

    #[test]
    fn from_api_error_422_validation() {
        let err = AdminError::from_api_error(make_api_error(422));
        assert!(err.is_validation());
    }

    #[test]
    fn from_api_error_429_rate_limited() {
        let mut api_err = make_api_error(429);
        api_err.retry_after = Some(30);
        let err = AdminError::from_api_error(api_err);
        assert!(err.is_rate_limited());
        let inner = err.api_error().unwrap();
        assert_eq!(inner.retry_after, Some(30));
    }

    #[test]
    fn from_api_error_500_server_fault() {
        let err = AdminError::from_api_error(make_api_error(500));
        assert!(err.is_server_fault());
    }

    #[test]
    fn from_api_error_502_server_fault() {
        let err = AdminError::from_api_error(make_api_error(502));
        assert!(err.is_server_fault());
    }

    #[test]
    fn from_api_error_503_server_fault() {
        let err = AdminError::from_api_error(make_api_error(503));
        assert!(err.is_server_fault());
    }

    #[test]
    fn from_api_error_418_falls_through_to_server_fault() {
        let err = AdminError::from_api_error(make_api_error(418));
        assert!(err.is_server_fault());
    }

    // ── is_* helpers ───────────────────────────────────────────────

    #[test]
    fn is_helpers_return_false_for_wrong_variant() {
        let err = AdminError::from_api_error(make_api_error(404));
        assert!(err.is_not_found());
        assert!(!err.is_unauthorized());
        assert!(!err.is_forbidden());
        assert!(!err.is_rate_limited());
        assert!(!err.is_conflict());
        assert!(!err.is_validation());
        assert!(!err.is_server_fault());
        assert!(!err.is_transport());
    }

    // ── api_error accessor ─────────────────────────────────────────

    #[test]
    fn api_error_returns_inner_for_api_variants() {
        let err = AdminError::from_api_error(make_api_error(404));
        let inner = err.api_error().unwrap();
        assert_eq!(inner.status, 404);
        assert_eq!(inner.code, "test_code");
        assert_eq!(inner.message, "test message");
        assert_eq!(inner.request_id.as_deref(), Some("req_123"));
    }

    #[tokio::test]
    async fn api_error_returns_none_for_transport() {
        // Make a request to an unreachable address to get a real reqwest::Error
        let err = reqwest::Client::new()
            .get("http://127.0.0.1:1")
            .send()
            .await
            .unwrap_err();
        let admin_err = AdminError::Transport(err);
        assert!(admin_err.api_error().is_none());
        assert!(admin_err.is_transport());
    }

    // ── ApiError Display ───────────────────────────────────────────

    #[test]
    fn api_error_display_format() {
        let err = make_api_error(404);
        let display = format!("{err}");
        assert_eq!(display, "[404] test_code: test message");
    }

    // ── ApiError deserialization ────────────────────────────────────

    #[test]
    fn api_error_deserialize_full() {
        let json = r#"{"status":404,"code":"not_found","message":"resource not found","request_id":"req_abc"}"#;
        let err: ApiError = serde_json::from_str(json).unwrap();
        assert_eq!(err.status, 404);
        assert_eq!(err.code, "not_found");
        assert_eq!(err.message, "resource not found");
        assert_eq!(err.request_id.as_deref(), Some("req_abc"));
        // body and retry_after are #[serde(skip)]
        assert!(err.body.is_none());
        assert!(err.retry_after.is_none());
    }

    #[test]
    fn api_error_deserialize_missing_fields_use_defaults() {
        let json = r#"{}"#;
        let err: ApiError = serde_json::from_str(json).unwrap();
        assert_eq!(err.status, 0);
        assert_eq!(err.code, "");
        assert_eq!(err.message, "");
        assert!(err.request_id.is_none());
    }

    #[test]
    fn api_error_deserialize_partial() {
        let json = r#"{"code":"validation_error","message":"name is required"}"#;
        let err: ApiError = serde_json::from_str(json).unwrap();
        assert_eq!(err.status, 0);
        assert_eq!(err.code, "validation_error");
        assert_eq!(err.message, "name is required");
    }
}

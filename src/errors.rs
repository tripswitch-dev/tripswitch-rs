use crate::admin::errors::ApiError;
use std::fmt;

/// Errors returned by the runtime client.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("breaker is open")]
    BreakerOpen,

    #[error("conflicting options: {0}")]
    ConflictingOptions(String),

    #[error("metadata unavailable")]
    MetadataUnavailable,

    #[error("initialization timed out")]
    InitTimeout,

    #[error("API error: {0}")]
    Api(ApiError),

    #[error("transport error: {0}")]
    Transport(#[from] reqwest::Error),
}

impl Error {
    /// Returns true if this is a `BreakerOpen` error.
    pub fn is_breaker_open(&self) -> bool {
        matches!(self, Error::BreakerOpen)
    }
}

/// Error type returned by `execute()` that preserves the user's error type.
#[derive(Debug)]
pub enum ExecuteError<E> {
    /// An error from the SDK (breaker open, conflicting options, etc.).
    Sdk(Error),
    /// An error from the user's task.
    Task(E),
}

impl<E: fmt::Display> fmt::Display for ExecuteError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteError::Sdk(e) => write!(f, "sdk error: {e}"),
            ExecuteError::Task(e) => write!(f, "task error: {e}"),
        }
    }
}

impl<E: fmt::Debug + fmt::Display> std::error::Error for ExecuteError<E> {}

impl<E> From<Error> for ExecuteError<E> {
    fn from(e: Error) -> Self {
        ExecuteError::Sdk(e)
    }
}

impl<E> ExecuteError<E> {
    /// Returns true if this is a breaker-related SDK error.
    pub fn is_breaker_error(&self) -> bool {
        matches!(self, ExecuteError::Sdk(Error::BreakerOpen))
    }

    /// Returns the SDK error if this is one.
    pub fn sdk_error(&self) -> Option<&Error> {
        match self {
            ExecuteError::Sdk(e) => Some(e),
            _ => None,
        }
    }

    /// Returns the task error if this is one.
    pub fn task_error(&self) -> Option<&E> {
        match self {
            ExecuteError::Task(e) => Some(e),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Error::is_breaker_open ─────────────────────────────────────

    #[test]
    fn is_breaker_open_true_for_breaker_open() {
        let err = Error::BreakerOpen;
        assert!(err.is_breaker_open());
    }

    #[test]
    fn is_breaker_open_false_for_other_variants() {
        assert!(!Error::ConflictingOptions("test".into()).is_breaker_open());
        assert!(!Error::MetadataUnavailable.is_breaker_open());
        assert!(!Error::InitTimeout.is_breaker_open());
    }

    // ── Error Display ──────────────────────────────────────────────

    #[test]
    fn error_display_breaker_open() {
        assert_eq!(format!("{}", Error::BreakerOpen), "breaker is open");
    }

    #[test]
    fn error_display_conflicting_options() {
        let err = Error::ConflictingOptions("breakers and select_breakers".into());
        assert_eq!(
            format!("{err}"),
            "conflicting options: breakers and select_breakers"
        );
    }

    #[test]
    fn error_display_metadata_unavailable() {
        assert_eq!(
            format!("{}", Error::MetadataUnavailable),
            "metadata unavailable"
        );
    }

    #[test]
    fn error_display_init_timeout() {
        assert_eq!(
            format!("{}", Error::InitTimeout),
            "initialization timed out"
        );
    }

    // ── ExecuteError::is_breaker_error ─────────────────────────────

    #[test]
    fn execute_error_is_breaker_error_true() {
        let err: ExecuteError<std::io::Error> = ExecuteError::Sdk(Error::BreakerOpen);
        assert!(err.is_breaker_error());
    }

    #[test]
    fn execute_error_is_breaker_error_false_for_other_sdk() {
        let err: ExecuteError<std::io::Error> =
            ExecuteError::Sdk(Error::ConflictingOptions("x".into()));
        assert!(!err.is_breaker_error());
    }

    #[test]
    fn execute_error_is_breaker_error_false_for_task() {
        let err: ExecuteError<std::io::Error> =
            ExecuteError::Task(std::io::Error::other("task failed"));
        assert!(!err.is_breaker_error());
    }

    // ── ExecuteError::sdk_error / task_error ───────────────────────

    #[test]
    fn sdk_error_returns_some_for_sdk() {
        let err: ExecuteError<String> = ExecuteError::Sdk(Error::BreakerOpen);
        assert!(err.sdk_error().is_some());
        assert!(err.task_error().is_none());
    }

    #[test]
    fn task_error_returns_some_for_task() {
        let err: ExecuteError<String> = ExecuteError::Task("my error".to_string());
        assert!(err.task_error().is_some());
        assert_eq!(err.task_error().unwrap(), "my error");
        assert!(err.sdk_error().is_none());
    }

    // ── From<Error> for ExecuteError ───────────────────────────────

    #[test]
    fn from_error_into_execute_error() {
        let err: ExecuteError<String> = Error::BreakerOpen.into();
        assert!(err.is_breaker_error());
        assert!(err.sdk_error().is_some());
    }

    // ── ExecuteError Display ───────────────────────────────────────

    #[test]
    fn execute_error_display_sdk() {
        let err: ExecuteError<String> = ExecuteError::Sdk(Error::BreakerOpen);
        assert_eq!(format!("{err}"), "sdk error: breaker is open");
    }

    #[test]
    fn execute_error_display_task() {
        let err: ExecuteError<String> = ExecuteError::Task("custom error".to_string());
        assert_eq!(format!("{err}"), "task error: custom error");
    }
}

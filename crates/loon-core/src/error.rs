use crate::common::UniqueId;
use thiserror::Error;
pub type CoreResult<T> = Result<T, CoreError>;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("not found: {0}")]
    NotFound(UniqueId),
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("internal error: {0}")]
    Internal(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn error_display() {
        let e = CoreError::NotFound(UniqueId("abc".into()));
        assert_eq!(e.to_string(), "not found: abc");
    }
    #[test]
    fn error_is_std_error() {
        let e: Box<dyn std::error::Error> = Box::new(CoreError::InvalidArgument("x".into()));
        assert!(!e.to_string().is_empty());
    }
}

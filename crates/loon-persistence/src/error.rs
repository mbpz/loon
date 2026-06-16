use thiserror::Error;
pub type PersistenceResult<T> = Result<T, PersistenceError>;

#[derive(Debug, Error)]
pub enum PersistenceError {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid filter: {0}")]
    InvalidFilter(String),
    #[error("serialization error: {0}")]
    Serialization(String),
    #[error("internal error: {0}")]
    Internal(String),
}

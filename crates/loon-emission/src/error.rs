use loon_core::CoreError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EmissionError {
    #[error("persistence failed: {0}")]
    PersistenceFailed(String),
    #[error("emitter not found")]
    EmitterNotFound,
    #[error("serialization: {0}")]
    Serialization(String),
    #[error("core error: {0}")]
    Core(#[from] CoreError),
}

impl From<serde_json::Error> for EmissionError {
    fn from(e: serde_json::Error) -> Self {
        EmissionError::Serialization(e.to_string())
    }
}

pub type EmissionResult<T> = Result<T, EmissionError>;

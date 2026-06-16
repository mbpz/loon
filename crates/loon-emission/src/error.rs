use thiserror::Error;
use loon_core::CoreError;

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

pub type EmissionResult<T> = Result<T, EmissionError>;

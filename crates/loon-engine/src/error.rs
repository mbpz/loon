//! Engine error types and `EngineResult` alias.

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("context load failed: {0}")]
    ContextLoadFailed(String),
    #[error("guideline matching failed: {0}")]
    GuidelineMatchingFailed(String),
    #[error("tool call failed for {0}: {1}")]
    ToolCallFailed(loon_core::ToolId, String),
    #[error("message generation failed: {0}")]
    MessageGenerationFailed(String),
    #[error("hook bailed out")]
    HookBail,
    #[error("core error: {0}")]
    Core(#[from] loon_core::CoreError),
    #[error("nlp error: {0}")]
    Nlp(#[from] loon_nlp::NlpError),
    #[error("emission error: {0}")]
    Emission(#[from] loon_emission::EmissionError),
    #[error("persistence error: {0}")]
    Persistence(#[from] loon_persistence::PersistenceError),
    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

pub type EngineResult<T> = Result<T, EngineError>;

#[cfg(test)]
mod tests {
    use super::*;
    use loon_core::CoreError;
    use loon_core::UniqueId;

    #[test]
    fn display_and_std_error() {
        let e = EngineError::ContextLoadFailed("oops".into());
        assert_eq!(e.to_string(), "context load failed: oops");
        let as_std: Box<dyn std::error::Error> = Box::new(e);
        assert!(!as_std.to_string().is_empty());

        let e2 = EngineError::ToolCallFailed(loon_core::ToolId::new(), "boom".into());
        assert!(e2.to_string().contains("boom"));

        let e3: EngineError = CoreError::NotFound(UniqueId("x".into())).into();
        assert!(e3.to_string().contains("not found"));
    }
}

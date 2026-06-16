use thiserror::Error;
use loon_core::{AgentId, GuidelineId, JourneyId, SessionId, ToolId, CustomerId, CannedResponseId, JourneyNodeId};

#[derive(Debug, Error)]
pub enum SdkError {
    #[error("agent not found: {0}")]
    AgentNotFound(AgentId),
    #[error("guideline not found: {0}")]
    GuidelineNotFound(GuidelineId),
    #[error("journey not found: {0}")]
    JourneyNotFound(JourneyId),
    #[error("session not found: {0}")]
    SessionNotFound(SessionId),
    #[error("tool not found: {0}")]
    ToolNotFound(ToolId),
    #[error("customer not found: {0}")]
    CustomerNotFound(CustomerId),
    #[error("canned response not found: {0}")]
    CannedResponseNotFound(CannedResponseId),
    #[error("node not found: {0}")]
    NodeNotFound(JourneyNodeId),
    #[error("validation error: {0}")]
    Validation(String),
    #[error("core error: {0}")]
    Core(#[from] loon_core::CoreError),
    #[error("nlp error: {0}")]
    Nlp(#[from] loon_nlp::NlpError),
    #[error("persistence error: {0}")]
    Persistence(#[from] loon_persistence::PersistenceError),
    #[error("engine error: {0}")]
    Engine(#[from] loon_engine::EngineError),
    #[error("emission error: {0}")]
    Emission(#[from] loon_emission::EmissionError),
    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

pub type SdkResult<T> = Result<T, SdkError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_messages() {
        let agent_id = AgentId::new();
        let e = SdkError::AgentNotFound(agent_id.clone());
        assert_eq!(e.to_string(), format!("agent not found: {}", agent_id));

        let e = SdkError::Validation("bad".into());
        assert_eq!(e.to_string(), "validation error: bad");

        let e = SdkError::SessionNotFound(SessionId::new());
        assert!(e.to_string().contains("session not found"));
    }
}
use crate::{AgentId, JsonValue, TagId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CompositionMode {
    #[default]
    Fluid,
    Strict,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MessageOutputMode {
    #[default]
    Fluid,
    Canned,
}

/// An agent (bot persona) that the engine drives.
///
/// # Example
///
/// ```
/// # use loon_core::Agent;
/// let agent = Agent::new("support", "helps users with technical questions");
/// assert_eq!(agent.name, "support");
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Agent {
    pub id: AgentId,
    pub name: String,
    pub description: String,
    pub composition_mode: CompositionMode,
    pub message_output_mode: MessageOutputMode,
    pub tags: Vec<TagId>,
    pub creation_utc: DateTime<Utc>,
    pub metadata: JsonValue,
}

impl Agent {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: AgentId::new(),
            name: name.into(),
            description: description.into(),
            composition_mode: CompositionMode::default(),
            message_output_mode: MessageOutputMode::default(),
            tags: vec![],
            creation_utc: Utc::now(),
            metadata: JsonValue::Null,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct AgentUpdateParams {
    pub name: Option<String>,
    pub description: Option<String>,
    pub composition_mode: Option<CompositionMode>,
    pub message_output_mode: Option<MessageOutputMode>,
    pub tags: Option<Vec<TagId>>,
    pub metadata: Option<JsonValue>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn agent_default_composition_mode() {
        let a = Agent::new("support", "helps users");
        assert_eq!(a.composition_mode, CompositionMode::Fluid);
        assert_eq!(a.message_output_mode, MessageOutputMode::Fluid);
    }
    #[test]
    fn agent_update_params() {
        let p = AgentUpdateParams {
            name: Some("new".into()),
            ..Default::default()
        };
        assert_eq!(p.name.as_deref(), Some("new"));
        assert!(p.description.is_none());
    }
}

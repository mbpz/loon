use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::{AgentId, TagId, JsonValue};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CompositionMode { Fluid, Strict }
impl Default for CompositionMode { fn default() -> Self { Self::Fluid } }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MessageOutputMode { Fluid, Canned }
impl Default for MessageOutputMode { fn default() -> Self { Self::Fluid } }

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
        let mut p = AgentUpdateParams::default();
        p.name = Some("new".into());
        assert_eq!(p.name.as_deref(), Some("new"));
        assert!(p.description.is_none());
    }
}

use crate::{AgentId, CustomerId, GuidelineId, JourneyId, SessionId, TagId, ToolId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Tag {
    pub id: TagId,
    pub name: String,
    pub creation_utc: DateTime<Utc>,
}

impl Tag {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: TagId::new(),
            name: name.into(),
            creation_utc: Utc::now(),
        }
    }
    pub fn for_agent_id(id: &AgentId) -> Self {
        Self::new(format!("agent:{}", id))
    }
    pub fn for_guideline_id(id: &GuidelineId) -> Self {
        Self::new(format!("guideline:{}", id))
    }
    pub fn for_journey_id(id: &JourneyId) -> Self {
        Self::new(format!("journey:{}", id))
    }
    pub fn for_tool_id(id: &ToolId) -> Self {
        Self::new(format!("tool:{}", id))
    }
    pub fn for_customer_id(id: &CustomerId) -> Self {
        Self::new(format!("customer:{}", id))
    }
    pub fn for_session_id(id: &SessionId) -> Self {
        Self::new(format!("session:{}", id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tag_factory_for_agent() {
        let id = AgentId::new();
        let tag = Tag::for_agent_id(&id);
        assert_eq!(tag.name, format!("agent:{}", id));
    }
}

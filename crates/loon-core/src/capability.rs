use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::{CapabilityId, RetrieverId};
use crate::TagId;
use crate::AgentId;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Capability {
    pub id: CapabilityId,
    pub agent_id: AgentId,
    pub name: String,
    pub description: String,
    pub tags: Vec<TagId>,
    pub creation_utc: DateTime<Utc>,
}

impl Capability {
    pub fn new(agent_id: &AgentId, name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: CapabilityId::new(),
            agent_id: agent_id.clone(),
            name: name.into(),
            description: description.into(),
            tags: vec![],
            creation_utc: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Retriever {
    pub id: RetrieverId,
    pub agent_id: AgentId,
    pub name: String,
    pub description: String,
    pub tags: Vec<TagId>,
    pub creation_utc: DateTime<Utc>,
}

impl Retriever {
    pub fn new(agent_id: &AgentId, name: impl Into<String>) -> Self {
        Self {
            id: RetrieverId::new(),
            agent_id: agent_id.clone(),
            name: name.into(),
            description: String::new(),
            tags: vec![],
            creation_utc: Utc::now(),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct CapabilityUpdateParams {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn capability_has_creation_time() {
        let c = Capability::new(&AgentId::new(), "x", "y");
        assert!(c.creation_utc.timestamp() > 0);
    }

    #[test]
    fn retriever_starts_with_empty_description() {
        let r = Retriever::new(&AgentId::new(), "x");
        assert_eq!(r.description, "");
    }
}

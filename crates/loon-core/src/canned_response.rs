use crate::AgentId;
use crate::CannedResponseId;
use crate::TagId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CannedResponse {
    pub id: CannedResponseId,
    pub agent_id: AgentId,
    pub value: String,
    pub tags: Vec<TagId>,
    pub matchers: Vec<String>,
    pub creation_utc: DateTime<Utc>,
}

impl CannedResponse {
    pub fn new(agent_id: &AgentId, value: impl Into<String>) -> Self {
        Self {
            id: CannedResponseId::new(),
            agent_id: agent_id.clone(),
            value: value.into(),
            tags: vec![],
            matchers: vec![],
            creation_utc: Utc::now(),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct CannedResponseUpdateParams {
    pub value: Option<String>,
    pub matchers: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canned_response_starts_with_empty_matchers() {
        let c = CannedResponse::new(&AgentId::new(), "hi");
        assert!(c.matchers.is_empty());
        assert_eq!(c.value, "hi");
    }
}

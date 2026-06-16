use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::{AgentId, EvaluationId, ToolId};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Observation {
    pub id: EvaluationId,
    pub agent_id: AgentId,
    pub condition: String,
    pub tools: Vec<ToolId>,
    pub enabled: bool,
    pub creation_utc: DateTime<Utc>,
}

impl Observation {
    pub fn new(condition: impl Into<String>, tools: Vec<ToolId>, agent_id: &AgentId) -> Self {
        Self {
            id: EvaluationId::new(),
            agent_id: agent_id.clone(),
            condition: condition.into(),
            tools,
            enabled: true,
            creation_utc: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn observation_can_hold_multiple_tools() {
        let o = Observation::new("cond", vec![ToolId::new(), ToolId::new()], &AgentId::new());
        assert_eq!(o.tools.len(), 2);
    }
}

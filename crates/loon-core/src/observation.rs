use crate::{AgentId, EvaluationId, ToolId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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

/// Partial-update params for `Observation`. Mutable fields are
/// `condition`, `tools`, and `enabled`; identity is immutable.
#[derive(Debug, Default, Clone)]
pub struct ObservationUpdateParams {
    pub condition: Option<String>,
    pub tools: Option<Vec<ToolId>>,
    pub enabled: Option<bool>,
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

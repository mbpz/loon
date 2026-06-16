use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::ShotId;
use crate::AgentId;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Shot {
    pub id: ShotId,
    pub agent_id: AgentId,
    pub condition: String,
    pub action: String,
    pub example_input: String,
    pub example_output: String,
    pub creation_utc: DateTime<Utc>,
}

impl Shot {
    pub fn new(
        agent_id: &AgentId,
        condition: impl Into<String>,
        action: impl Into<String>,
        input: impl Into<String>,
        output: impl Into<String>,
    ) -> Self {
        Self {
            id: ShotId::new(),
            agent_id: agent_id.clone(),
            condition: condition.into(),
            action: action.into(),
            example_input: input.into(),
            example_output: output.into(),
            creation_utc: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shot_stores_example_pair() {
        let s = Shot::new(&AgentId::new(), "x", "y", "in", "out");
        assert_eq!(s.example_input, "in");
        assert_eq!(s.example_output, "out");
    }
}

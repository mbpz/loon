use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::{AgentId, GuidelineId, TagId, Criticality, JsonValue};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GuidelineContent {
    pub condition: String,
    pub action: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Guideline {
    pub id: GuidelineId,
    pub agent_id: AgentId,
    pub content: GuidelineContent,
    pub criticality: Criticality,
    pub enabled: bool,
    pub tags: Vec<TagId>,
    pub creation_utc: DateTime<Utc>,
    pub metadata: JsonValue,
}

impl Guideline {
    pub fn new(content: GuidelineContent, agent_id: &AgentId, enabled: bool, criticality_int: i32) -> Self {
        Self {
            id: GuidelineId::new(),
            agent_id: agent_id.clone(),
            content,
            criticality: match criticality_int {
                0 => Criticality::Low,
                1 => Criticality::Medium,
                _ => Criticality::High,
            },
            enabled,
            tags: vec![],
            creation_utc: Utc::now(),
            metadata: JsonValue::Null,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct GuidelineUpdateParams {
    pub condition: Option<String>,
    pub action: Option<String>,
    pub enabled: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn guideline_default_has_empty_action() {
        let g = Guideline::new(
            GuidelineContent {
                condition: "x".into(),
                action: "y".into(),
                description: None,
            },
            &AgentId::new(),
            false,
            0,
        );
        assert_eq!(g.content.action, "y");
    }
    #[test]
    fn guideline_update_params_default_none() {
        let p = GuidelineUpdateParams::default();
        assert!(p.condition.is_none());
    }
}

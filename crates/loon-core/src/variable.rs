use crate::common::JsonValue;
use crate::AgentId;
use crate::ContextVariableId;
use crate::TagId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FreshnessRule {
    pub max_age_seconds: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextVariable {
    pub id: ContextVariableId,
    pub agent_id: AgentId,
    pub key: String,
    pub freshness_rules: Vec<FreshnessRule>,
    pub tags: Vec<TagId>,
    pub creation_utc: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextVariableValue {
    pub key: String,
    pub data: JsonValue,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Default, Clone)]
pub struct ContextVariableUpdateParams {
    pub key: Option<String>,
    pub data: Option<JsonValue>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn freshness_rule_no_max_age() {
        let r = FreshnessRule {
            max_age_seconds: None,
        };
        assert!(r.max_age_seconds.is_none());
    }

    #[test]
    fn context_variable_new_has_empty_rules() {
        let v = ContextVariable {
            id: ContextVariableId::new(),
            agent_id: AgentId::new(),
            key: "k".into(),
            freshness_rules: vec![],
            tags: vec![],
            creation_utc: Utc::now(),
        };
        assert!(v.freshness_rules.is_empty());
    }
}

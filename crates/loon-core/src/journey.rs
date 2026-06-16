use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::{AgentId, JourneyId, JourneyNodeId, JourneyEdgeId, TagId, ToolId, JsonValue};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeKind {
    Initial,
    Tool,
    Chat,
    Fork,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JourneyNode {
    pub id: JourneyNodeId,
    pub kind: NodeKind,
    pub action: String,
    pub description: Option<String>,
    pub tools: Vec<ToolId>,
    pub labels: HashMap<String, String>,
    pub metadata: JsonValue,
}

impl JourneyNode {
    pub fn initial() -> Self {
        Self {
            id: JourneyNodeId::new(),
            kind: NodeKind::Initial,
            action: "".into(),
            description: None,
            tools: vec![],
            labels: HashMap::new(),
            metadata: JsonValue::Null,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JourneyEdge {
    pub id: JourneyEdgeId,
    pub source: JourneyNodeId,
    pub target: JourneyNodeId,
    pub condition: String,
    pub metadata: JsonValue,
}

impl JourneyEdge {
    pub fn new(source: JourneyNodeId, target: JourneyNodeId, condition: impl Into<String>) -> Self {
        Self {
            id: JourneyEdgeId::new(),
            source,
            target,
            condition: condition.into(),
            metadata: JsonValue::Null,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Journey {
    pub id: JourneyId,
    pub agent_id: AgentId,
    pub title: String,
    pub description: String,
    pub root_id: JourneyNodeId,
    pub tags: Vec<TagId>,
    pub creation_utc: DateTime<Utc>,
}

#[derive(Debug, Default, Clone)]
pub struct JourneyUpdateParams {
    pub title: Option<String>,
    pub description: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn journey_initial_node_default() {
        let n = JourneyNode::initial();
        assert!(matches!(n.kind, NodeKind::Initial));
    }
    #[test]
    fn journey_edge_connects_two_nodes() {
        let e = JourneyEdge::new(JourneyNodeId::new(), JourneyNodeId::new(), "always");
        assert_eq!(e.condition, "always");
    }
}

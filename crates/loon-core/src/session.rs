use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::{AgentId, CustomerId, SessionId, EventId, ToolCallData, ToolId, JsonValue};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SessionMode {
    Auto,
    Manual,
}

impl Default for SessionMode {
    fn default() -> Self {
        Self::Auto
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Session {
    pub id: SessionId,
    pub agent_id: AgentId,
    pub customer_id: Option<CustomerId>,
    pub title: Option<String>,
    pub mode: SessionMode,
    pub labels: HashSet<String>,
    pub creation_utc: DateTime<Utc>,
}

impl Session {
    pub fn new(agent_id: &AgentId) -> Self {
        Self {
            id: SessionId::new(),
            agent_id: agent_id.clone(),
            customer_id: None,
            title: None,
            mode: SessionMode::default(),
            labels: HashSet::new(),
            creation_utc: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventSource {
    Customer,
    AiAgent,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventKind {
    Status,
    Message,
    Tool,
    Custom,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub id: EventId,
    pub source: EventSource,
    pub kind: EventKind,
    pub trace_id: String,
    pub data: JsonValue,
    pub metadata: Option<HashMap<String, JsonValue>>,
    pub creation_utc: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MessageKind {
    User,
    Agent,
    System,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub kind: MessageKind,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Participant {
    pub id: String,
    pub display_name: String,
}

impl Default for Participant {
    fn default() -> Self {
        Self { id: "agent".into(), display_name: "Agent".into() }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageEventData {
    pub message: String,
    pub participant: Participant,
    pub updated: bool,
}

impl MessageEventData {
    pub fn new(s: impl Into<String>) -> Self {
        Self { message: s.into(), participant: Participant::default(), updated: false }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatusEventData {
    pub stage: String,
    pub details: Option<JsonValue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolEventData {
    pub tool_calls: Vec<ToolCallData>,
}

#[derive(Debug, Default, Clone)]
pub struct EventUpdateParams {
    pub data: Option<JsonValue>,
    pub metadata: Option<HashMap<String, JsonValue>>,
}

#[derive(Debug, Default, Clone)]
pub struct SessionUpdateParams {
    pub title: Option<String>,
    pub mode: Option<SessionMode>,
    pub labels: Option<HashSet<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn session_default_mode_is_auto() {
        let s = Session::new(&AgentId::new());
        assert_eq!(s.mode, SessionMode::Auto);
    }
    #[test]
    fn event_source_serializes() {
        let s = EventSource::AiAgent;
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(json, "\"ai_agent\"");
    }
    #[test]
    fn event_kind_distinct() {
        assert_ne!(EventKind::Status, EventKind::Message);
    }
}

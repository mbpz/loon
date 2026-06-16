use std::collections::HashMap;
use loon_core::{EventSource, EventKind, MessageEventData, JsonValue, StatusEventData, ToolEventData};
use loon_core::async_utils::BoxFuture;

#[derive(Debug, Clone)]
pub struct EmittedEvent {
    pub source: EventSource,
    pub kind: EventKind,
    pub trace_id: String,
    pub data: JsonValue,
    pub metadata: Option<HashMap<String, JsonValue>>,
}

#[derive(Debug, Clone)]
pub enum MessageEmitData {
    Simple(String),
    Structured(MessageEventData),
}

pub type EventUpdater = std::sync::Arc<
    dyn Fn(MessageEventData) -> BoxFuture<'static, crate::EmissionResult<MessageEventHandle>>
        + Send
        + Sync,
>;

pub struct MessageEventHandle {
    pub event: EmittedEvent,
    pub update: EventUpdater,
}

impl std::fmt::Debug for MessageEventHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MessageEventHandle")
            .field("event", &self.event)
            .finish()
    }
}

#[allow(dead_code)]
fn _accepts_trait(_: &dyn std::fmt::Debug) {}

#[cfg(test)]
mod tests {
    use super::*;
    use loon_core::StatusEventData;

    #[test]
    fn emitted_event_constructs() {
        let evt = EmittedEvent {
            source: EventSource::AiAgent,
            kind: EventKind::Status,
            trace_id: "t1".into(),
            data: serde_json::to_value(StatusEventData { stage: "ack".into(), details: None }).unwrap(),
            metadata: None,
        };
        assert_eq!(evt.trace_id, "t1");
        assert!(matches!(evt.kind, EventKind::Status));
    }

    #[test]
    fn message_emit_data_variants() {
        let s = MessageEmitData::Simple("hi".into());
        let d = MessageEmitData::Structured(MessageEventData::new("hi"));
        match s {
            MessageEmitData::Simple(v) => assert_eq!(v, "hi"),
            _ => panic!(),
        }
        match d {
            MessageEmitData::Structured(v) => assert_eq!(v.message, "hi"),
            _ => panic!(),
        }
    }
}

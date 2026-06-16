//! `MessageEventComposer` — wraps a `MessageEventData` payload in
//! an `EmittedEvent` envelope tagged with `EventKind::Message`.

use loon_core::{Agent, EventKind, EventSource, MessageEventData};
use loon_emission::EmittedEvent;

/// Phase-1 composer: produces a `Message`-kind `EmittedEvent`
/// sourced from the AI agent with a fresh `trace_id`.
pub struct MessageEventComposer;

impl MessageEventComposer {
    pub fn compose_message_event(message: MessageEventData, _agent: &Agent) -> EmittedEvent {
        EmittedEvent {
            source: EventSource::AiAgent,
            kind: EventKind::Message,
            trace_id: uuid::Uuid::new_v4().to_string(),
            data: serde_json::to_value(&message).unwrap(),
            metadata: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compose_message_event_emits_message_kind() {
        let msg = MessageEventData::new("hi there");
        let agent = Agent::new("a", "b");
        let evt = MessageEventComposer::compose_message_event(msg, &agent);
        assert!(matches!(evt.kind, EventKind::Message));
        assert!(matches!(evt.source, EventSource::AiAgent));
        assert!(!evt.trace_id.is_empty());
        assert_eq!(evt.data["message"], "hi there");
    }
}

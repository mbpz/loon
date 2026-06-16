use crate::types::EventUpdater;
use crate::{
    EmissionError, EmissionResult, EmittedEvent, EventEmitter, EventEmitterFactory,
    MessageEmitData, MessageEventHandle,
};
use async_trait::async_trait;
use loon_core::{
    Agent, AgentId, EventKind, EventSource, JsonValue, MessageEventData, Participant, SessionId,
    StatusEventData, ToolEventData,
};
use parking_lot::Mutex;
use std::collections::HashMap;

pub struct EventBuffer {
    pub agent: Agent,
    pub events: Mutex<Vec<EmittedEvent>>,
}

impl EventBuffer {
    pub fn new(agent: Agent) -> Self {
        Self {
            agent,
            events: Mutex::new(vec![]),
        }
    }
    pub fn events(&self) -> Vec<EmittedEvent> {
        self.events.lock().clone()
    }
}

#[async_trait]
impl EventEmitter for EventBuffer {
    async fn emit_status_event(
        &self,
        trace_id: &str,
        data: StatusEventData,
        metadata: Option<HashMap<String, JsonValue>>,
    ) -> EmissionResult<EmittedEvent> {
        let event = EmittedEvent {
            source: EventSource::AiAgent,
            kind: EventKind::Status,
            trace_id: trace_id.into(),
            data: serde_json::to_value(&data)?,
            metadata,
        };
        self.events.lock().push(event.clone());
        Ok(event)
    }

    async fn emit_message_event(
        &self,
        trace_id: &str,
        data: MessageEmitData,
        metadata: Option<HashMap<String, JsonValue>>,
    ) -> EmissionResult<MessageEventHandle> {
        let data = match data {
            MessageEmitData::Simple(s) => MessageEventData {
                message: s,
                participant: Participant::default(),
                updated: false,
            },
            MessageEmitData::Structured(d) => d,
        };
        let event = EmittedEvent {
            source: EventSource::AiAgent,
            kind: EventKind::Message,
            trace_id: trace_id.into(),
            data: serde_json::to_value(&data)?,
            metadata,
        };
        self.events.lock().push(event.clone());

        let trace_id_owned = trace_id.to_string();
        let update: EventUpdater = std::sync::Arc::new(move |d: MessageEventData| {
            let trace_id = trace_id_owned.clone();
            Box::pin(async move {
                let evt = EmittedEvent {
                    source: EventSource::AiAgent,
                    kind: EventKind::Message,
                    trace_id,
                    data: serde_json::to_value(&d)?,
                    metadata: None,
                };
                Ok(MessageEventHandle {
                    event: evt,
                    update: std::sync::Arc::new(|_| unreachable!()),
                })
            })
        });
        Ok(MessageEventHandle { event, update })
    }

    async fn emit_tool_event(
        &self,
        trace_id: &str,
        data: ToolEventData,
        metadata: Option<HashMap<String, JsonValue>>,
    ) -> EmissionResult<EmittedEvent> {
        let event = EmittedEvent {
            source: EventSource::AiAgent,
            kind: EventKind::Tool,
            trace_id: trace_id.into(),
            data: serde_json::to_value(&data)?,
            metadata,
        };
        self.events.lock().push(event.clone());
        Ok(event)
    }

    async fn emit_custom_event(
        &self,
        trace_id: &str,
        data: JsonValue,
        metadata: Option<HashMap<String, JsonValue>>,
    ) -> EmissionResult<EmittedEvent> {
        let event = EmittedEvent {
            source: EventSource::System,
            kind: EventKind::Custom,
            trace_id: trace_id.into(),
            data,
            metadata,
        };
        self.events.lock().push(event.clone());
        Ok(event)
    }
}

pub struct EventBufferFactory;

#[async_trait]
impl EventEmitterFactory for EventBufferFactory {
    async fn create_event_emitter(
        &self,
        _agent_id: &AgentId,
        _session_id: &SessionId,
    ) -> EmissionResult<Box<dyn EventEmitter>> {
        Err(EmissionError::EmitterNotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use loon_core::{Agent, StatusEventData, ToolCallData, ToolEventData};

    #[tokio::test]
    async fn buffer_collects_all_event_kinds() {
        let agent = Agent::new("a", "b");
        let buf = EventBuffer::new(agent);
        buf.emit_status_event(
            "t1",
            StatusEventData {
                stage: "ack".into(),
                details: None,
            },
            None,
        )
        .await
        .unwrap();
        let _handle = buf
            .emit_message_event("t1", MessageEmitData::Simple("hi".into()), None)
            .await
            .unwrap();
        buf.emit_tool_event(
            "t1",
            ToolEventData {
                tool_calls: vec![ToolCallData {
                    tool_id: "t".into(),
                    arguments: serde_json::json!({}),
                    result: None,
                }],
            },
            None,
        )
        .await
        .unwrap();
        buf.emit_custom_event("t1", serde_json::json!({"k":1}), None)
            .await
            .unwrap();
        let events = buf.events();
        assert_eq!(events.len(), 4);
        assert!(matches!(events[0].kind, EventKind::Status));
        assert!(matches!(events[1].kind, EventKind::Message));
        assert!(matches!(events[2].kind, EventKind::Tool));
        assert!(matches!(events[3].kind, EventKind::Custom));
    }
}

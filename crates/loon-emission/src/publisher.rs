use async_trait::async_trait;
use loon_core::{Agent, AgentId, SessionId, JsonValue, MessageEventData, StatusEventData, ToolEventData, EventSource, EventKind, Event, EventId, EventUpdateParams, Participant};
use loon_core::stores::SessionStore;
use std::collections::HashMap;
use std::sync::Arc;
use chrono::Utc;
use crate::{EmissionResult, EmittedEvent, MessageEmitData, MessageEventHandle, EventEmitter, EventEmitterFactory, EmissionError};
use crate::types::EventUpdater;

pub struct EventPublisher {
    pub agent: Agent,
    pub session_store: Arc<dyn SessionStore>,
    pub session_id: SessionId,
}

#[async_trait]
impl EventEmitter for EventPublisher {
    async fn emit_status_event(
        &self,
        trace_id: &str,
        data: StatusEventData,
        metadata: Option<HashMap<String, JsonValue>>,
    ) -> EmissionResult<EmittedEvent> {
        let event = Event {
            id: EventId::new(),
            source: EventSource::AiAgent,
            kind: EventKind::Status,
            trace_id: trace_id.into(),
            data: serde_json::to_value(&data)?,
            metadata,
            creation_utc: Utc::now(),
        };
        self.session_store
            .create_event(self.session_id.clone(), event.clone())
            .await?;
        Ok(EmittedEvent {
            source: event.source,
            kind: event.kind,
            trace_id: event.trace_id,
            data: event.data,
            metadata: event.metadata,
        })
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
        let event = Event {
            id: EventId::new(),
            source: EventSource::AiAgent,
            kind: EventKind::Message,
            trace_id: trace_id.into(),
            data: serde_json::to_value(&data)?,
            metadata,
            creation_utc: Utc::now(),
        };
        self.session_store
            .create_event(self.session_id.clone(), event.clone())
            .await?;

        let stored_id = event.id.clone();
        let session_id = self.session_id.clone();
        let session_store = self.session_store.clone();
        let trace_id_owned = trace_id.to_string();
        let update: EventUpdater = std::sync::Arc::new(move |d: MessageEventData| {
            let event_id = stored_id.clone();
            let session_id_clone = session_id.clone();
            let session_store = session_store.clone();
            let trace_id_inner = trace_id_owned.clone();
            Box::pin(async move {
                let updated = Event {
                    id: event_id,
                    source: EventSource::AiAgent,
                    kind: EventKind::Message,
                    trace_id: trace_id_inner,
                    data: serde_json::to_value(&d)?,
                    metadata: None,
                    creation_utc: Utc::now(),
                };
                session_store
                    .update_event(
                        &session_id_clone,
                        &updated.id,
                        EventUpdateParams {
                            data: Some(updated.data.clone()),
                            metadata: updated.metadata.clone(),
                        },
                    )
                    .await?;
                let emitted = EmittedEvent {
                    source: updated.source,
                    kind: updated.kind,
                    trace_id: updated.trace_id,
                    data: updated.data,
                    metadata: updated.metadata,
                };
                Ok(MessageEventHandle {
                    event: emitted,
                    update: std::sync::Arc::new(|_| unreachable!()),
                })
            })
        });

        Ok(MessageEventHandle {
            event: EmittedEvent {
                source: event.source,
                kind: event.kind,
                trace_id: event.trace_id,
                data: event.data,
                metadata: event.metadata,
            },
            update,
        })
    }

    async fn emit_tool_event(
        &self,
        trace_id: &str,
        data: ToolEventData,
        metadata: Option<HashMap<String, JsonValue>>,
    ) -> EmissionResult<EmittedEvent> {
        let event = Event {
            id: EventId::new(),
            source: EventSource::AiAgent,
            kind: EventKind::Tool,
            trace_id: trace_id.into(),
            data: serde_json::to_value(&data)?,
            metadata,
            creation_utc: Utc::now(),
        };
        self.session_store
            .create_event(self.session_id.clone(), event.clone())
            .await?;
        Ok(EmittedEvent {
            source: event.source,
            kind: event.kind,
            trace_id: event.trace_id,
            data: event.data,
            metadata: event.metadata,
        })
    }

    async fn emit_custom_event(
        &self,
        trace_id: &str,
        data: JsonValue,
        metadata: Option<HashMap<String, JsonValue>>,
    ) -> EmissionResult<EmittedEvent> {
        let event = Event {
            id: EventId::new(),
            source: EventSource::System,
            kind: EventKind::Custom,
            trace_id: trace_id.into(),
            data,
            metadata,
            creation_utc: Utc::now(),
        };
        self.session_store
            .create_event(self.session_id.clone(), event.clone())
            .await?;
        Ok(EmittedEvent {
            source: event.source,
            kind: event.kind,
            trace_id: event.trace_id,
            data: event.data,
            metadata: event.metadata,
        })
    }
}

pub struct EventPublisherFactory;

#[async_trait]
impl EventEmitterFactory for EventPublisherFactory {
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
    use loon_core::{Agent, Session, StatusEventData, ToolEventData, ToolCallData, MessageEventData};
    use crate::MessageEmitData;
    use std::sync::Mutex as StdMutex;

    /// Fake SessionStore: tracks created events and supports update_event.
    struct FakeStore {
        pub events: StdMutex<Vec<(SessionId, Event)>>,
        pub updates: StdMutex<Vec<(SessionId, EventId)>>,
    }

    impl FakeStore {
        fn new() -> Self {
            Self {
                events: StdMutex::new(vec![]),
                updates: StdMutex::new(vec![]),
            }
        }
    }

    #[async_trait]
    impl SessionStore for FakeStore {
        async fn create(&self, _s: Session) -> loon_core::CoreResult<Session> {
            unimplemented!()
        }
        async fn read(&self, _id: &SessionId) -> loon_core::CoreResult<Option<Session>> {
            unimplemented!()
        }
        async fn update(
            &self,
            _id: &SessionId,
            _p: loon_core::SessionUpdateParams,
        ) -> loon_core::CoreResult<Session> {
            unimplemented!()
        }
        async fn delete(&self, _id: &SessionId) -> loon_core::CoreResult<()> {
            unimplemented!()
        }
        async fn list(
            &self,
            _agent_id: Option<&AgentId>,
            _customer_id: Option<&loon_core::CustomerId>,
        ) -> loon_core::CoreResult<Vec<Session>> {
            unimplemented!()
        }
        async fn create_event(
            &self,
            session_id: SessionId,
            event: Event,
        ) -> loon_core::CoreResult<Event> {
            self.events.lock().unwrap().push((session_id, event.clone()));
            Ok(event)
        }
        async fn update_event(
            &self,
            session_id: &SessionId,
            event_id: &EventId,
            _p: EventUpdateParams,
        ) -> loon_core::CoreResult<Event> {
            self.updates.lock().unwrap().push((session_id.clone(), event_id.clone()));
            // Return any matching event if we have it, else fabricate one
            let evt = Event {
                id: event_id.clone(),
                source: EventSource::AiAgent,
                kind: EventKind::Message,
                trace_id: "t".into(),
                data: serde_json::json!({}),
                metadata: None,
                creation_utc: Utc::now(),
            };
            Ok(evt)
        }
        async fn read_events(
            &self,
            _session_id: &SessionId,
        ) -> loon_core::CoreResult<Vec<Event>> {
            Ok(vec![])
        }
        async fn find_events(
            &self,
            _session_id: &SessionId,
        ) -> loon_core::CoreResult<Vec<Event>> {
            Ok(vec![])
        }
    }

    #[tokio::test]
    async fn publisher_writes_status_event_to_store() {
        let agent = Agent::new("a", "b");
        let session_id = SessionId::new();
        let store = Arc::new(FakeStore::new());
        let pub_ = EventPublisher {
            agent: agent.clone(),
            session_store: store.clone(),
            session_id: session_id.clone(),
        };
        let emitted = pub_
            .emit_status_event(
                "t1",
                StatusEventData { stage: "ack".into(), details: None },
                None,
            )
            .await
            .unwrap();
        assert_eq!(emitted.trace_id, "t1");
        let stored = store.events.lock().unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].0, session_id);
    }

    #[tokio::test]
    async fn publisher_message_event_returns_handle_with_working_update() {
        let agent = Agent::new("a", "b");
        let session_id = SessionId::new();
        let store = Arc::new(FakeStore::new());
        let pub_ = EventPublisher {
            agent: agent.clone(),
            session_store: store.clone(),
            session_id: session_id.clone(),
        };
        let handle = pub_
            .emit_message_event("t1", MessageEmitData::Simple("hi".into()), None)
            .await
            .unwrap();
        assert_eq!(handle.event.trace_id, "t1");
        let stored = store.events.lock().unwrap();
        assert_eq!(stored.len(), 1);
        let stored_id = stored[0].1.id.clone();
        drop(stored);

        // Call the updater
        let _ = (handle.update)(MessageEventData::new("updated")).await.unwrap();
        let updates = store.updates.lock().unwrap();
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].0, session_id);
        assert_eq!(updates[0].1, stored_id);
    }

    #[tokio::test]
    async fn publisher_writes_tool_event() {
        let agent = Agent::new("a", "b");
        let store = Arc::new(FakeStore::new());
        let pub_ = EventPublisher {
            agent,
            session_store: store.clone(),
            session_id: SessionId::new(),
        };
        let _ = pub_
            .emit_tool_event(
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
        let stored = store.events.lock().unwrap();
        assert_eq!(stored.len(), 1);
    }

    #[tokio::test]
    async fn publisher_writes_custom_event() {
        let agent = Agent::new("a", "b");
        let store = Arc::new(FakeStore::new());
        let pub_ = EventPublisher {
            agent,
            session_store: store.clone(),
            session_id: SessionId::new(),
        };
        let _ = pub_
            .emit_custom_event("t1", serde_json::json!({"k":1}), None)
            .await
            .unwrap();
        let stored = store.events.lock().unwrap();
        assert_eq!(stored.len(), 1);
    }
}

//! WS chat endpoint with streaming via `tokio::sync::mpsc`.
//!
//! Protocol:
//! - Client → Server: `{"type":"user_message","content":"..."}`
//! - Server → Client: `{"type":"agent_message","delta":"..."}` (one per
//!   `Message` event the engine emits during `Engine::process`; arrives
//!   *as the engine emits it*, not buffered until process() returns)
//! - Server → Client: `{"type":"done"}` (terminator after the engine
//!   has finished one user turn)
//!
//! The bridge from engine → WS is a [`StreamingEventEmitter`] that
//! forwards every `Message` / `Status` event into an mpsc channel
//! the WS handler reads from.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use futures::{SinkExt, StreamExt};

use crate::app::AppState;
use loon_core::{
    EventKind, EventSource, JsonValue, MessageEventData, Participant, SessionId, StatusEventData,
    ToolEventData,
};
use loon_emission::{
    EmissionResult, EmittedEvent, EventEmitter, MessageEmitData, MessageEventHandle,
};

/// `GET /v1/sessions/:id/chat` — upgrade to a chat WebSocket.
pub async fn chat_ws(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<SessionId>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state, session_id))
}

/// One frame the WS handler sends back to the client. Pulled
/// out as an enum so the engine-driven assembly logic can be
/// tested without standing up a real WebSocket transport.
#[derive(Debug, Clone, PartialEq)]
pub enum OutgoingFrame {
    /// A single agent message delta (one per message event the
    /// engine emitted during `Engine::process`).
    AgentMessage(String),
    /// Terminal marker signalling the engine has finished
    /// responding to the current user input.
    Done,
}

impl OutgoingFrame {
    /// Render this frame as the JSON wire format the existing
    /// client speaks (the loon CLI REPL).
    pub fn to_wire_json(&self) -> String {
        match self {
            OutgoingFrame::AgentMessage(delta) => {
                format!(
                    r#"{{"type":"agent_message","delta":{}}}"#,
                    serde_json::Value::String(delta.clone())
                )
            }
            OutgoingFrame::Done => r#"{"type":"done"}"#.to_string(),
        }
    }
}

/// `EventEmitter` that forwards each `emit_message_event` to a
/// `tokio::sync::mpsc::Sender<OutgoingFrame>` as the engine produces
/// it. Status / tool / custom events are dropped — only message
/// events surface to the WS client today.
pub struct StreamingEventEmitter {
    tx: tokio::sync::mpsc::Sender<OutgoingFrame>,
}

impl StreamingEventEmitter {
    pub fn new(tx: tokio::sync::mpsc::Sender<OutgoingFrame>) -> Self {
        Self { tx }
    }

    fn extract_message(data: &MessageEmitData) -> String {
        match data {
            MessageEmitData::Simple(s) => s.clone(),
            MessageEmitData::Structured(m) => m.message.clone(),
        }
    }
}

#[async_trait]
impl EventEmitter for StreamingEventEmitter {
    async fn emit_status_event(
        &self,
        trace_id: &str,
        data: StatusEventData,
        _: Option<HashMap<String, JsonValue>>,
    ) -> EmissionResult<EmittedEvent> {
        // Status events are not forwarded to the wire (Phase 3 keeps
        // them server-side); they show up only in server logs / OTel.
        Ok(EmittedEvent {
            source: EventSource::AiAgent,
            kind: EventKind::Status,
            trace_id: trace_id.into(),
            data: serde_json::to_value(&data).unwrap_or(JsonValue::Null),
            metadata: None,
        })
    }

    async fn emit_message_event(
        &self,
        trace_id: &str,
        data: MessageEmitData,
        _: Option<HashMap<String, JsonValue>>,
    ) -> EmissionResult<MessageEventHandle> {
        let text = Self::extract_message(&data);
        let frame = OutgoingFrame::AgentMessage(text.clone());
        // best-effort: if the receiver has dropped the channel
        // (client disconnect), we drop the frame silently.
        let _ = self.tx.send(frame).await;

        let event = EmittedEvent {
            source: EventSource::AiAgent,
            kind: EventKind::Message,
            trace_id: trace_id.into(),
            data: serde_json::to_value(&MessageEventData {
                message: text,
                participant: Participant::default(),
                updated: false,
            })
            .unwrap_or(JsonValue::Null),
            metadata: None,
        };
        Ok(MessageEventHandle {
            event,
            update: std::sync::Arc::new(|_| Box::pin(async { unreachable!() })),
        })
    }

    async fn emit_tool_event(
        &self,
        trace_id: &str,
        data: ToolEventData,
        _: Option<HashMap<String, JsonValue>>,
    ) -> EmissionResult<EmittedEvent> {
        Ok(EmittedEvent {
            source: EventSource::AiAgent,
            kind: EventKind::Tool,
            trace_id: trace_id.into(),
            data: serde_json::to_value(&data).unwrap_or(JsonValue::Null),
            metadata: None,
        })
    }

    async fn emit_custom_event(
        &self,
        trace_id: &str,
        data: JsonValue,
        _: Option<HashMap<String, JsonValue>>,
    ) -> EmissionResult<EmittedEvent> {
        Ok(EmittedEvent {
            source: EventSource::System,
            kind: EventKind::Custom,
            trace_id: trace_id.into(),
            data,
            metadata: None,
        })
    }
}

/// Drive the engine for one user message, streaming each agent message
/// delta through `tx` as it arrives, terminated by `OutgoingFrame::Done`.
/// Errors are swallowed: a session that doesn't exist yields just a
/// `Done` frame so the WS client doesn't hang.
pub async fn drive_engine_streaming(
    state: Arc<AppState>,
    session_id: SessionId,
    _content: String,
    tx: tokio::sync::mpsc::Sender<OutgoingFrame>,
) {
    let queries = &state.server.queries;
    let Ok(Some(session)) = queries.session_store.read(&session_id).await else {
        let _ = tx.send(OutgoingFrame::Done).await;
        return;
    };
    let Ok(Some(_agent)) = queries.agent_store.read(&session.agent_id).await else {
        let _ = tx.send(OutgoingFrame::Done).await;
        return;
    };

    let emitter = StreamingEventEmitter::new(tx.clone());
    let ctx = loon_engine::engine_context::Context {
        session_id: session_id.clone(),
        agent_id: session.agent_id.clone(),
    };
    // best-effort: errors are swallowed but Done is still sent so
    // the client doesn't hang
    let _ = state.server.engine.process(&ctx, &emitter).await;
    let _ = tx.send(OutgoingFrame::Done).await;
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>, session_id: SessionId) {
    let (mut sender, mut receiver) = socket.split();
    // Bounded channel so backpressure flows back into the engine if
    // the client is slow.
    let (tx, mut rx) = tokio::sync::mpsc::channel::<OutgoingFrame>(64);

    // Forwarder task: pulls from the channel and writes to the WS sink.
    let forward = tokio::spawn(async move {
        while let Some(frame) = rx.recv().await {
            if sender
                .send(Message::Text(frame.to_wire_json().into()))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    // Reader loop: parse incoming user_message frames, spawn an
    // engine driver per turn that pushes deltas into the channel.
    while let Some(Ok(msg)) = receiver.next().await {
        if let Message::Text(text) = msg {
            let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) else {
                continue;
            };
            if parsed["type"] != "user_message" {
                continue;
            }
            let content = parsed["content"].as_str().unwrap_or("").to_string();
            let state = state.clone();
            let sid = session_id.clone();
            let tx_for_turn = tx.clone();
            // Each turn drives the engine in a fresh task; this lets
            // the engine's emit_message_event push frames into the
            // channel concurrently with the reader loop continuing
            // to listen for more user_message frames.
            tokio::spawn(async move {
                drive_engine_streaming(state, sid, content, tx_for_turn).await;
            });
        }
    }
    // The reader loop exits when the client closes. Drop our tx so
    // the forwarder can exit cleanly.
    drop(tx);
    let _ = forward.await;
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn build_test_state() -> Arc<crate::app::AppState> {
        let queries = loon_core::entity_cq::EntityQueries::in_memory();
        let nlp: Arc<dyn loon_nlp::NlpService> =
            Arc::new(loon_nlp::test_utils::FakeNlpService::new());
        let server = std::sync::Arc::new(
            loon_sdk::Server::builder()
                .with_nlp_service(nlp)
                .with_entity_queries(queries)
                .build()
                .await
                .expect("server build"),
        );
        Arc::new(crate::app::AppState {
            server,
            auth: Arc::new(crate::auth::NoopAuthProvider),
            rate_limiter: Arc::new(crate::middleware::rate_limit::RateLimiter::new(
                crate::middleware::rate_limit::RateLimitConfig::default(),
            )),
        })
    }

    #[tokio::test]
    async fn streaming_emitter_forwards_message_delta() {
        // emit_message_event pushes an AgentMessage frame on the channel
        // *before* the call returns, so we can read it immediately.
        let (tx, mut rx) = tokio::sync::mpsc::channel::<OutgoingFrame>(8);
        let emitter = StreamingEventEmitter::new(tx);
        emitter
            .emit_message_event("t1", MessageEmitData::Simple("hello".into()), None)
            .await
            .unwrap();
        let frame = rx.recv().await.expect("frame should arrive");
        assert_eq!(frame, OutgoingFrame::AgentMessage("hello".into()));
    }

    #[tokio::test]
    async fn streaming_emitter_drops_status_events() {
        // Status events don't reach the WS — only message events do.
        let (tx, mut rx) = tokio::sync::mpsc::channel::<OutgoingFrame>(8);
        let emitter = StreamingEventEmitter::new(tx);
        emitter
            .emit_status_event(
                "t1",
                StatusEventData {
                    stage: "acknowledging".into(),
                    details: None,
                },
                None,
            )
            .await
            .unwrap();
        // Channel should be empty (status not forwarded).
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn drive_engine_streaming_emits_done_terminator() {
        let state = build_test_state().await;
        let queries = state.server.queries.clone();
        let agent = loon_core::Agent::new("a", "b");
        let agent_id = agent.id.clone();
        queries.agent_store.create(agent).await.unwrap();
        let session = loon_core::Session::new(&agent_id);
        let session_id = session.id.clone();
        queries.session_store.create(session).await.unwrap();

        let (tx, mut rx) = tokio::sync::mpsc::channel::<OutgoingFrame>(64);
        drive_engine_streaming(state, session_id, "hi".into(), tx).await;

        // Pull all frames; last one must be Done.
        let mut all = Vec::new();
        while let Ok(f) = rx.try_recv() {
            all.push(f);
        }
        assert!(matches!(all.last(), Some(OutgoingFrame::Done)));
        // No `Hello!` placeholder anywhere
        for f in &all {
            if let OutgoingFrame::AgentMessage(s) = f {
                assert_ne!(s, "Hello!");
            }
        }
    }

    #[tokio::test]
    async fn drive_engine_streaming_returns_done_for_missing_session() {
        let state = build_test_state().await;
        let (tx, mut rx) = tokio::sync::mpsc::channel::<OutgoingFrame>(8);
        // A session id that doesn't exist in the queries graph.
        drive_engine_streaming(state, loon_core::SessionId::new(), "x".into(), tx).await;
        let frame = rx.recv().await.expect("done frame");
        assert_eq!(frame, OutgoingFrame::Done);
    }

    #[test]
    fn outgoing_frame_to_wire_json_uses_expected_keys() {
        assert_eq!(
            OutgoingFrame::AgentMessage("hi".into()).to_wire_json(),
            r#"{"type":"agent_message","delta":"hi"}"#
        );
        assert_eq!(OutgoingFrame::Done.to_wire_json(), r#"{"type":"done"}"#);
    }
}

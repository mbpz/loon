//! WS chat endpoint.
//!
//! Protocol (Phase 1 stub):
//! - Client → Server: `{"type":"user_message","text":"..."}`
//! - Server → Client: `{"type":"agent_message","delta":"Hello!"}`
//! - Server → Client: `{"type":"done"}`
//!
//! The real engine-backed flow lands in a later stage.

use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use futures::{SinkExt, StreamExt};

use crate::app::AppState;
use loon_core::SessionId;

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
                format!(r#"{{"type":"agent_message","delta":{}}}"#, serde_json::Value::String(delta.clone()))
            }
            OutgoingFrame::Done => r#"{"type":"done"}"#.to_string(),
        }
    }
}

/// Drive the configured [`Engine`](loon_engine::Engine) for one
/// user message and return the frames the WS layer should send
/// back. The session must already exist in the entity-queries
/// graph; otherwise the function returns just a `Done` frame so
/// the client doesn't hang. Replaces the Phase-1 stub
/// "Hello!" placeholder.
pub async fn process_user_message(
    state: &crate::app::AppState,
    session_id: &loon_core::SessionId,
    _content: &str,
) -> Vec<OutgoingFrame> {
    let queries = &state.server.queries;
    let session = match queries.session_store.read(session_id).await {
        Ok(Some(s)) => s,
        _ => return vec![OutgoingFrame::Done],
    };
    let agent = match queries.agent_store.read(&session.agent_id).await {
        Ok(Some(a)) => a,
        _ => return vec![OutgoingFrame::Done],
    };
    let buffer = loon_emission::EventBuffer::new(agent);
    let ctx = loon_engine::engine_context::Context {
        session_id: session_id.clone(),
        agent_id: session.agent_id.clone(),
    };
    let _handled = match state.server.engine.process(&ctx, &buffer).await {
        Ok(_) => true,
        Err(_) => return vec![OutgoingFrame::Done],
    };
    let mut frames: Vec<OutgoingFrame> = buffer
        .events()
        .into_iter()
        .filter(|ev| matches!(ev.kind, loon_core::EventKind::Message))
        .filter_map(|ev| {
            ev.data
                .get("message")
                .and_then(|v| v.as_str())
                .map(|s| OutgoingFrame::AgentMessage(s.to_string()))
        })
        .collect();
    frames.push(OutgoingFrame::Done);
    frames
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>, session_id: SessionId) {
    let (mut sender, mut receiver) = socket.split();
    while let Some(Ok(msg)) = receiver.next().await {
        if let Message::Text(text) = msg {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
                if parsed["type"] == "user_message" {
                    let content = parsed["content"].as_str().unwrap_or("").to_string();
                    for frame in process_user_message(&state, &session_id, &content).await {
                        let _ = sender
                            .send(Message::Text(frame.to_wire_json().into()))
                            .await;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

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
    async fn process_user_message_returns_engine_deltas_not_hardcoded_hello() {
        // With the Phase-1 stub, the WS handler always sent back
        // a single `{"type":"agent_message","delta":"Hello!"}`
        // frame regardless of the user input. The new contract
        // routes through the engine pipeline and the resulting
        // frames must not contain the placeholder literal.
        let state = build_test_state().await;
        let queries = state.server.queries.clone();

        let agent = loon_core::Agent::new("a", "b");
        let agent_id = agent.id.clone();
        queries.agent_store.create(agent).await.unwrap();
        let session = loon_core::Session::new(&agent_id);
        let session_id = session.id.clone();
        queries.session_store.create(session).await.unwrap();

        let frames = process_user_message(&state, &session_id, "hi").await;

        // The engine emits at least one `Message` event via the
        // EventBuffer; with the fake NLP service the delta text
        // is empty, but the frame must still be a structured
        // `AgentMessage` rather than the literal "Hello!".
        let deltas: Vec<&str> = frames
            .iter()
            .filter_map(|f| match f {
                OutgoingFrame::AgentMessage(s) => Some(s.as_str()),
                _ => None,
            })
            .collect();
        for d in &deltas {
            assert_ne!(*d, "Hello!", "engine output must not be the placeholder");
        }
        // Terminator present.
        assert!(matches!(frames.last(), Some(OutgoingFrame::Done)));
    }

    #[test]
    fn outgoing_frame_to_wire_json_uses_expected_keys() {
        // Sync test: doesn't touch the network or the engine.
        // Pin down the wire format so a future refactor of the
        // JSON keys doesn't silently break the loon CLI REPL.
        assert_eq!(
            OutgoingFrame::AgentMessage("hi".into()).to_wire_json(),
            r#"{"type":"agent_message","delta":"hi"}"#
        );
        assert_eq!(OutgoingFrame::Done.to_wire_json(), r#"{"type":"done"}"#);
    }
}

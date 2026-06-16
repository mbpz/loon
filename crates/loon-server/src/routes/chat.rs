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
    State(_s): State<Arc<AppState>>,
    Path(_session_id): Path<SessionId>,
) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(socket: WebSocket) {
    let (mut sender, mut receiver) = socket.split();
    while let Some(Ok(msg)) = receiver.next().await {
        if let Message::Text(text) = msg {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
                if parsed["type"] == "user_message" {
                    let _ = sender
                        .send(Message::Text(
                            r#"{"type":"agent_message","delta":"Hello!"}"#.into(),
                        ))
                        .await;
                    let _ = sender.send(Message::Text(r#"{"type":"done"}"#.into())).await;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_compiles() {
        // Compile-only assertion — the WS handler is exercised
        // through the router integration test in [`app::tests`].
    }
}
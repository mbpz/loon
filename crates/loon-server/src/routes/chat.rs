//! WS chat endpoint stub — full wiring lands in a later stage.

use std::sync::Arc;

use axum::extract::ws::WebSocketUpgrade;
use axum::extract::State;
use axum::response::IntoResponse;

/// `GET /v1/sessions/:id/chat` — WebSocket upgrade.
///
/// Phase 1: the WS upgrade succeeds, but no protocol is spoken.
pub async fn chat_ws(
    ws: WebSocketUpgrade,
    State(_s): State<Arc<crate::app::AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|_socket| async move {
        // Phase 1 stub: close the socket without speaking a protocol.
        // The real `user_message` / `done` flow lands in a later
        // stage once the engine is wired in.
    })
}
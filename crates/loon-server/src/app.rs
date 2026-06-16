//! Shared application state + Axum router builder.

use std::sync::Arc;

use axum::{routing::get, Router};
use loon_sdk::Server;

/// State injected into every Axum handler via
/// `axum::extract::State`.
pub struct AppState {
    pub server: Arc<Server>,
}

/// Build the root [`Router`] with the liveness routes wired in.
/// Resource routes are mounted in [`crate::routes`].
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(crate::routes::health::health))
        .route("/version", get(crate::routes::health::version))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn router_builds_with_empty_server() {
        // Phase 1 SDK builder returns a Server without external deps,
        // so we can build the router for the unit test.
        let server = Server::builder().build().await.expect("build server");
        let state = Arc::new(AppState {
            server: Arc::new(server),
        });
        let _router: Router = router(state);
    }
}
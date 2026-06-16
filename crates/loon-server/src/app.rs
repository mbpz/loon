//! Shared application state + Axum router builder.

use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};
use loon_sdk::Server;

/// State injected into every Axum handler via
/// `axum::extract::State`.
pub struct AppState {
    pub server: Arc<Server>,
}

/// Build the root [`Router`] with the liveness routes + a
/// representative set of v1 resource routes wired in.
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(crate::routes::health::health))
        .route("/version", get(crate::routes::health::version))
        .route(
            "/v1/agents",
            get(crate::routes::agents::list_agents).post(crate::routes::agents::create_agent),
        )
        .route(
            "/v1/guidelines",
            get(crate::routes::guidelines::list_guidelines)
                .post(crate::routes::guidelines::create_guideline),
        )
        .route(
            "/v1/sessions",
            get(crate::routes::sessions::list_sessions).post(crate::routes::sessions::create_session),
        )
        .route(
            "/v1/customers",
            get(crate::routes::customers::list_customers)
                .post(crate::routes::customers::create_customer),
        )
        .route("/v1/tags", get(crate::routes::tags::list_tags))
        .route(
            "/v1/relationships",
            get(crate::routes::relationships::list_relationships),
        )
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn router_builds_with_empty_server() {
        // Phase 1 SDK builder returns a Server without external
        // deps, so we can build the router for the unit test.
        let server = Server::builder().build().await.expect("build server");
        let state = Arc::new(AppState {
            server: Arc::new(server),
        });
        let _router: Router = router(state);
    }
}
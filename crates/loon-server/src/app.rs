//! Shared application state + Axum router builder.

use std::sync::Arc;

use axum::{routing::get, Router, middleware};
use loon_sdk::Server;

use crate::auth::AuthProvider;
use crate::middleware::rate_limit::{RateLimiter, RateLimitConfig, rate_limit_middleware};

/// State injected into every Axum handler via
/// `axum::extract::State`.
pub struct AppState {
    pub server: Arc<Server>,
    pub auth: Arc<dyn AuthProvider>,
    pub rate_limiter: Arc<RateLimiter>,
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
            get(crate::routes::sessions::list_sessions)
                .post(crate::routes::sessions::create_session),
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
        // ---- Phase 1 stubs (return 501 NOT_IMPLEMENTED) ----
        .route(
            "/v1/canned-responses",
            get(crate::routes::canned_responses::list_canned_responses)
                .post(crate::routes::canned_responses::create_canned_response),
        )
        .route(
            "/v1/glossary",
            get(crate::routes::glossary::list_glossary),
        )
        .route(
            "/v1/journeys",
            get(crate::routes::journeys::list_journeys)
                .post(crate::routes::journeys::create_journey),
        )
        .route(
            "/v1/observations",
            get(crate::routes::observations::list_observations)
                .post(crate::routes::observations::create_observation),
        )
        .route(
            "/v1/tools",
            get(crate::routes::tools::list_tools),
        )
        // ---- end Phase 1 stubs ----
        .route("/v1/sessions/{id}/chat", get(crate::routes::chat::chat_ws))
        .route_layer(middleware::from_fn_with_state(state.clone(), rate_limit_middleware))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::NoopAuthProvider;
    use crate::middleware::rate_limit::RateLimitConfig;

    #[tokio::test]
    async fn router_builds_with_empty_server() {
        // Phase 1 SDK builder returns a Server without external
        // deps, so we can build the router for the unit test.
        let server = Server::builder().build().await.expect("build server");
        let state = Arc::new(AppState {
            server: Arc::new(server),
            auth: Arc::new(NoopAuthProvider),
            rate_limiter: Arc::new(RateLimiter::new(RateLimitConfig::default())),
        });
        let _router: Router = router(state);
    }

    #[test]
    fn app_state_constructs() {
        // Sanity: ensure AppState fields are typed correctly without
        // needing a real Server. We don't construct AppState itself
        // here because Server is async-build; the previous test covers
        // full construction.
        let _: Arc<dyn AuthProvider> = Arc::new(NoopAuthProvider);
        let _: Arc<RateLimiter> = Arc::new(RateLimiter::new(RateLimitConfig::default()));
    }
}

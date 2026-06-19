//! Shared application state + Axum router builder.

use std::sync::Arc;

use axum::{middleware, routing::get, Router};
use loon_sdk::Server;

use crate::auth::AuthProvider;
use crate::middleware::rate_limit::{rate_limit_middleware, RateLimiter};

/// State injected into every Axum handler via
/// `axum::extract::State`.
pub struct AppState {
    pub server: Arc<Server>,
    pub auth: Arc<dyn AuthProvider>,
    pub rate_limiter: Arc<RateLimiter>,
}

/// Build the root [`Router`] with the liveness routes + every v1
/// resource route wired in. Each resource exposes the full
/// `GET / POST / GET/:id / PATCH/:id / DELETE/:id` lifecycle (the
/// few entities parlcant exposes as read-only or create-only only
/// have the relevant subset).
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(crate::routes::health::health))
        .route("/version", get(crate::routes::health::version))
        .route(
            "/v1/agents",
            get(crate::routes::agents::list_agents).post(crate::routes::agents::create_agent),
        )
        .route(
            "/v1/agents/{id}",
            get(crate::routes::agents::read_agent)
                .patch(crate::routes::agents::update_agent)
                .delete(crate::routes::agents::delete_agent),
        )
        .route(
            "/v1/guidelines",
            get(crate::routes::guidelines::list_guidelines)
                .post(crate::routes::guidelines::create_guideline),
        )
        .route(
            "/v1/guidelines/{id}",
            get(crate::routes::guidelines::read_guideline)
                .patch(crate::routes::guidelines::update_guideline)
                .delete(crate::routes::guidelines::delete_guideline),
        )
        .route(
            "/v1/journeys",
            get(crate::routes::journeys::list_journeys)
                .post(crate::routes::journeys::create_journey),
        )
        .route(
            "/v1/journeys/{id}",
            get(crate::routes::journeys::read_journey)
                .patch(crate::routes::journeys::update_journey)
                .delete(crate::routes::journeys::delete_journey),
        )
        .route(
            "/v1/tools",
            get(crate::routes::tools::list_tools).post(crate::routes::tools::create_tool),
        )
        .route(
            "/v1/tools/{id}",
            get(crate::routes::tools::read_tool)
                .patch(crate::routes::tools::update_tool)
                .delete(crate::routes::tools::delete_tool),
        )
        .route(
            "/v1/observations",
            get(crate::routes::observations::list_observations)
                .post(crate::routes::observations::create_observation),
        )
        .route(
            "/v1/observations/{id}",
            get(crate::routes::observations::read_observation)
                .patch(crate::routes::observations::update_observation)
                .delete(crate::routes::observations::delete_observation),
        )
        .route(
            "/v1/sessions",
            get(crate::routes::sessions::list_sessions)
                .post(crate::routes::sessions::create_session),
        )
        .route(
            "/v1/sessions/{id}",
            get(crate::routes::sessions::read_session)
                .patch(crate::routes::sessions::update_session)
                .delete(crate::routes::sessions::delete_session),
        )
        .route(
            "/v1/customers",
            get(crate::routes::customers::list_customers)
                .post(crate::routes::customers::create_customer),
        )
        .route(
            "/v1/customers/{id}",
            get(crate::routes::customers::read_customer)
                .patch(crate::routes::customers::update_customer)
                .delete(crate::routes::customers::delete_customer),
        )
        .route("/v1/tags", get(crate::routes::tags::list_tags))
        .route(
            "/v1/tags/{id}",
            get(crate::routes::tags::read_tag)
                .patch(crate::routes::tags::update_tag)
                .delete(crate::routes::tags::delete_tag),
        )
        .route(
            "/v1/relationships",
            get(crate::routes::relationships::list_relationships),
        )
        .route(
            "/v1/relationships/{id}",
            get(crate::routes::relationships::read_relationship)
                .delete(crate::routes::relationships::delete_relationship),
        )
        .route(
            "/v1/glossary",
            get(crate::routes::glossary::list_glossary)
                .post(crate::routes::glossary::create_glossary_entry),
        )
        .route(
            "/v1/glossary/{id}",
            get(crate::routes::glossary::read_glossary)
                .patch(crate::routes::glossary::update_glossary)
                .delete(crate::routes::glossary::delete_glossary),
        )
        .route(
            "/v1/context_variables",
            get(crate::routes::context_variables::list_context_variables)
                .post(crate::routes::context_variables::create_context_variable),
        )
        .route(
            "/v1/context_variables/{id}",
            get(crate::routes::context_variables::read_context_variable)
                .patch(crate::routes::context_variables::update_context_variable)
                .delete(crate::routes::context_variables::delete_context_variable),
        )
        .route(
            "/v1/canned_responses",
            get(crate::routes::canned_responses::list_canned_responses)
                .post(crate::routes::canned_responses::create_canned_response),
        )
        .route(
            "/v1/canned_responses/{id}",
            get(crate::routes::canned_responses::read_canned_response)
                .patch(crate::routes::canned_responses::update_canned_response)
                .delete(crate::routes::canned_responses::delete_canned_response),
        )
        .route("/v1/sessions/{id}/chat", get(crate::routes::chat::chat_ws))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            rate_limit_middleware,
        ))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            crate::middleware::auth::auth_middleware,
        ))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::NoopAuthProvider;
    use crate::middleware::rate_limit::RateLimitConfig;

    #[tokio::test]
    async fn router_builds_with_empty_server() {
        // The SDK builder returns a Server with an in-memory
        // EntityQueries graph (no external deps), so we can build
        // the router for the unit test without wiring a database.
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

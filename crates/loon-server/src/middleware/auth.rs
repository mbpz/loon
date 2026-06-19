//! Authentication middleware. Calls the configured `AuthProvider`
//! on every request and either propagates the request (with the
//! `AuthContext` injected as a request extension) or returns 401.
//!
//! **Status (2026-06-19): skeleton only — declared in `mod.rs` and
//! unit-tested here, but NOT yet wired into the router in
//! `app.rs`. Production traffic still bypasses this layer.**
//!
//! Mount this layer AFTER `with_state` and BEFORE protected routes
//! that need an authenticated principal.

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;

use crate::app::AppState;

pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response<Body>, StatusCode> {
    match state.auth.authenticate(request.headers()).await {
        Ok(ctx) => {
            // Stash the authenticated principal as a request extension
            // so downstream handlers can read it via Extension<AuthContext>.
            request.extensions_mut().insert(ctx);
            Ok(next.run(request).await)
        }
        Err(_) => Err(StatusCode::UNAUTHORIZED),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        middleware::from_fn_with_state,
        response::Response,
        routing::get,
        Router,
    };
    use std::sync::Arc;
    use tower::ServiceExt; // for `oneshot`

    /// Build a minimal `AppState` for tests. `server` and
    /// `rate_limiter` are required by the struct but not touched
    /// by `auth_middleware`, so we supply throwaway instances.
    async fn test_state(auth: Arc<dyn crate::auth::AuthProvider>) -> Arc<AppState> {
        use crate::middleware::rate_limit::{RateLimitConfig, RateLimiter};
        use loon_sdk::Server;
        Arc::new(AppState {
            server: Arc::new(Server::builder().build().await.unwrap()),
            auth,
            rate_limiter: Arc::new(RateLimiter::new(RateLimitConfig::default())),
        })
    }

    /// Drive `auth_middleware` through a one-route `Router`. We use
    /// the real axum 0.8 stack (Router + from_fn_with_state +
    /// oneshot) so the test path matches what production would
    /// look like once the layer is wired up in `app.rs`.
    async fn run(state: Arc<AppState>, req: Request<Body>) -> Response {
        Router::new()
            .route("/x", get(|| async { StatusCode::OK }))
            .layer(from_fn_with_state(state, auth_middleware))
            .oneshot(req)
            .await
            .expect("router should not fail")
    }

    fn get_request(uri: &str) -> Request<Body> {
        Request::builder()
            .method("GET")
            .uri(uri)
            .body(Body::empty())
            .unwrap()
    }

    #[tokio::test]
    async fn noop_auth_allows_request() {
        let state = test_state(Arc::new(crate::auth::NoopAuthProvider)).await;
        let res = run(state, get_request("/x")).await;
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn bearer_auth_rejects_missing_header() {
        let state = test_state(Arc::new(crate::auth::BearerTokenAuthProvider::new(vec![
            "secret".into(),
        ])))
        .await;
        let res = run(state, get_request("/x")).await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn bearer_auth_accepts_known_token() {
        use axum::http::header::AUTHORIZATION;
        let state = test_state(Arc::new(crate::auth::BearerTokenAuthProvider::new(vec![
            "secret".into(),
        ])))
        .await;
        let req = Request::builder()
            .method("GET")
            .uri("/x")
            .header(AUTHORIZATION, "Bearer secret")
            .body(Body::empty())
            .unwrap();
        let res = run(state, req).await;
        assert_eq!(res.status(), StatusCode::OK);
    }
}

//! Authentication provider trait + implementations.
//!
//! Phase 1: defines the trait, a `NoopAuthProvider` (always
//! allows), and a `BearerTokenAuthProvider` (checks against a
//! static token list). The actual `tower-http`-style middleware
//! lives in `auth_middleware`.

use async_trait::async_trait;
use axum::http::HeaderMap;
use std::collections::HashSet;

#[async_trait]
pub trait AuthProvider: Send + Sync {
    async fn authenticate(&self, headers: &HeaderMap) -> Result<AuthContext, AuthError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthContext {
    pub principal: String,
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("missing or invalid Authorization header")]
    Unauthorized,
    #[error("internal auth error: {0}")]
    Internal(String),
}

pub struct NoopAuthProvider;
#[async_trait]
impl AuthProvider for NoopAuthProvider {
    async fn authenticate(&self, _headers: &HeaderMap) -> Result<AuthContext, AuthError> {
        Ok(AuthContext {
            principal: "anonymous".into(),
        })
    }
}

pub struct BearerTokenAuthProvider {
    pub tokens: HashSet<String>,
}
impl BearerTokenAuthProvider {
    pub fn new(tokens: Vec<String>) -> Self {
        Self {
            tokens: tokens.into_iter().collect(),
        }
    }
}
#[async_trait]
impl AuthProvider for BearerTokenAuthProvider {
    async fn authenticate(&self, headers: &HeaderMap) -> Result<AuthContext, AuthError> {
        let auth = headers
            .get("authorization")
            .and_then(|h| h.to_str().ok())
            .ok_or(AuthError::Unauthorized)?;
        let token = auth
            .strip_prefix("Bearer ")
            .ok_or(AuthError::Unauthorized)?;
        if self.tokens.contains(token) {
            Ok(AuthContext {
                principal: "bearer".into(),
            })
        } else {
            Err(AuthError::Unauthorized)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;
    use axum::http::HeaderValue;

    #[tokio::test]
    async fn noop_auth_allows_anonymous() {
        let auth = NoopAuthProvider;
        let ctx = auth.authenticate(&HeaderMap::new()).await.unwrap();
        assert_eq!(ctx.principal, "anonymous");
    }

    #[tokio::test]
    async fn bearer_token_rejects_missing_header() {
        let auth = BearerTokenAuthProvider::new(vec!["secret".into()]);
        let result = auth.authenticate(&HeaderMap::new()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn bearer_token_accepts_known_token() {
        let auth = BearerTokenAuthProvider::new(vec!["secret".into()]);
        let mut headers = HeaderMap::new();
        headers.insert("authorization", HeaderValue::from_static("Bearer secret"));
        let ctx = auth.authenticate(&headers).await.unwrap();
        assert_eq!(ctx.principal, "bearer");
    }
}

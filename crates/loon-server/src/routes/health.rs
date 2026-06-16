//! Liveness / version probes.

use axum::Json;
use serde_json::{json, Value};

/// `GET /health` — returns `{"status": "ok"}`.
pub async fn health() -> Json<Value> {
    Json(json!({"status": "ok"}))
}

/// `GET /version` — returns the crate version baked at build time.
pub async fn version() -> Json<Value> {
    Json(json!({"version": env!("CARGO_PKG_VERSION")}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn health_returns_ok() {
        let v = health().await;
        assert_eq!(v.0, json!({"status": "ok"}));
    }
}
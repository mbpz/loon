//! `agents` resource routes.
//!
//! Phase 1 wiring: `list_*` returns an empty list and `create_*`
//! returns a freshly-constructed entity. Real persistence wiring
//! (storing through `s.server`'s engine) lands once the
//! `AlphaEngine` exposes high-level CRUD methods.

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use parking_lot::Mutex;
use serde::Deserialize;

use crate::api::common::{ApiError, ApiListResponse, ApiResponse};
use crate::app::AppState;
use loon_sdk::Agent;

// Phase 1 stub store keyed by AgentId. Real persistence lands later.
static AGENTS: OnceLock<Arc<Mutex<HashMap<String, serde_json::Value>>>> = OnceLock::new();
fn agent_store() -> Arc<Mutex<HashMap<String, serde_json::Value>>> {
    AGENTS
        .get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
        .clone()
}

pub async fn list_agents(
    State(_s): State<Arc<AppState>>,
) -> Result<Json<ApiListResponse<Agent>>, ApiError> {
    let store_arc = agent_store();
    let store = store_arc.lock();
    let items: Vec<Agent> = store
        .values()
        .filter_map(|v| serde_json::from_value(v.clone()).ok())
        .collect();
    Ok(Json(ApiListResponse {
        total: items.len(),
        items,
    }))
}

#[derive(Debug, Deserialize)]
pub struct CreateAgentRequest {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
}

pub async fn create_agent(
    State(_s): State<Arc<AppState>>,
    Json(req): Json<CreateAgentRequest>,
) -> Result<Json<ApiResponse<Agent>>, ApiError> {
    let agent = Agent::new(req.name, req.description.unwrap_or_default());
    let store_arc = agent_store();
    let mut store = store_arc.lock();
    let value = serde_json::to_value(&agent).map_err(|e| ApiError::Internal(e.to_string()))?;
    store.insert(agent.id.0.clone(), value);
    Ok(Json(ApiResponse { data: agent, meta: None }))
}

pub async fn read_agent(
    Path(id): Path<String>,
    State(_s): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Agent>>, ApiError> {
    let store_arc = agent_store();
    let store = store_arc.lock();
    let value = store
        .get(&id)
        .cloned()
        .ok_or_else(|| ApiError::NotFound(format!("agent {id}"), "AGENT_NOT_FOUND".into()))?;
    let agent: Agent =
        serde_json::from_value(value).map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(ApiResponse { data: agent, meta: None }))
}

#[derive(Debug, Deserialize)]
pub struct UpdateAgentRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub composition_mode: Option<loon_core::CompositionMode>,
    pub message_output_mode: Option<loon_core::MessageOutputMode>,
}

pub async fn update_agent(
    Path(id): Path<String>,
    State(_s): State<Arc<AppState>>,
    Json(req): Json<UpdateAgentRequest>,
) -> Result<Json<ApiResponse<Agent>>, ApiError> {
    let store_arc = agent_store();
    let mut store = store_arc.lock();
    let value = store
        .get(&id)
        .cloned()
        .ok_or_else(|| ApiError::NotFound(format!("agent {id}"), "AGENT_NOT_FOUND".into()))?;
    let mut agent: Agent =
        serde_json::from_value(value).map_err(|e| ApiError::Internal(e.to_string()))?;
    if let Some(n) = req.name {
        agent.name = n;
    }
    if let Some(d) = req.description {
        agent.description = d;
    }
    if let Some(m) = req.composition_mode {
        agent.composition_mode = m;
    }
    if let Some(m) = req.message_output_mode {
        agent.message_output_mode = m;
    }
    let updated = serde_json::to_value(&agent).map_err(|e| ApiError::Internal(e.to_string()))?;
    store.insert(id, updated);
    Ok(Json(ApiResponse { data: agent, meta: None }))
}

pub async fn delete_agent(
    Path(id): Path<String>,
    State(_s): State<Arc<AppState>>,
) -> Result<StatusCode, ApiError> {
    let store_arc = agent_store();
    let mut store = store_arc.lock();
    store
        .remove(&id)
        .ok_or_else(|| ApiError::NotFound(format!("agent {id}"), "AGENT_NOT_FOUND".into()))?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::NoopAuthProvider;
    use crate::middleware::rate_limit::{RateLimitConfig, RateLimiter};

    async fn test_app_state() -> Arc<AppState> {
        let server = loon_sdk::Server::builder().build().await.expect("build server");
        Arc::new(AppState {
            server: Arc::new(server),
            auth: Arc::new(NoopAuthProvider),
            rate_limiter: Arc::new(RateLimiter::new(RateLimitConfig::default())),
        })
    }

    #[tokio::test]
    async fn agent_full_lifecycle() {
        let state = test_app_state().await;

        // Create
        let created = create_agent(
            State(state.clone()),
            Json(CreateAgentRequest {
                name: "support".into(),
                description: Some("helps users".into()),
            }),
        )
        .await
        .expect("create ok");
        let id = created.data.id.0.clone();

        // Read
        let read = read_agent(Path(id.clone()), State(state.clone()))
            .await
            .expect("read ok");
        assert_eq!(read.data.name, "support");

        // Update
        let updated = update_agent(
            Path(id.clone()),
            State(state.clone()),
            Json(UpdateAgentRequest {
                name: Some("support-v2".into()),
                description: None,
                composition_mode: None,
                message_output_mode: None,
            }),
        )
        .await
        .expect("update ok");
        assert_eq!(updated.data.name, "support-v2");
        assert_eq!(updated.data.description, "helps users");

        // Delete
        let status = delete_agent(Path(id.clone()), State(state.clone()))
            .await
            .expect("delete ok");
        assert_eq!(status, StatusCode::NO_CONTENT);

        // 404 on missing
        let read_missing = read_agent(Path(id), State(state)).await;
        assert!(matches!(read_missing, Err(ApiError::NotFound(_, _))));
    }
}

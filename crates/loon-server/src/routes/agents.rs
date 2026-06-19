//! `agents` resource routes.
//!
//! Reads and writes go through `AppState.server.queries.agent_store`,
//! the same in-memory (or, in a future phase, persistent) graph
//! the engine pipeline uses. There is no separate route-only
//! HashMap.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

use crate::api::common::{ApiError, ApiListResponse, ApiResponse};
use crate::app::AppState;
use loon_core::{AgentId, AgentUpdateParams, CoreError};
use loon_sdk::Agent;

pub async fn list_agents(
    State(s): State<Arc<AppState>>,
) -> Result<Json<ApiListResponse<Agent>>, ApiError> {
    let items = s
        .server
        .queries
        .agent_store
        .list(&[])
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
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
    #[serde(default)]
    pub composition_mode: Option<loon_core::CompositionMode>,
    #[serde(default)]
    pub message_output_mode: Option<loon_core::MessageOutputMode>,
}

pub async fn create_agent(
    State(s): State<Arc<AppState>>,
    Json(req): Json<CreateAgentRequest>,
) -> Result<Json<ApiResponse<Agent>>, ApiError> {
    let mut agent = Agent::new(req.name, req.description.unwrap_or_default());
    if let Some(cm) = req.composition_mode {
        agent.composition_mode = cm;
    }
    if let Some(mom) = req.message_output_mode {
        agent.message_output_mode = mom;
    }
    let stored = s
        .server
        .queries
        .agent_store
        .create(agent)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(ApiResponse {
        data: stored,
        meta: None,
    }))
}

pub async fn read_agent(
    Path(id): Path<String>,
    State(s): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Agent>>, ApiError> {
    let aid = AgentId(id.clone());
    let agent = s
        .server
        .queries
        .agent_store
        .read(&aid)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("agent {id}"), "AGENT_NOT_FOUND".into()))?;
    Ok(Json(ApiResponse {
        data: agent,
        meta: None,
    }))
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
    State(s): State<Arc<AppState>>,
    Json(req): Json<UpdateAgentRequest>,
) -> Result<Json<ApiResponse<Agent>>, ApiError> {
    let aid = AgentId(id.clone());
    let updated = s
        .server
        .queries
        .agent_store
        .update(
            &aid,
            AgentUpdateParams {
                name: req.name,
                description: req.description,
                composition_mode: req.composition_mode,
                message_output_mode: req.message_output_mode,
                tags: None,
                metadata: None,
            },
        )
        .await
        .map_err(|e| match e {
            CoreError::NotFound(_) => {
                ApiError::NotFound(format!("agent {id}"), "AGENT_NOT_FOUND".into())
            }
            other => ApiError::Internal(other.to_string()),
        })?;
    Ok(Json(ApiResponse {
        data: updated,
        meta: None,
    }))
}

pub async fn delete_agent(
    Path(id): Path<String>,
    State(s): State<Arc<AppState>>,
) -> Result<StatusCode, ApiError> {
    let aid = AgentId(id);
    s.server
        .queries
        .agent_store
        .delete(&aid)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::NoopAuthProvider;
    use crate::middleware::rate_limit::{RateLimitConfig, RateLimiter};

    async fn test_app_state() -> Arc<AppState> {
        let server = loon_sdk::Server::builder()
            .build()
            .await
            .expect("build server");
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
                composition_mode: None,
                message_output_mode: None,
            }),
        )
        .await
        .expect("create ok");
        let id = created.data.id.0.clone();

        // List shows the agent we just created.
        let listed = list_agents(State(state.clone())).await.expect("list ok");
        assert_eq!(listed.total, 1);

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

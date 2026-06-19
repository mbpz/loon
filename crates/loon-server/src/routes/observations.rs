//! `observations` resource routes.
//!
//! Backed by `EvaluationStore` — `Observation` is stored under the
//! evaluation graph in core. Phase 5: there is no `update` method
//! on `EvaluationStore`, so the PATCH handler returns 400.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

use crate::api::common::{ApiError, ApiListResponse, ApiResponse};
use crate::app::AppState;
use loon_core::{AgentId, EvaluationId};
use loon_sdk::Observation;

#[derive(Debug, Deserialize)]
pub struct ListObservationsQuery {
    pub agent_id: Option<String>,
}

pub async fn list_observations(
    Query(q): Query<ListObservationsQuery>,
    State(s): State<Arc<AppState>>,
) -> Result<Json<ApiListResponse<Observation>>, ApiError> {
    let items = match q.agent_id {
        Some(id) => s
            .server
            .queries
            .evaluation_store
            .list(&AgentId(id))
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?,
        None => Vec::new(),
    };
    Ok(Json(ApiListResponse {
        total: items.len(),
        items,
    }))
}

#[derive(Debug, Deserialize)]
pub struct CreateObservationRequest {
    pub agent_id: AgentId,
    #[serde(default)]
    pub condition: String,
    #[serde(default)]
    pub tools: Vec<loon_core::ToolId>,
}

pub async fn create_observation(
    State(s): State<Arc<AppState>>,
    Json(req): Json<CreateObservationRequest>,
) -> Result<Json<ApiResponse<Observation>>, ApiError> {
    let o = Observation::new(req.condition, req.tools, &req.agent_id);
    let stored = s
        .server
        .queries
        .evaluation_store
        .create(o)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(ApiResponse {
        data: stored,
        meta: None,
    }))
}

pub async fn read_observation(
    Path(id): Path<String>,
    State(s): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Observation>>, ApiError> {
    let oid = EvaluationId(id.clone());
    let o = s
        .server
        .queries
        .evaluation_store
        .read(&oid)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| {
            ApiError::NotFound(format!("observation {id}"), "OBSERVATION_NOT_FOUND".into())
        })?;
    Ok(Json(ApiResponse {
        data: o,
        meta: None,
    }))
}

#[derive(Debug, Deserialize)]
pub struct UpdateObservationRequest {
    pub condition: Option<String>,
    pub tools: Option<Vec<String>>,
    pub enabled: Option<bool>,
}

pub async fn update_observation(
    Path(id): Path<String>,
    State(s): State<Arc<AppState>>,
    Json(req): Json<UpdateObservationRequest>,
) -> Result<Json<ApiResponse<Observation>>, ApiError> {
    let eid = loon_core::EvaluationId(id);
    let tools = req
        .tools
        .map(|ts| ts.into_iter().map(loon_core::ToolId).collect());
    let updated = s
        .server
        .queries
        .evaluation_store
        .update(
            &eid,
            loon_core::ObservationUpdateParams {
                condition: req.condition,
                tools,
                enabled: req.enabled,
            },
        )
        .await
        .map_err(|e| match e {
            loon_core::CoreError::NotFound(uid) => ApiError::NotFound(
                format!("observation {}", uid.0),
                "OBSERVATION_NOT_FOUND".into(),
            ),
            other => ApiError::Internal(other.to_string()),
        })?;
    Ok(Json(ApiResponse {
        data: updated,
        meta: None,
    }))
}

pub async fn delete_observation(
    Path(id): Path<String>,
    State(s): State<Arc<AppState>>,
) -> Result<StatusCode, ApiError> {
    let oid = EvaluationId(id);
    s.server
        .queries
        .evaluation_store
        .delete(&oid)
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
    async fn observation_create_read_delete() {
        let state = test_app_state().await;
        let agent_id = AgentId::new();

        let created = create_observation(
            State(state.clone()),
            Json(CreateObservationRequest {
                agent_id: agent_id.clone(),
                condition: "spike in errors".into(),
                tools: vec![],
            }),
        )
        .await
        .expect("create ok");
        let id = created.data.id.0.clone();

        let read = read_observation(Path(id.clone()), State(state.clone()))
            .await
            .expect("read ok");
        assert_eq!(read.data.condition, "spike in errors");

        let upd = update_observation(
            Path(id.clone()),
            State(state.clone()),
            Json(UpdateObservationRequest {
                condition: Some("updated".into()),
                tools: None,
                enabled: Some(false),
            }),
        )
        .await
        .expect("update");
        assert_eq!(upd.0.data.condition, "updated");
        assert!(!upd.0.data.enabled);

        let status = delete_observation(Path(id.clone()), State(state.clone()))
            .await
            .expect("delete ok");
        assert_eq!(status, StatusCode::NO_CONTENT);

        let read_missing = read_observation(Path(id), State(state)).await;
        assert!(matches!(read_missing, Err(ApiError::NotFound(_, _))));
    }
}

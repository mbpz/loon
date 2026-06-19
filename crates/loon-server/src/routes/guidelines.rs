//! `guidelines` resource routes.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

use crate::api::common::{ApiError, ApiListResponse, ApiResponse};
use crate::app::AppState;
use loon_core::{AgentId, CoreError, GuidelineId, GuidelineUpdateParams};
use loon_sdk::{Guideline, GuidelineContent};

#[derive(Debug, Deserialize)]
pub struct ListGuidelinesQuery {
    pub agent_id: Option<String>,
}

pub async fn list_guidelines(
    Query(q): Query<ListGuidelinesQuery>,
    State(s): State<Arc<AppState>>,
) -> Result<Json<ApiListResponse<Guideline>>, ApiError> {
    // `GuidelineStore::list` is agent-scoped. Without an
    // `?agent_id=...` query the route returns an empty list rather
    // than scanning every agent.
    let items = match q.agent_id {
        Some(id) => s
            .server
            .queries
            .guideline_store
            .list(&AgentId(id), &[])
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
pub struct CreateGuidelineRequest {
    pub agent_id: AgentId,
    #[serde(default)]
    pub condition: String,
    #[serde(default)]
    pub action: String,
    #[serde(default)]
    pub description: Option<String>,
}

pub async fn create_guideline(
    State(s): State<Arc<AppState>>,
    Json(req): Json<CreateGuidelineRequest>,
) -> Result<Json<ApiResponse<Guideline>>, ApiError> {
    let g = Guideline::new(
        GuidelineContent {
            condition: req.condition,
            action: req.action,
            description: req.description,
        },
        &req.agent_id,
        true,
        0,
    );
    let stored = s
        .server
        .queries
        .guideline_store
        .create(g)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(ApiResponse {
        data: stored,
        meta: None,
    }))
}

pub async fn read_guideline(
    Path(id): Path<String>,
    State(s): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Guideline>>, ApiError> {
    let gid = GuidelineId(id.clone());
    let g = s
        .server
        .queries
        .guideline_store
        .read(&gid)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| {
            ApiError::NotFound(format!("guideline {id}"), "GUIDELINE_NOT_FOUND".into())
        })?;
    Ok(Json(ApiResponse {
        data: g,
        meta: None,
    }))
}

#[derive(Debug, Deserialize)]
pub struct UpdateGuidelineRequest {
    pub condition: Option<String>,
    pub action: Option<String>,
    pub enabled: Option<bool>,
}

pub async fn update_guideline(
    Path(id): Path<String>,
    State(s): State<Arc<AppState>>,
    Json(req): Json<UpdateGuidelineRequest>,
) -> Result<Json<ApiResponse<Guideline>>, ApiError> {
    let gid = GuidelineId(id.clone());
    let updated = s
        .server
        .queries
        .guideline_store
        .update(
            &gid,
            GuidelineUpdateParams {
                condition: req.condition,
                action: req.action,
                enabled: req.enabled,
            },
        )
        .await
        .map_err(|e| match e {
            CoreError::NotFound(_) => {
                ApiError::NotFound(format!("guideline {id}"), "GUIDELINE_NOT_FOUND".into())
            }
            other => ApiError::Internal(other.to_string()),
        })?;
    Ok(Json(ApiResponse {
        data: updated,
        meta: None,
    }))
}

pub async fn delete_guideline(
    Path(id): Path<String>,
    State(s): State<Arc<AppState>>,
) -> Result<StatusCode, ApiError> {
    let gid = GuidelineId(id);
    s.server
        .queries
        .guideline_store
        .delete(&gid)
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
    async fn guideline_full_lifecycle() {
        let state = test_app_state().await;
        let agent_id = AgentId::new();

        let created = create_guideline(
            State(state.clone()),
            Json(CreateGuidelineRequest {
                agent_id: agent_id.clone(),
                condition: "user asks for help".into(),
                action: "respond politely".into(),
                description: None,
            }),
        )
        .await
        .expect("create ok");
        let id = created.data.id.0.clone();

        let listed = list_guidelines(
            Query(ListGuidelinesQuery {
                agent_id: Some(agent_id.0.clone()),
            }),
            State(state.clone()),
        )
        .await
        .expect("list ok");
        assert_eq!(listed.total, 1);

        let read = read_guideline(Path(id.clone()), State(state.clone()))
            .await
            .expect("read ok");
        assert_eq!(read.data.content.condition, "user asks for help");

        let updated = update_guideline(
            Path(id.clone()),
            State(state.clone()),
            Json(UpdateGuidelineRequest {
                condition: Some("user is angry".into()),
                action: None,
                enabled: Some(false),
            }),
        )
        .await
        .expect("update ok");
        assert_eq!(updated.data.content.condition, "user is angry");
        assert!(!updated.data.enabled);

        let status = delete_guideline(Path(id.clone()), State(state.clone()))
            .await
            .expect("delete ok");
        assert_eq!(status, StatusCode::NO_CONTENT);

        let read_missing = read_guideline(Path(id), State(state)).await;
        assert!(matches!(read_missing, Err(ApiError::NotFound(_, _))));
    }
}

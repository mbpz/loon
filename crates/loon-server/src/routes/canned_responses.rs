//! `canned_responses` resource routes.

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::Deserialize;

use crate::api::common::{ApiError, ApiListResponse, ApiResponse};
use crate::app::AppState;
use loon_core::{AgentId, CannedResponseId, CannedResponseUpdateParams, CoreError};
use loon_sdk::CannedResponse;

#[derive(Debug, Deserialize)]
pub struct ListCannedResponsesQuery {
    pub agent_id: Option<String>,
}

pub async fn list_canned_responses(
    Query(q): Query<ListCannedResponsesQuery>,
    State(s): State<Arc<AppState>>,
) -> Result<Json<ApiListResponse<CannedResponse>>, ApiError> {
    let items = match q.agent_id {
        Some(id) => s
            .server
            .queries
            .canned_response_store
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
pub struct CreateCannedResponseRequest {
    pub agent_id: AgentId,
    pub value: String,
}

pub async fn create_canned_response(
    State(s): State<Arc<AppState>>,
    Json(req): Json<CreateCannedResponseRequest>,
) -> Result<Json<ApiResponse<CannedResponse>>, ApiError> {
    let c = CannedResponse::new(&req.agent_id, req.value);
    let stored = s
        .server
        .queries
        .canned_response_store
        .create(c)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(ApiResponse {
        data: stored,
        meta: None,
    }))
}

pub async fn read_canned_response(
    Path(id): Path<String>,
    State(s): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<CannedResponse>>, ApiError> {
    let cid = CannedResponseId(id.clone());
    let c = s
        .server
        .queries
        .canned_response_store
        .read(&cid)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| {
            ApiError::NotFound(
                format!("canned_response {id}"),
                "CANNED_RESPONSE_NOT_FOUND".into(),
            )
        })?;
    Ok(Json(ApiResponse {
        data: c,
        meta: None,
    }))
}

#[derive(Debug, Deserialize)]
pub struct UpdateCannedResponseRequest {
    pub value: Option<String>,
    pub matchers: Option<Vec<String>>,
}

pub async fn update_canned_response(
    Path(id): Path<String>,
    State(s): State<Arc<AppState>>,
    Json(req): Json<UpdateCannedResponseRequest>,
) -> Result<Json<ApiResponse<CannedResponse>>, ApiError> {
    let cid = CannedResponseId(id.clone());
    let updated = s
        .server
        .queries
        .canned_response_store
        .update(
            &cid,
            CannedResponseUpdateParams {
                value: req.value,
                matchers: req.matchers,
            },
        )
        .await
        .map_err(|e| match e {
            CoreError::NotFound(_) => ApiError::NotFound(
                format!("canned_response {id}"),
                "CANNED_RESPONSE_NOT_FOUND".into(),
            ),
            other => ApiError::Internal(other.to_string()),
        })?;
    Ok(Json(ApiResponse {
        data: updated,
        meta: None,
    }))
}

pub async fn delete_canned_response(
    Path(id): Path<String>,
    State(s): State<Arc<AppState>>,
) -> Result<StatusCode, ApiError> {
    let cid = CannedResponseId(id);
    s.server
        .queries
        .canned_response_store
        .delete(&cid)
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
    async fn canned_response_full_lifecycle() {
        let state = test_app_state().await;
        let agent_id = AgentId::new();

        let created = create_canned_response(
            State(state.clone()),
            Json(CreateCannedResponseRequest {
                agent_id: agent_id.clone(),
                value: "Hello!".into(),
            }),
        )
        .await
        .expect("create ok");
        let id = created.data.id.0.clone();

        let read = read_canned_response(Path(id.clone()), State(state.clone()))
            .await
            .expect("read ok");
        assert_eq!(read.data.value, "Hello!");

        let updated = update_canned_response(
            Path(id.clone()),
            State(state.clone()),
            Json(UpdateCannedResponseRequest {
                value: Some("Hi!".into()),
                matchers: None,
            }),
        )
        .await
        .expect("update ok");
        assert_eq!(updated.data.value, "Hi!");

        let status = delete_canned_response(Path(id.clone()), State(state.clone()))
            .await
            .expect("delete ok");
        assert_eq!(status, StatusCode::NO_CONTENT);

        let read_missing = read_canned_response(Path(id), State(state)).await;
        assert!(matches!(read_missing, Err(ApiError::NotFound(_, _))));
    }
}

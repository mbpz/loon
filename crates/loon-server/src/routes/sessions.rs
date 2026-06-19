//! `sessions` resource routes.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

use crate::api::common::{ApiError, ApiListResponse, ApiResponse};
use crate::app::AppState;
use loon_core::{AgentId, CoreError, CustomerId, SessionId, SessionUpdateParams};
use loon_sdk::Session;

#[derive(Debug, Deserialize)]
pub struct ListSessionsQuery {
    pub agent_id: Option<String>,
    pub customer_id: Option<String>,
}

pub async fn list_sessions(
    Query(q): Query<ListSessionsQuery>,
    State(s): State<Arc<AppState>>,
) -> Result<Json<ApiListResponse<Session>>, ApiError> {
    let agent_id = q.agent_id.map(AgentId);
    let customer_id = q.customer_id.map(CustomerId);
    let items = s
        .server
        .queries
        .session_store
        .list(agent_id.as_ref(), customer_id.as_ref())
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(ApiListResponse {
        total: items.len(),
        items,
    }))
}

#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    pub agent_id: AgentId,
    #[serde(default)]
    pub customer_id: Option<CustomerId>,
    #[serde(default)]
    pub title: Option<String>,
}

pub async fn create_session(
    State(s): State<Arc<AppState>>,
    Json(req): Json<CreateSessionRequest>,
) -> Result<Json<ApiResponse<Session>>, ApiError> {
    let mut session = Session::new(&req.agent_id);
    session.customer_id = req.customer_id;
    session.title = req.title;
    let stored = s
        .server
        .queries
        .session_store
        .create(session)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(ApiResponse {
        data: stored,
        meta: None,
    }))
}

pub async fn read_session(
    Path(id): Path<String>,
    State(s): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Session>>, ApiError> {
    let sid = SessionId(id.clone());
    let session = s
        .server
        .queries
        .session_store
        .read(&sid)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("session {id}"), "SESSION_NOT_FOUND".into()))?;
    Ok(Json(ApiResponse {
        data: session,
        meta: None,
    }))
}

#[derive(Debug, Deserialize)]
pub struct UpdateSessionRequest {
    pub title: Option<String>,
    pub mode: Option<loon_core::SessionMode>,
}

pub async fn update_session(
    Path(id): Path<String>,
    State(s): State<Arc<AppState>>,
    Json(req): Json<UpdateSessionRequest>,
) -> Result<Json<ApiResponse<Session>>, ApiError> {
    let sid = SessionId(id.clone());
    let updated = s
        .server
        .queries
        .session_store
        .update(
            &sid,
            SessionUpdateParams {
                title: req.title,
                mode: req.mode,
                labels: None,
            },
        )
        .await
        .map_err(|e| match e {
            CoreError::NotFound(_) => {
                ApiError::NotFound(format!("session {id}"), "SESSION_NOT_FOUND".into())
            }
            other => ApiError::Internal(other.to_string()),
        })?;
    Ok(Json(ApiResponse {
        data: updated,
        meta: None,
    }))
}

pub async fn delete_session(
    Path(id): Path<String>,
    State(s): State<Arc<AppState>>,
) -> Result<StatusCode, ApiError> {
    let sid = SessionId(id);
    s.server
        .queries
        .session_store
        .delete(&sid)
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
    async fn session_full_lifecycle() {
        let state = test_app_state().await;
        let agent_id = AgentId::new();

        let created = create_session(
            State(state.clone()),
            Json(CreateSessionRequest {
                agent_id: agent_id.clone(),
                customer_id: None,
                title: Some("first chat".into()),
            }),
        )
        .await
        .expect("create ok");
        let id = created.data.id.0.clone();

        let listed = list_sessions(
            Query(ListSessionsQuery {
                agent_id: Some(agent_id.0.clone()),
                customer_id: None,
            }),
            State(state.clone()),
        )
        .await
        .expect("list ok");
        assert_eq!(listed.total, 1);

        let read = read_session(Path(id.clone()), State(state.clone()))
            .await
            .expect("read ok");
        assert_eq!(read.data.title.as_deref(), Some("first chat"));

        let updated = update_session(
            Path(id.clone()),
            State(state.clone()),
            Json(UpdateSessionRequest {
                title: Some("renamed".into()),
                mode: None,
            }),
        )
        .await
        .expect("update ok");
        assert_eq!(updated.data.title.as_deref(), Some("renamed"));

        let status = delete_session(Path(id.clone()), State(state.clone()))
            .await
            .expect("delete ok");
        assert_eq!(status, StatusCode::NO_CONTENT);

        let read_missing = read_session(Path(id), State(state)).await;
        assert!(matches!(read_missing, Err(ApiError::NotFound(_, _))));
    }
}

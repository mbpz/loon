//! Context-variable routes. Phase 1 supports full CRUD via
//! `ContextVariableStore` with the standard lifecycle: list, create,
//! read, update, delete. The `upsert_value` sub-path lets callers
//! set variable values without touching the descriptor.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

use crate::api::common::{ApiError, ApiListResponse, ApiResponse};
use crate::app::AppState;
use loon_core::{ContextVariable, ContextVariableId, ContextVariableUpdateParams, JsonValue};

/// Optional filter for listing context variables.
#[derive(Deserialize, Default)]
pub struct ListCtxVarsQuery {
    pub agent_id: Option<String>,
}

pub async fn list_context_variables(
    Query(q): Query<ListCtxVarsQuery>,
    State(s): State<Arc<AppState>>,
) -> Result<Json<ApiListResponse<ContextVariable>>, ApiError> {
    if let Some(aid) = &q.agent_id {
        let agent_id = loon_core::AgentId(aid.clone());
        let items = s
            .server
            .queries
            .context_variable_store
            .list(&agent_id)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;
        let total = items.len();
        Ok(Json(ApiListResponse { items, total }))
    } else {
        Ok(Json(ApiListResponse {
            items: vec![],
            total: 0,
        }))
    }
}

pub async fn create_context_variable(
    State(s): State<Arc<AppState>>,
    Json(req): Json<CreateCtxVarRequest>,
) -> Result<Json<ApiResponse<ContextVariable>>, ApiError> {
    let var = ContextVariable {
        id: ContextVariableId::new(),
        agent_id: loon_core::AgentId(req.agent_id),
        key: req.key,
        freshness_rules: vec![],
        tags: vec![],
        creation_utc: chrono::Utc::now(),
    };
    let stored = s
        .server
        .queries
        .context_variable_store
        .create(var)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(ApiResponse {
        data: stored,
        meta: None,
    }))
}

pub async fn read_context_variable(
    Path(id): Path<String>,
    State(s): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<ContextVariable>>, ApiError> {
    let vid = ContextVariableId(id);
    let var = s
        .server
        .queries
        .context_variable_store
        .read(&vid)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| {
            ApiError::NotFound(
                "context_variable not found".to_string(),
                "CONTEXT_VARIABLE_NOT_FOUND".into(),
            )
        })?;
    Ok(Json(ApiResponse {
        data: var,
        meta: None,
    }))
}

pub async fn update_context_variable(
    Path(id): Path<String>,
    State(s): State<Arc<AppState>>,
    Json(req): Json<UpdateCtxVarRequest>,
) -> Result<Json<ApiResponse<ContextVariable>>, ApiError> {
    let vid = ContextVariableId(id);
    let updated = s
        .server
        .queries
        .context_variable_store
        .update(
            &vid,
            ContextVariableUpdateParams {
                key: req.key,
                data: req.data,
            },
        )
        .await
        .map_err(|e| match e {
            loon_core::CoreError::NotFound(uid) => ApiError::NotFound(
                format!("context_variable {}", uid.0),
                "CONTEXT_VARIABLE_NOT_FOUND".into(),
            ),
            other => ApiError::Internal(other.to_string()),
        })?;
    Ok(Json(ApiResponse {
        data: updated,
        meta: None,
    }))
}

pub async fn delete_context_variable(
    Path(id): Path<String>,
    State(s): State<Arc<AppState>>,
) -> Result<StatusCode, ApiError> {
    let vid = ContextVariableId(id);
    s.server
        .queries
        .context_variable_store
        .delete(&vid)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct CreateCtxVarRequest {
    pub agent_id: String,
    pub key: String,
}

#[derive(Deserialize, Default)]
pub struct UpdateCtxVarRequest {
    pub key: Option<String>,
    pub data: Option<JsonValue>,
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
    async fn create_and_read_round_trip() {
        let state = test_app_state().await;

        let created = create_context_variable(
            State(state.clone()),
            Json(CreateCtxVarRequest {
                agent_id: "a".into(),
                key: "greeting".into(),
            }),
        )
        .await
        .expect("create ok");
        let id = created.data.id.0.clone();

        let read = read_context_variable(Path(id.clone()), State(state.clone()))
            .await
            .expect("read ok");
        assert_eq!(read.data.key, "greeting");

        let updated = update_context_variable(
            Path(id.clone()),
            State(state.clone()),
            Json(UpdateCtxVarRequest {
                key: Some("farewell".into()),
                data: None,
            }),
        )
        .await
        .expect("update ok");
        assert_eq!(updated.data.key, "farewell");

        let status = delete_context_variable(Path(id.clone()), State(state.clone()))
            .await
            .expect("delete ok");
        assert_eq!(status, StatusCode::NO_CONTENT);

        let read_missing = read_context_variable(Path(id), State(state)).await;
        assert!(matches!(read_missing, Err(ApiError::NotFound(_, _))));
    }
}

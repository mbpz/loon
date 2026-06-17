//! `tools` resource routes.

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::Deserialize;

use crate::api::common::{ApiError, ApiListResponse, ApiResponse};
use crate::app::AppState;
use loon_core::{AgentId, ToolId};
use loon_sdk::Tool;

#[derive(Debug, Deserialize)]
pub struct ListToolsQuery {
    pub agent_id: Option<String>,
}

pub async fn list_tools(
    Query(q): Query<ListToolsQuery>,
    State(s): State<Arc<AppState>>,
) -> Result<Json<ApiListResponse<Tool>>, ApiError> {
    let items = match q.agent_id {
        Some(id) => s
            .server
            .queries
            .tool_store
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
pub struct CreateToolRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
}

pub async fn create_tool(
    State(s): State<Arc<AppState>>,
    Json(req): Json<CreateToolRequest>,
) -> Result<Json<ApiResponse<Tool>>, ApiError> {
    let t = Tool {
        id: ToolId::new(),
        name: req.name,
        description: req.description,
        parameters_schema: serde_json::json!({}),
        kind: loon_core::ToolKind::Local,
        creation_utc: chrono::Utc::now(),
    };
    let stored = s
        .server
        .queries
        .tool_store
        .create(t)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(ApiResponse {
        data: stored,
        meta: None,
    }))
}

pub async fn read_tool(
    Path(id): Path<String>,
    State(s): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Tool>>, ApiError> {
    let tid = ToolId(id.clone());
    let t = s
        .server
        .queries
        .tool_store
        .read(&tid)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("tool {id}"), "TOOL_NOT_FOUND".into()))?;
    Ok(Json(ApiResponse {
        data: t,
        meta: None,
    }))
}

#[derive(Debug, Deserialize)]
pub struct UpdateToolRequest {
    #[serde(default)]
    pub _unused: Option<String>,
}

/// `ToolStore` does not currently expose `update`, so the route
/// returns 400 to make the limitation visible to clients. Once the
/// store grows an `update` method this route can be filled in.
pub async fn update_tool(
    Path(_id): Path<String>,
    State(_s): State<Arc<AppState>>,
    Json(_req): Json<UpdateToolRequest>,
) -> Result<Json<ApiResponse<Tool>>, ApiError> {
    Err(ApiError::InvalidArgument(
        "tools do not support update".into(),
        "TOOL_UPDATE_NOT_SUPPORTED".into(),
    ))
}

pub async fn delete_tool(
    Path(id): Path<String>,
    State(s): State<Arc<AppState>>,
) -> Result<StatusCode, ApiError> {
    let tid = ToolId(id);
    s.server
        .queries
        .tool_store
        .delete(&tid)
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
    async fn tool_create_read_delete() {
        let state = test_app_state().await;

        let created = create_tool(
            State(state.clone()),
            Json(CreateToolRequest {
                name: "echo".into(),
                description: "say it back".into(),
            }),
        )
        .await
        .expect("create ok");
        let id = created.data.id.0.clone();

        let read = read_tool(Path(id.clone()), State(state.clone()))
            .await
            .expect("read ok");
        assert_eq!(read.data.name, "echo");

        // update is not supported by ToolStore — must surface as 400.
        let upd = update_tool(
            Path(id.clone()),
            State(state.clone()),
            Json(UpdateToolRequest { _unused: None }),
        )
        .await;
        assert!(matches!(upd, Err(ApiError::InvalidArgument(_, _))));

        let status = delete_tool(Path(id.clone()), State(state.clone()))
            .await
            .expect("delete ok");
        assert_eq!(status, StatusCode::NO_CONTENT);

        let read_missing = read_tool(Path(id), State(state)).await;
        assert!(matches!(read_missing, Err(ApiError::NotFound(_, _))));
    }
}

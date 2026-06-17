//! `tools` resource routes.

use std::sync::Arc;

use axum::{extract::State, Json};
use serde::Deserialize;

use crate::api::common::{ApiError, ApiListResponse, ApiResponse};
use crate::app::AppState;
use loon_sdk::Tool;

pub async fn list_tools(
    State(_s): State<Arc<AppState>>,
) -> Result<Json<ApiListResponse<Tool>>, ApiError> {
    Ok(Json(ApiListResponse { items: vec![], total: 0 }))
}

#[derive(Debug, Deserialize)]
pub struct CreateToolRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
}

pub async fn create_tool(
    State(_s): State<Arc<AppState>>,
    Json(req): Json<CreateToolRequest>,
) -> Result<Json<ApiResponse<Tool>>, ApiError> {
    let t = Tool {
        id: loon_core::ToolId::new(),
        name: req.name,
        description: req.description,
        parameters_schema: serde_json::json!({}),
        kind: loon_core::ToolKind::Local,
        creation_utc: chrono::Utc::now(),
    };
    Ok(Json(ApiResponse { data: t, meta: None }))
}

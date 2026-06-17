//! `tools` resource routes.

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
use loon_sdk::Tool;

// Phase 1 stub store keyed by ToolId. Real persistence lands later.
static TOOLS: OnceLock<Arc<Mutex<HashMap<String, serde_json::Value>>>> = OnceLock::new();
fn tool_store() -> Arc<Mutex<HashMap<String, serde_json::Value>>> {
    TOOLS
        .get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
        .clone()
}

pub async fn list_tools(
    State(_s): State<Arc<AppState>>,
) -> Result<Json<ApiListResponse<Tool>>, ApiError> {
    let store_arc = tool_store();
    let store = store_arc.lock();
    let items: Vec<Tool> = store
        .values()
        .filter_map(|v| serde_json::from_value(v.clone()).ok())
        .collect();
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
    let store_arc = tool_store();
    let mut store = store_arc.lock();
    let value = serde_json::to_value(&t).map_err(|e| ApiError::Internal(e.to_string()))?;
    store.insert(t.id.0.clone(), value);
    Ok(Json(ApiResponse { data: t, meta: None }))
}

pub async fn read_tool(
    Path(id): Path<String>,
    State(_s): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Tool>>, ApiError> {
    let store_arc = tool_store();
    let store = store_arc.lock();
    let value = store
        .get(&id)
        .cloned()
        .ok_or_else(|| ApiError::NotFound(format!("tool {id}"), "TOOL_NOT_FOUND".into()))?;
    let t: Tool =
        serde_json::from_value(value).map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(ApiResponse { data: t, meta: None }))
}

#[derive(Debug, Deserialize)]
pub struct UpdateToolRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

pub async fn update_tool(
    Path(id): Path<String>,
    State(_s): State<Arc<AppState>>,
    Json(req): Json<UpdateToolRequest>,
) -> Result<Json<ApiResponse<Tool>>, ApiError> {
    let store_arc = tool_store();
    let mut store = store_arc.lock();
    let value = store
        .get(&id)
        .cloned()
        .ok_or_else(|| ApiError::NotFound(format!("tool {id}"), "TOOL_NOT_FOUND".into()))?;
    let mut t: Tool =
        serde_json::from_value(value).map_err(|e| ApiError::Internal(e.to_string()))?;
    if let Some(n) = req.name {
        t.name = n;
    }
    if let Some(d) = req.description {
        t.description = d;
    }
    let updated = serde_json::to_value(&t).map_err(|e| ApiError::Internal(e.to_string()))?;
    store.insert(id, updated);
    Ok(Json(ApiResponse { data: t, meta: None }))
}

pub async fn delete_tool(
    Path(id): Path<String>,
    State(_s): State<Arc<AppState>>,
) -> Result<StatusCode, ApiError> {
    let store_arc = tool_store();
    let mut store = store_arc.lock();
    store
        .remove(&id)
        .ok_or_else(|| ApiError::NotFound(format!("tool {id}"), "TOOL_NOT_FOUND".into()))?;
    Ok(StatusCode::NO_CONTENT)
}

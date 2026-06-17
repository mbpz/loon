//! `guidelines` resource routes.

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
use loon_sdk::{Guideline, GuidelineContent};

// Phase 1 stub store keyed by GuidelineId. Real persistence lands later.
static GUIDELINES: OnceLock<Arc<Mutex<HashMap<String, serde_json::Value>>>> = OnceLock::new();
fn guideline_store() -> Arc<Mutex<HashMap<String, serde_json::Value>>> {
    GUIDELINES
        .get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
        .clone()
}

pub async fn list_guidelines(
    State(_s): State<Arc<AppState>>,
) -> Result<Json<ApiListResponse<Guideline>>, ApiError> {
    let store_arc = guideline_store();
    let store = store_arc.lock();
    let items: Vec<Guideline> = store
        .values()
        .filter_map(|v| serde_json::from_value(v.clone()).ok())
        .collect();
    Ok(Json(ApiListResponse {
        total: items.len(),
        items,
    }))
}

#[derive(Debug, Deserialize)]
pub struct CreateGuidelineRequest {
    pub agent_id: loon_core::AgentId,
    #[serde(default)]
    pub condition: String,
    #[serde(default)]
    pub action: String,
    #[serde(default)]
    pub description: Option<String>,
}

pub async fn create_guideline(
    State(_s): State<Arc<AppState>>,
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
    let store_arc = guideline_store();
    let mut store = store_arc.lock();
    let value = serde_json::to_value(&g).map_err(|e| ApiError::Internal(e.to_string()))?;
    store.insert(g.id.0.clone(), value);
    Ok(Json(ApiResponse { data: g, meta: None }))
}

pub async fn read_guideline(
    Path(id): Path<String>,
    State(_s): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Guideline>>, ApiError> {
    let store_arc = guideline_store();
    let store = store_arc.lock();
    let value = store
        .get(&id)
        .cloned()
        .ok_or_else(|| ApiError::NotFound(format!("guideline {id}"), "GUIDELINE_NOT_FOUND".into()))?;
    let g: Guideline =
        serde_json::from_value(value).map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(ApiResponse { data: g, meta: None }))
}

#[derive(Debug, Deserialize)]
pub struct UpdateGuidelineRequest {
    pub condition: Option<String>,
    pub action: Option<String>,
    pub description: Option<String>,
    pub enabled: Option<bool>,
}

pub async fn update_guideline(
    Path(id): Path<String>,
    State(_s): State<Arc<AppState>>,
    Json(req): Json<UpdateGuidelineRequest>,
) -> Result<Json<ApiResponse<Guideline>>, ApiError> {
    let store_arc = guideline_store();
    let mut store = store_arc.lock();
    let value = store
        .get(&id)
        .cloned()
        .ok_or_else(|| ApiError::NotFound(format!("guideline {id}"), "GUIDELINE_NOT_FOUND".into()))?;
    let mut g: Guideline =
        serde_json::from_value(value).map_err(|e| ApiError::Internal(e.to_string()))?;
    if let Some(c) = req.condition {
        g.content.condition = c;
    }
    if let Some(a) = req.action {
        g.content.action = a;
    }
    if let Some(d) = req.description {
        g.content.description = Some(d);
    }
    if let Some(e) = req.enabled {
        g.enabled = e;
    }
    let updated = serde_json::to_value(&g).map_err(|e| ApiError::Internal(e.to_string()))?;
    store.insert(id, updated);
    Ok(Json(ApiResponse { data: g, meta: None }))
}

pub async fn delete_guideline(
    Path(id): Path<String>,
    State(_s): State<Arc<AppState>>,
) -> Result<StatusCode, ApiError> {
    let store_arc = guideline_store();
    let mut store = store_arc.lock();
    store
        .remove(&id)
        .ok_or_else(|| ApiError::NotFound(format!("guideline {id}"), "GUIDELINE_NOT_FOUND".into()))?;
    Ok(StatusCode::NO_CONTENT)
}

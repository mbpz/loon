//! `canned_responses` resource routes.

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
use loon_sdk::CannedResponse;

// Phase 1 stub store keyed by CannedResponseId. Real persistence lands later.
static CANNED_RESPONSES: OnceLock<Arc<Mutex<HashMap<String, serde_json::Value>>>> = OnceLock::new();
fn canned_response_store() -> Arc<Mutex<HashMap<String, serde_json::Value>>> {
    CANNED_RESPONSES
        .get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
        .clone()
}

pub async fn list_canned_responses(
    State(_s): State<Arc<AppState>>,
) -> Result<Json<ApiListResponse<CannedResponse>>, ApiError> {
    let store_arc = canned_response_store();
    let store = store_arc.lock();
    let items: Vec<CannedResponse> = store
        .values()
        .filter_map(|v| serde_json::from_value(v.clone()).ok())
        .collect();
    Ok(Json(ApiListResponse {
        total: items.len(),
        items,
    }))
}

#[derive(Debug, Deserialize)]
pub struct CreateCannedResponseRequest {
    pub agent_id: loon_core::AgentId,
    pub value: String,
}

pub async fn create_canned_response(
    State(_s): State<Arc<AppState>>,
    Json(req): Json<CreateCannedResponseRequest>,
) -> Result<Json<ApiResponse<CannedResponse>>, ApiError> {
    let c = CannedResponse::new(&req.agent_id, req.value);
    let store_arc = canned_response_store();
    let mut store = store_arc.lock();
    let value = serde_json::to_value(&c).map_err(|e| ApiError::Internal(e.to_string()))?;
    store.insert(c.id.0.clone(), value);
    Ok(Json(ApiResponse { data: c, meta: None }))
}

pub async fn read_canned_response(
    Path(id): Path<String>,
    State(_s): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<CannedResponse>>, ApiError> {
    let store_arc = canned_response_store();
    let store = store_arc.lock();
    let value = store.get(&id).cloned().ok_or_else(|| {
        ApiError::NotFound(
            format!("canned_response {id}"),
            "CANNED_RESPONSE_NOT_FOUND".into(),
        )
    })?;
    let c: CannedResponse =
        serde_json::from_value(value).map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(ApiResponse { data: c, meta: None }))
}

#[derive(Debug, Deserialize)]
pub struct UpdateCannedResponseRequest {
    pub value: Option<String>,
}

pub async fn update_canned_response(
    Path(id): Path<String>,
    State(_s): State<Arc<AppState>>,
    Json(req): Json<UpdateCannedResponseRequest>,
) -> Result<Json<ApiResponse<CannedResponse>>, ApiError> {
    let store_arc = canned_response_store();
    let mut store = store_arc.lock();
    let value = store.get(&id).cloned().ok_or_else(|| {
        ApiError::NotFound(
            format!("canned_response {id}"),
            "CANNED_RESPONSE_NOT_FOUND".into(),
        )
    })?;
    let mut c: CannedResponse =
        serde_json::from_value(value).map_err(|e| ApiError::Internal(e.to_string()))?;
    if let Some(v) = req.value {
        c.value = v;
    }
    let updated = serde_json::to_value(&c).map_err(|e| ApiError::Internal(e.to_string()))?;
    store.insert(id, updated);
    Ok(Json(ApiResponse { data: c, meta: None }))
}

pub async fn delete_canned_response(
    Path(id): Path<String>,
    State(_s): State<Arc<AppState>>,
) -> Result<StatusCode, ApiError> {
    let store_arc = canned_response_store();
    let mut store = store_arc.lock();
    store.remove(&id).ok_or_else(|| {
        ApiError::NotFound(
            format!("canned_response {id}"),
            "CANNED_RESPONSE_NOT_FOUND".into(),
        )
    })?;
    Ok(StatusCode::NO_CONTENT)
}

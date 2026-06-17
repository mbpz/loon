//! `observations` resource routes.

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
use loon_sdk::Observation;

// Phase 1 stub store keyed by EvaluationId (Observation::id). Real persistence lands later.
static OBSERVATIONS: OnceLock<Arc<Mutex<HashMap<String, serde_json::Value>>>> = OnceLock::new();
fn observation_store() -> Arc<Mutex<HashMap<String, serde_json::Value>>> {
    OBSERVATIONS
        .get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
        .clone()
}

pub async fn list_observations(
    State(_s): State<Arc<AppState>>,
) -> Result<Json<ApiListResponse<Observation>>, ApiError> {
    let store_arc = observation_store();
    let store = store_arc.lock();
    let items: Vec<Observation> = store
        .values()
        .filter_map(|v| serde_json::from_value(v.clone()).ok())
        .collect();
    Ok(Json(ApiListResponse {
        total: items.len(),
        items,
    }))
}

#[derive(Debug, Deserialize)]
pub struct CreateObservationRequest {
    pub agent_id: loon_core::AgentId,
    #[serde(default)]
    pub condition: String,
    #[serde(default)]
    pub tools: Vec<loon_core::ToolId>,
}

pub async fn create_observation(
    State(_s): State<Arc<AppState>>,
    Json(req): Json<CreateObservationRequest>,
) -> Result<Json<ApiResponse<Observation>>, ApiError> {
    let o = Observation::new(req.condition, req.tools, &req.agent_id);
    let store_arc = observation_store();
    let mut store = store_arc.lock();
    let value = serde_json::to_value(&o).map_err(|e| ApiError::Internal(e.to_string()))?;
    store.insert(o.id.0.clone(), value);
    Ok(Json(ApiResponse { data: o, meta: None }))
}

pub async fn read_observation(
    Path(id): Path<String>,
    State(_s): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Observation>>, ApiError> {
    let store_arc = observation_store();
    let store = store_arc.lock();
    let value = store
        .get(&id)
        .cloned()
        .ok_or_else(|| ApiError::NotFound(format!("observation {id}"), "OBSERVATION_NOT_FOUND".into()))?;
    let o: Observation =
        serde_json::from_value(value).map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(ApiResponse { data: o, meta: None }))
}

#[derive(Debug, Deserialize)]
pub struct UpdateObservationRequest {
    pub condition: Option<String>,
    pub tools: Option<Vec<loon_core::ToolId>>,
    pub enabled: Option<bool>,
}

pub async fn update_observation(
    Path(id): Path<String>,
    State(_s): State<Arc<AppState>>,
    Json(req): Json<UpdateObservationRequest>,
) -> Result<Json<ApiResponse<Observation>>, ApiError> {
    let store_arc = observation_store();
    let mut store = store_arc.lock();
    let value = store
        .get(&id)
        .cloned()
        .ok_or_else(|| ApiError::NotFound(format!("observation {id}"), "OBSERVATION_NOT_FOUND".into()))?;
    let mut o: Observation =
        serde_json::from_value(value).map_err(|e| ApiError::Internal(e.to_string()))?;
    if let Some(c) = req.condition {
        o.condition = c;
    }
    if let Some(t) = req.tools {
        o.tools = t;
    }
    if let Some(e) = req.enabled {
        o.enabled = e;
    }
    let updated = serde_json::to_value(&o).map_err(|e| ApiError::Internal(e.to_string()))?;
    store.insert(id, updated);
    Ok(Json(ApiResponse { data: o, meta: None }))
}

pub async fn delete_observation(
    Path(id): Path<String>,
    State(_s): State<Arc<AppState>>,
) -> Result<StatusCode, ApiError> {
    let store_arc = observation_store();
    let mut store = store_arc.lock();
    store
        .remove(&id)
        .ok_or_else(|| ApiError::NotFound(format!("observation {id}"), "OBSERVATION_NOT_FOUND".into()))?;
    Ok(StatusCode::NO_CONTENT)
}

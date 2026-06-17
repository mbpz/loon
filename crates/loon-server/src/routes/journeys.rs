//! `journeys` resource routes.

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
use loon_sdk::Journey;

// Phase 1 stub store keyed by JourneyId. Real persistence lands later.
static JOURNEYS: OnceLock<Arc<Mutex<HashMap<String, serde_json::Value>>>> = OnceLock::new();
fn journey_store() -> Arc<Mutex<HashMap<String, serde_json::Value>>> {
    JOURNEYS
        .get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
        .clone()
}

pub async fn list_journeys(
    State(_s): State<Arc<AppState>>,
) -> Result<Json<ApiListResponse<Journey>>, ApiError> {
    let store_arc = journey_store();
    let store = store_arc.lock();
    let items: Vec<Journey> = store
        .values()
        .filter_map(|v| serde_json::from_value(v.clone()).ok())
        .collect();
    Ok(Json(ApiListResponse {
        total: items.len(),
        items,
    }))
}

#[derive(Debug, Deserialize)]
pub struct CreateJourneyRequest {
    pub agent_id: loon_core::AgentId,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
}

pub async fn create_journey(
    State(_s): State<Arc<AppState>>,
    Json(req): Json<CreateJourneyRequest>,
) -> Result<Json<ApiResponse<Journey>>, ApiError> {
    let root = loon_core::JourneyNode::initial();
    let j = Journey {
        id: loon_core::JourneyId::new(),
        agent_id: req.agent_id,
        title: req.title,
        description: req.description,
        root_id: root.id,
        tags: vec![],
        creation_utc: chrono::Utc::now(),
    };
    let store_arc = journey_store();
    let mut store = store_arc.lock();
    let value = serde_json::to_value(&j).map_err(|e| ApiError::Internal(e.to_string()))?;
    store.insert(j.id.0.clone(), value);
    Ok(Json(ApiResponse { data: j, meta: None }))
}

pub async fn read_journey(
    Path(id): Path<String>,
    State(_s): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Journey>>, ApiError> {
    let store_arc = journey_store();
    let store = store_arc.lock();
    let value = store
        .get(&id)
        .cloned()
        .ok_or_else(|| ApiError::NotFound(format!("journey {id}"), "JOURNEY_NOT_FOUND".into()))?;
    let j: Journey =
        serde_json::from_value(value).map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(ApiResponse { data: j, meta: None }))
}

#[derive(Debug, Deserialize)]
pub struct UpdateJourneyRequest {
    pub title: Option<String>,
    pub description: Option<String>,
}

pub async fn update_journey(
    Path(id): Path<String>,
    State(_s): State<Arc<AppState>>,
    Json(req): Json<UpdateJourneyRequest>,
) -> Result<Json<ApiResponse<Journey>>, ApiError> {
    let store_arc = journey_store();
    let mut store = store_arc.lock();
    let value = store
        .get(&id)
        .cloned()
        .ok_or_else(|| ApiError::NotFound(format!("journey {id}"), "JOURNEY_NOT_FOUND".into()))?;
    let mut j: Journey =
        serde_json::from_value(value).map_err(|e| ApiError::Internal(e.to_string()))?;
    if let Some(t) = req.title {
        j.title = t;
    }
    if let Some(d) = req.description {
        j.description = d;
    }
    let updated = serde_json::to_value(&j).map_err(|e| ApiError::Internal(e.to_string()))?;
    store.insert(id, updated);
    Ok(Json(ApiResponse { data: j, meta: None }))
}

pub async fn delete_journey(
    Path(id): Path<String>,
    State(_s): State<Arc<AppState>>,
) -> Result<StatusCode, ApiError> {
    let store_arc = journey_store();
    let mut store = store_arc.lock();
    store
        .remove(&id)
        .ok_or_else(|| ApiError::NotFound(format!("journey {id}"), "JOURNEY_NOT_FOUND".into()))?;
    Ok(StatusCode::NO_CONTENT)
}

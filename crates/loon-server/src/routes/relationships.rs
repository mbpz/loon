//! `relationships` resource routes.
//!
//! Phase 1: read-only (no create) per parlcant. list + read +
//! delete only.

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use parking_lot::Mutex;

use crate::api::common::{ApiError, ApiListResponse, ApiResponse};
use crate::app::AppState;
use loon_sdk::Relationship;

// Phase 1 stub store keyed by RelationshipId. Real persistence lands later.
static RELATIONSHIPS: OnceLock<Arc<Mutex<HashMap<String, serde_json::Value>>>> = OnceLock::new();
fn relationship_store() -> Arc<Mutex<HashMap<String, serde_json::Value>>> {
    RELATIONSHIPS
        .get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
        .clone()
}

pub async fn list_relationships(
    State(_s): State<Arc<AppState>>,
) -> Result<Json<ApiListResponse<Relationship>>, ApiError> {
    let store_arc = relationship_store();
    let store = store_arc.lock();
    let items: Vec<Relationship> = store
        .values()
        .filter_map(|v| serde_json::from_value(v.clone()).ok())
        .collect();
    Ok(Json(ApiListResponse {
        total: items.len(),
        items,
    }))
}

pub async fn create_relationship(
    State(_s): State<Arc<AppState>>,
    Json(_req): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<Relationship>>, ApiError> {
    Err(ApiError::InvalidArgument(
        "relationships".into(),
        "CREATE_NOT_SUPPORTED".into(),
    ))
}

pub async fn read_relationship(
    Path(id): Path<String>,
    State(_s): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Relationship>>, ApiError> {
    let store_arc = relationship_store();
    let store = store_arc.lock();
    let value = store.get(&id).cloned().ok_or_else(|| {
        ApiError::NotFound(format!("relationship {id}"), "RELATIONSHIP_NOT_FOUND".into())
    })?;
    let r: Relationship =
        serde_json::from_value(value).map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(ApiResponse { data: r, meta: None }))
}

pub async fn delete_relationship(
    Path(id): Path<String>,
    State(_s): State<Arc<AppState>>,
) -> Result<StatusCode, ApiError> {
    let store_arc = relationship_store();
    let mut store = store_arc.lock();
    store.remove(&id).ok_or_else(|| {
        ApiError::NotFound(format!("relationship {id}"), "RELATIONSHIP_NOT_FOUND".into())
    })?;
    Ok(StatusCode::NO_CONTENT)
}

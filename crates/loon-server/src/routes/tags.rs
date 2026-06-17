//! `tags` resource routes.

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
use loon_sdk::Tag;

// Phase 1 stub store keyed by TagId. Real persistence lands later.
static TAGS: OnceLock<Arc<Mutex<HashMap<String, serde_json::Value>>>> = OnceLock::new();
fn tag_store() -> Arc<Mutex<HashMap<String, serde_json::Value>>> {
    TAGS
        .get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
        .clone()
}

pub async fn list_tags(
    State(_s): State<Arc<AppState>>,
) -> Result<Json<ApiListResponse<Tag>>, ApiError> {
    let store_arc = tag_store();
    let store = store_arc.lock();
    let items: Vec<Tag> = store
        .values()
        .filter_map(|v| serde_json::from_value(v.clone()).ok())
        .collect();
    Ok(Json(ApiListResponse {
        total: items.len(),
        items,
    }))
}

#[derive(Debug, Deserialize)]
pub struct CreateTagRequest {
    pub name: String,
}

pub async fn create_tag(
    State(_s): State<Arc<AppState>>,
    Json(req): Json<CreateTagRequest>,
) -> Result<Json<ApiResponse<Tag>>, ApiError> {
    let t = Tag::new(req.name);
    let store_arc = tag_store();
    let mut store = store_arc.lock();
    let value = serde_json::to_value(&t).map_err(|e| ApiError::Internal(e.to_string()))?;
    store.insert(t.id.0.clone(), value);
    Ok(Json(ApiResponse { data: t, meta: None }))
}

pub async fn read_tag(
    Path(id): Path<String>,
    State(_s): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Tag>>, ApiError> {
    let store_arc = tag_store();
    let store = store_arc.lock();
    let value = store
        .get(&id)
        .cloned()
        .ok_or_else(|| ApiError::NotFound(format!("tag {id}"), "TAG_NOT_FOUND".into()))?;
    let t: Tag =
        serde_json::from_value(value).map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(ApiResponse { data: t, meta: None }))
}

#[derive(Debug, Deserialize)]
pub struct UpdateTagRequest {
    pub name: Option<String>,
}

pub async fn update_tag(
    Path(id): Path<String>,
    State(_s): State<Arc<AppState>>,
    Json(req): Json<UpdateTagRequest>,
) -> Result<Json<ApiResponse<Tag>>, ApiError> {
    let store_arc = tag_store();
    let mut store = store_arc.lock();
    let value = store
        .get(&id)
        .cloned()
        .ok_or_else(|| ApiError::NotFound(format!("tag {id}"), "TAG_NOT_FOUND".into()))?;
    let mut t: Tag =
        serde_json::from_value(value).map_err(|e| ApiError::Internal(e.to_string()))?;
    if let Some(n) = req.name {
        t.name = n;
    }
    let updated = serde_json::to_value(&t).map_err(|e| ApiError::Internal(e.to_string()))?;
    store.insert(id, updated);
    Ok(Json(ApiResponse { data: t, meta: None }))
}

pub async fn delete_tag(
    Path(id): Path<String>,
    State(_s): State<Arc<AppState>>,
) -> Result<StatusCode, ApiError> {
    let store_arc = tag_store();
    let mut store = store_arc.lock();
    store
        .remove(&id)
        .ok_or_else(|| ApiError::NotFound(format!("tag {id}"), "TAG_NOT_FOUND".into()))?;
    Ok(StatusCode::NO_CONTENT)
}

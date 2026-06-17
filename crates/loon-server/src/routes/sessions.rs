//! `sessions` resource routes.

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
use loon_sdk::Session;

// Phase 1 stub store keyed by SessionId. Real persistence lands later.
static SESSIONS: OnceLock<Arc<Mutex<HashMap<String, serde_json::Value>>>> = OnceLock::new();
fn session_store() -> Arc<Mutex<HashMap<String, serde_json::Value>>> {
    SESSIONS
        .get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
        .clone()
}

pub async fn list_sessions(
    State(_s): State<Arc<AppState>>,
) -> Result<Json<ApiListResponse<Session>>, ApiError> {
    let store_arc = session_store();
    let store = store_arc.lock();
    let items: Vec<Session> = store
        .values()
        .filter_map(|v| serde_json::from_value(v.clone()).ok())
        .collect();
    Ok(Json(ApiListResponse {
        total: items.len(),
        items,
    }))
}

#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    pub agent_id: loon_core::AgentId,
    #[serde(default)]
    pub customer_id: Option<loon_core::CustomerId>,
    #[serde(default)]
    pub title: Option<String>,
}

pub async fn create_session(
    State(_s): State<Arc<AppState>>,
    Json(req): Json<CreateSessionRequest>,
) -> Result<Json<ApiResponse<Session>>, ApiError> {
    let mut session = Session::new(&req.agent_id);
    session.customer_id = req.customer_id;
    session.title = req.title;
    let store_arc = session_store();
    let mut store = store_arc.lock();
    let value =
        serde_json::to_value(&session).map_err(|e| ApiError::Internal(e.to_string()))?;
    store.insert(session.id.0.clone(), value);
    Ok(Json(ApiResponse { data: session, meta: None }))
}

pub async fn read_session(
    Path(id): Path<String>,
    State(_s): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Session>>, ApiError> {
    let store_arc = session_store();
    let store = store_arc.lock();
    let value = store
        .get(&id)
        .cloned()
        .ok_or_else(|| ApiError::NotFound(format!("session {id}"), "SESSION_NOT_FOUND".into()))?;
    let s: Session =
        serde_json::from_value(value).map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(ApiResponse { data: s, meta: None }))
}

#[derive(Debug, Deserialize)]
pub struct UpdateSessionRequest {
    pub customer_id: Option<loon_core::CustomerId>,
    pub title: Option<String>,
    pub mode: Option<loon_core::SessionMode>,
}

pub async fn update_session(
    Path(id): Path<String>,
    State(_s): State<Arc<AppState>>,
    Json(req): Json<UpdateSessionRequest>,
) -> Result<Json<ApiResponse<Session>>, ApiError> {
    let store_arc = session_store();
    let mut store = store_arc.lock();
    let value = store
        .get(&id)
        .cloned()
        .ok_or_else(|| ApiError::NotFound(format!("session {id}"), "SESSION_NOT_FOUND".into()))?;
    let mut s: Session =
        serde_json::from_value(value).map_err(|e| ApiError::Internal(e.to_string()))?;
    if let Some(c) = req.customer_id {
        s.customer_id = Some(c);
    }
    if let Some(t) = req.title {
        s.title = Some(t);
    }
    if let Some(m) = req.mode {
        s.mode = m;
    }
    let updated = serde_json::to_value(&s).map_err(|e| ApiError::Internal(e.to_string()))?;
    store.insert(id, updated);
    Ok(Json(ApiResponse { data: s, meta: None }))
}

pub async fn delete_session(
    Path(id): Path<String>,
    State(_s): State<Arc<AppState>>,
) -> Result<StatusCode, ApiError> {
    let store_arc = session_store();
    let mut store = store_arc.lock();
    store
        .remove(&id)
        .ok_or_else(|| ApiError::NotFound(format!("session {id}"), "SESSION_NOT_FOUND".into()))?;
    Ok(StatusCode::NO_CONTENT)
}

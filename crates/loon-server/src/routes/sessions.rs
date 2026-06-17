//! `sessions` resource routes.

use std::sync::Arc;

use axum::{extract::State, Json};
use serde::Deserialize;

use crate::api::common::{ApiError, ApiListResponse, ApiResponse};
use crate::app::AppState;
use loon_sdk::Session;

pub async fn list_sessions(
    State(_s): State<Arc<AppState>>,
) -> Result<Json<ApiListResponse<Session>>, ApiError> {
    Ok(Json(ApiListResponse { items: vec![], total: 0 }))
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
    Ok(Json(ApiResponse { data: session, meta: None }))
}

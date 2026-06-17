//! `canned_responses` resource routes.

use std::sync::Arc;

use axum::{extract::State, Json};
use serde::Deserialize;

use crate::api::common::{ApiError, ApiListResponse, ApiResponse};
use crate::app::AppState;
use loon_sdk::CannedResponse;

pub async fn list_canned_responses(
    State(_s): State<Arc<AppState>>,
) -> Result<Json<ApiListResponse<CannedResponse>>, ApiError> {
    Ok(Json(ApiListResponse { items: vec![], total: 0 }))
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
    Ok(Json(ApiResponse { data: c, meta: None }))
}

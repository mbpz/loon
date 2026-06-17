//! `observations` resource routes.

use std::sync::Arc;

use axum::{extract::State, Json};
use serde::Deserialize;

use crate::api::common::{ApiError, ApiListResponse, ApiResponse};
use crate::app::AppState;
use loon_sdk::Observation;

pub async fn list_observations(
    State(_s): State<Arc<AppState>>,
) -> Result<Json<ApiListResponse<Observation>>, ApiError> {
    Ok(Json(ApiListResponse { items: vec![], total: 0 }))
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
    Ok(Json(ApiResponse { data: o, meta: None }))
}

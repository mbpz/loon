//! `agents` resource routes.
//!
//! Phase 1 wiring: `list_*` returns an empty list and `create_*`
//! returns a freshly-constructed entity. Real persistence wiring
//! (storing through `s.server`'s engine) lands once the
//! `AlphaEngine` exposes high-level CRUD methods.

use std::sync::Arc;

use axum::{extract::State, Json};
use serde::Deserialize;

use crate::api::common::{ApiError, ApiListResponse, ApiResponse};
use crate::app::AppState;
use loon_sdk::Agent;

pub async fn list_agents(
    State(_s): State<Arc<AppState>>,
) -> Result<Json<ApiListResponse<Agent>>, ApiError> {
    Ok(Json(ApiListResponse { items: vec![], total: 0 }))
}

#[derive(Debug, Deserialize)]
pub struct CreateAgentRequest {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
}

pub async fn create_agent(
    State(_s): State<Arc<AppState>>,
    Json(req): Json<CreateAgentRequest>,
) -> Result<Json<ApiResponse<Agent>>, ApiError> {
    let agent = Agent::new(req.name, req.description.unwrap_or_default());
    Ok(Json(ApiResponse { data: agent, meta: None }))
}

//! `agents` resource routes — Phase 1 stubs.

use std::sync::Arc;

use axum::{extract::State, Json};

use crate::api::common::{ApiError, ApiListResponse, ApiResponse};
use crate::app::AppState;
use loon_sdk::Agent;

pub async fn list_agents(
    State(_s): State<Arc<AppState>>,
) -> Result<Json<ApiListResponse<Agent>>, ApiError> {
    Err(ApiError::NotFound("agents".into(), "NOT_IMPLEMENTED".into()))
}

pub async fn create_agent(
    State(_s): State<Arc<AppState>>,
    Json(_req): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<Agent>>, ApiError> {
    Err(ApiError::NotFound("agents".into(), "NOT_IMPLEMENTED".into()))
}
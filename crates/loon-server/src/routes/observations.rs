//! `observations` resource routes — Phase 1 stubs.

use std::sync::Arc;

use axum::{extract::State, Json};

use crate::api::common::{ApiError, ApiListResponse, ApiResponse};
use crate::app::AppState;
use loon_sdk::Observation;

pub async fn list_observations(
    State(_s): State<Arc<AppState>>,
) -> Result<Json<ApiListResponse<Observation>>, ApiError> {
    Err(ApiError::NotFound(
        "observations".into(),
        "NOT_IMPLEMENTED".into(),
    ))
}

pub async fn create_observation(
    State(_s): State<Arc<AppState>>,
    Json(_req): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<Observation>>, ApiError> {
    Err(ApiError::NotFound(
        "observations".into(),
        "NOT_IMPLEMENTED".into(),
    ))
}
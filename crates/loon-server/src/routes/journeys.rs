//! `journeys` resource routes — Phase 1 stubs.

use std::sync::Arc;

use axum::{extract::State, Json};

use crate::api::common::{ApiError, ApiListResponse, ApiResponse};
use crate::app::AppState;
use loon_sdk::Journey;

pub async fn list_journeys(
    State(_s): State<Arc<AppState>>,
) -> Result<Json<ApiListResponse<Journey>>, ApiError> {
    Err(ApiError::NotFound(
        "journeys".into(),
        "NOT_IMPLEMENTED".into(),
    ))
}

pub async fn create_journey(
    State(_s): State<Arc<AppState>>,
    Json(_req): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<Journey>>, ApiError> {
    Err(ApiError::NotFound(
        "journeys".into(),
        "NOT_IMPLEMENTED".into(),
    ))
}

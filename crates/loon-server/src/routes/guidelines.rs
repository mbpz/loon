//! `guidelines` resource routes — Phase 1 stubs.

use std::sync::Arc;

use axum::{extract::State, Json};

use crate::api::common::{ApiError, ApiListResponse, ApiResponse};
use crate::app::AppState;
use loon_sdk::Guideline;

pub async fn list_guidelines(
    State(_s): State<Arc<AppState>>,
) -> Result<Json<ApiListResponse<Guideline>>, ApiError> {
    Err(ApiError::NotFound(
        "guidelines".into(),
        "NOT_IMPLEMENTED".into(),
    ))
}

pub async fn create_guideline(
    State(_s): State<Arc<AppState>>,
    Json(_req): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<Guideline>>, ApiError> {
    Err(ApiError::NotFound(
        "guidelines".into(),
        "NOT_IMPLEMENTED".into(),
    ))
}
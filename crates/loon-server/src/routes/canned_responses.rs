//! `canned_responses` resource routes — Phase 1 stubs.

use std::sync::Arc;

use axum::{extract::State, Json};

use crate::api::common::{ApiError, ApiListResponse, ApiResponse};
use crate::app::AppState;
use loon_sdk::CannedResponse;

pub async fn list_canned_responses(
    State(_s): State<Arc<AppState>>,
) -> Result<Json<ApiListResponse<CannedResponse>>, ApiError> {
    Err(ApiError::NotFound(
        "canned_responses".into(),
        "NOT_IMPLEMENTED".into(),
    ))
}

pub async fn create_canned_response(
    State(_s): State<Arc<AppState>>,
    Json(_req): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<CannedResponse>>, ApiError> {
    Err(ApiError::NotFound(
        "canned_responses".into(),
        "NOT_IMPLEMENTED".into(),
    ))
}
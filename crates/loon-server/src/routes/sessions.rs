//! `sessions` resource routes — Phase 1 stubs.

use std::sync::Arc;

use axum::{extract::State, Json};

use crate::api::common::{ApiError, ApiListResponse, ApiResponse};
use crate::app::AppState;
use loon_sdk::Session;

pub async fn list_sessions(
    State(_s): State<Arc<AppState>>,
) -> Result<Json<ApiListResponse<Session>>, ApiError> {
    Err(ApiError::NotFound(
        "sessions".into(),
        "NOT_IMPLEMENTED".into(),
    ))
}

pub async fn create_session(
    State(_s): State<Arc<AppState>>,
    Json(_req): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<Session>>, ApiError> {
    Err(ApiError::NotFound(
        "sessions".into(),
        "NOT_IMPLEMENTED".into(),
    ))
}
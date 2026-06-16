//! `tools` resource routes — Phase 1 stubs.

use std::sync::Arc;

use axum::{extract::State, Json};

use crate::api::common::{ApiError, ApiListResponse, ApiResponse};
use crate::app::AppState;
use loon_sdk::Tool;

pub async fn list_tools(
    State(_s): State<Arc<AppState>>,
) -> Result<Json<ApiListResponse<Tool>>, ApiError> {
    Err(ApiError::NotFound(
        "tools".into(),
        "NOT_IMPLEMENTED".into(),
    ))
}

pub async fn create_tool(
    State(_s): State<Arc<AppState>>,
    Json(_req): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<Tool>>, ApiError> {
    Err(ApiError::NotFound(
        "tools".into(),
        "NOT_IMPLEMENTED".into(),
    ))
}
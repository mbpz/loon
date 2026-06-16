//! `tags` resource routes — Phase 1 stubs.

use std::sync::Arc;

use axum::{extract::State, Json};

use crate::api::common::{ApiError, ApiListResponse, ApiResponse};
use crate::app::AppState;
use loon_sdk::Tag;

pub async fn list_tags(
    State(_s): State<Arc<AppState>>,
) -> Result<Json<ApiListResponse<Tag>>, ApiError> {
    Err(ApiError::NotFound("tags".into(), "NOT_IMPLEMENTED".into()))
}

pub async fn create_tag(
    State(_s): State<Arc<AppState>>,
    Json(_req): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<Tag>>, ApiError> {
    Err(ApiError::NotFound("tags".into(), "NOT_IMPLEMENTED".into()))
}

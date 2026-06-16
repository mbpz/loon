//! `glossary` resource routes — Phase 1 stubs.

use std::sync::Arc;

use axum::{extract::State, Json};

use crate::api::common::{ApiError, ApiListResponse, ApiResponse};
use crate::app::AppState;
use loon_sdk::Glossary;

pub async fn list_glossary(
    State(_s): State<Arc<AppState>>,
) -> Result<Json<ApiListResponse<Glossary>>, ApiError> {
    Err(ApiError::NotFound(
        "glossary".into(),
        "NOT_IMPLEMENTED".into(),
    ))
}

pub async fn create_glossary_entry(
    State(_s): State<Arc<AppState>>,
    Json(_req): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<Glossary>>, ApiError> {
    Err(ApiError::NotFound(
        "glossary".into(),
        "NOT_IMPLEMENTED".into(),
    ))
}
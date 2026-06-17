//! `relationships` resource routes.

use std::sync::Arc;

use axum::{extract::State, Json};

use crate::api::common::{ApiError, ApiListResponse, ApiResponse};
use crate::app::AppState;
use loon_sdk::Relationship;

pub async fn list_relationships(
    State(_s): State<Arc<AppState>>,
) -> Result<Json<ApiListResponse<Relationship>>, ApiError> {
    Ok(Json(ApiListResponse { items: vec![], total: 0 }))
}

pub async fn create_relationship(
    State(_s): State<Arc<AppState>>,
    Json(_req): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<Relationship>>, ApiError> {
    Err(ApiError::InvalidArgument(
        "relationships".into(),
        "CREATE_NOT_SUPPORTED".into(),
    ))
}

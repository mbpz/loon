//! `tags` resource routes.

use std::sync::Arc;

use axum::{extract::State, Json};
use serde::Deserialize;

use crate::api::common::{ApiError, ApiListResponse, ApiResponse};
use crate::app::AppState;
use loon_sdk::Tag;

pub async fn list_tags(
    State(_s): State<Arc<AppState>>,
) -> Result<Json<ApiListResponse<Tag>>, ApiError> {
    Ok(Json(ApiListResponse { items: vec![], total: 0 }))
}

#[derive(Debug, Deserialize)]
pub struct CreateTagRequest {
    pub name: String,
}

pub async fn create_tag(
    State(_s): State<Arc<AppState>>,
    Json(req): Json<CreateTagRequest>,
) -> Result<Json<ApiResponse<Tag>>, ApiError> {
    let t = Tag::new(req.name);
    Ok(Json(ApiResponse { data: t, meta: None }))
}

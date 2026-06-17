//! `guidelines` resource routes.

use std::sync::Arc;

use axum::{extract::State, Json};
use serde::Deserialize;

use crate::api::common::{ApiError, ApiListResponse, ApiResponse};
use crate::app::AppState;
use loon_sdk::{Guideline, GuidelineContent};

pub async fn list_guidelines(
    State(_s): State<Arc<AppState>>,
) -> Result<Json<ApiListResponse<Guideline>>, ApiError> {
    Ok(Json(ApiListResponse { items: vec![], total: 0 }))
}

#[derive(Debug, Deserialize)]
pub struct CreateGuidelineRequest {
    pub agent_id: loon_core::AgentId,
    #[serde(default)]
    pub condition: String,
    #[serde(default)]
    pub action: String,
    #[serde(default)]
    pub description: Option<String>,
}

pub async fn create_guideline(
    State(_s): State<Arc<AppState>>,
    Json(req): Json<CreateGuidelineRequest>,
) -> Result<Json<ApiResponse<Guideline>>, ApiError> {
    let g = Guideline::new(
        GuidelineContent {
            condition: req.condition,
            action: req.action,
            description: req.description,
        },
        &req.agent_id,
        true,
        0,
    );
    Ok(Json(ApiResponse { data: g, meta: None }))
}

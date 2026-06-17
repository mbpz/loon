//! `journeys` resource routes.

use std::sync::Arc;

use axum::{extract::State, Json};
use serde::Deserialize;

use crate::api::common::{ApiError, ApiListResponse, ApiResponse};
use crate::app::AppState;
use loon_sdk::Journey;

pub async fn list_journeys(
    State(_s): State<Arc<AppState>>,
) -> Result<Json<ApiListResponse<Journey>>, ApiError> {
    Ok(Json(ApiListResponse { items: vec![], total: 0 }))
}

#[derive(Debug, Deserialize)]
pub struct CreateJourneyRequest {
    pub agent_id: loon_core::AgentId,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
}

pub async fn create_journey(
    State(_s): State<Arc<AppState>>,
    Json(req): Json<CreateJourneyRequest>,
) -> Result<Json<ApiResponse<Journey>>, ApiError> {
    let root = loon_core::JourneyNode::initial();
    let j = Journey {
        id: loon_core::JourneyId::new(),
        agent_id: req.agent_id,
        title: req.title,
        description: req.description,
        root_id: root.id,
        tags: vec![],
        creation_utc: chrono::Utc::now(),
    };
    Ok(Json(ApiResponse { data: j, meta: None }))
}

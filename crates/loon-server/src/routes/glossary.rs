//! `glossary` resource routes.

use std::sync::Arc;

use axum::{extract::State, Json};
use serde::Deserialize;

use crate::api::common::{ApiError, ApiListResponse, ApiResponse};
use crate::app::AppState;
use loon_sdk::Glossary;

pub async fn list_glossary(
    State(_s): State<Arc<AppState>>,
) -> Result<Json<ApiListResponse<Glossary>>, ApiError> {
    Ok(Json(ApiListResponse { items: vec![], total: 0 }))
}

#[derive(Debug, Deserialize)]
pub struct CreateGlossaryEntryRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
}

pub async fn create_glossary_entry(
    State(_s): State<Arc<AppState>>,
    Json(req): Json<CreateGlossaryEntryRequest>,
) -> Result<Json<ApiResponse<Glossary>>, ApiError> {
    let g = Glossary {
        terms: vec![loon_core::Term::new(req.name, req.description)],
    };
    Ok(Json(ApiResponse { data: g, meta: None }))
}

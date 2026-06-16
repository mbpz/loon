//! `customers` resource routes — Phase 1 stubs.

use std::sync::Arc;

use axum::{extract::State, Json};

use crate::api::common::{ApiError, ApiListResponse, ApiResponse};
use crate::app::AppState;
use loon_sdk::Customer;

pub async fn list_customers(
    State(_s): State<Arc<AppState>>,
) -> Result<Json<ApiListResponse<Customer>>, ApiError> {
    Err(ApiError::NotFound(
        "customers".into(),
        "NOT_IMPLEMENTED".into(),
    ))
}

pub async fn create_customer(
    State(_s): State<Arc<AppState>>,
    Json(_req): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<Customer>>, ApiError> {
    Err(ApiError::NotFound(
        "customers".into(),
        "NOT_IMPLEMENTED".into(),
    ))
}
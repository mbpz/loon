//! `customers` resource routes.

use std::sync::Arc;

use axum::{extract::State, Json};
use serde::Deserialize;

use crate::api::common::{ApiError, ApiListResponse, ApiResponse};
use crate::app::AppState;
use loon_sdk::Customer;

pub async fn list_customers(
    State(_s): State<Arc<AppState>>,
) -> Result<Json<ApiListResponse<Customer>>, ApiError> {
    Ok(Json(ApiListResponse { items: vec![], total: 0 }))
}

#[derive(Debug, Deserialize)]
pub struct CreateCustomerRequest {
    pub name: String,
}

pub async fn create_customer(
    State(_s): State<Arc<AppState>>,
    Json(req): Json<CreateCustomerRequest>,
) -> Result<Json<ApiResponse<Customer>>, ApiError> {
    let c = Customer::new(req.name);
    Ok(Json(ApiResponse { data: c, meta: None }))
}

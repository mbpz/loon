//! `customers` resource routes.

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use parking_lot::Mutex;
use serde::Deserialize;

use crate::api::common::{ApiError, ApiListResponse, ApiResponse};
use crate::app::AppState;
use loon_sdk::Customer;

// Phase 1 stub store keyed by CustomerId. Real persistence lands later.
static CUSTOMERS: OnceLock<Arc<Mutex<HashMap<String, serde_json::Value>>>> = OnceLock::new();
fn customer_store() -> Arc<Mutex<HashMap<String, serde_json::Value>>> {
    CUSTOMERS
        .get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
        .clone()
}

pub async fn list_customers(
    State(_s): State<Arc<AppState>>,
) -> Result<Json<ApiListResponse<Customer>>, ApiError> {
    let store_arc = customer_store();
    let store = store_arc.lock();
    let items: Vec<Customer> = store
        .values()
        .filter_map(|v| serde_json::from_value(v.clone()).ok())
        .collect();
    Ok(Json(ApiListResponse {
        total: items.len(),
        items,
    }))
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
    let store_arc = customer_store();
    let mut store = store_arc.lock();
    let value = serde_json::to_value(&c).map_err(|e| ApiError::Internal(e.to_string()))?;
    store.insert(c.id.0.clone(), value);
    Ok(Json(ApiResponse { data: c, meta: None }))
}

pub async fn read_customer(
    Path(id): Path<String>,
    State(_s): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Customer>>, ApiError> {
    let store_arc = customer_store();
    let store = store_arc.lock();
    let value = store
        .get(&id)
        .cloned()
        .ok_or_else(|| ApiError::NotFound(format!("customer {id}"), "CUSTOMER_NOT_FOUND".into()))?;
    let c: Customer =
        serde_json::from_value(value).map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(ApiResponse { data: c, meta: None }))
}

#[derive(Debug, Deserialize)]
pub struct UpdateCustomerRequest {
    pub name: Option<String>,
}

pub async fn update_customer(
    Path(id): Path<String>,
    State(_s): State<Arc<AppState>>,
    Json(req): Json<UpdateCustomerRequest>,
) -> Result<Json<ApiResponse<Customer>>, ApiError> {
    let store_arc = customer_store();
    let mut store = store_arc.lock();
    let value = store
        .get(&id)
        .cloned()
        .ok_or_else(|| ApiError::NotFound(format!("customer {id}"), "CUSTOMER_NOT_FOUND".into()))?;
    let mut c: Customer =
        serde_json::from_value(value).map_err(|e| ApiError::Internal(e.to_string()))?;
    if let Some(n) = req.name {
        c.name = n;
    }
    let updated = serde_json::to_value(&c).map_err(|e| ApiError::Internal(e.to_string()))?;
    store.insert(id, updated);
    Ok(Json(ApiResponse { data: c, meta: None }))
}

pub async fn delete_customer(
    Path(id): Path<String>,
    State(_s): State<Arc<AppState>>,
) -> Result<StatusCode, ApiError> {
    let store_arc = customer_store();
    let mut store = store_arc.lock();
    store
        .remove(&id)
        .ok_or_else(|| ApiError::NotFound(format!("customer {id}"), "CUSTOMER_NOT_FOUND".into()))?;
    Ok(StatusCode::NO_CONTENT)
}

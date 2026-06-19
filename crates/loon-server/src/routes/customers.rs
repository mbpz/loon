//! `customers` resource routes.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

use crate::api::common::{ApiError, ApiListResponse, ApiResponse};
use crate::app::AppState;
use loon_core::{CoreError, CustomerId, CustomerUpdateParams};
use loon_sdk::Customer;

pub async fn list_customers(
    State(s): State<Arc<AppState>>,
) -> Result<Json<ApiListResponse<Customer>>, ApiError> {
    let items = s
        .server
        .queries
        .customer_store
        .list(&[])
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
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
    State(s): State<Arc<AppState>>,
    Json(req): Json<CreateCustomerRequest>,
) -> Result<Json<ApiResponse<Customer>>, ApiError> {
    let c = Customer::new(req.name);
    let stored = s
        .server
        .queries
        .customer_store
        .create(c)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(ApiResponse {
        data: stored,
        meta: None,
    }))
}

pub async fn read_customer(
    Path(id): Path<String>,
    State(s): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Customer>>, ApiError> {
    let cid = CustomerId(id.clone());
    let c = s
        .server
        .queries
        .customer_store
        .read(&cid)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("customer {id}"), "CUSTOMER_NOT_FOUND".into()))?;
    Ok(Json(ApiResponse {
        data: c,
        meta: None,
    }))
}

#[derive(Debug, Deserialize)]
pub struct UpdateCustomerRequest {
    pub name: Option<String>,
}

pub async fn update_customer(
    Path(id): Path<String>,
    State(s): State<Arc<AppState>>,
    Json(req): Json<UpdateCustomerRequest>,
) -> Result<Json<ApiResponse<Customer>>, ApiError> {
    let cid = CustomerId(id.clone());
    let updated = s
        .server
        .queries
        .customer_store
        .update(
            &cid,
            CustomerUpdateParams {
                name: req.name,
                metadata: None,
            },
        )
        .await
        .map_err(|e| match e {
            CoreError::NotFound(_) => {
                ApiError::NotFound(format!("customer {id}"), "CUSTOMER_NOT_FOUND".into())
            }
            other => ApiError::Internal(other.to_string()),
        })?;
    Ok(Json(ApiResponse {
        data: updated,
        meta: None,
    }))
}

pub async fn delete_customer(
    Path(id): Path<String>,
    State(s): State<Arc<AppState>>,
) -> Result<StatusCode, ApiError> {
    let cid = CustomerId(id);
    s.server
        .queries
        .customer_store
        .delete(&cid)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::NoopAuthProvider;
    use crate::middleware::rate_limit::{RateLimitConfig, RateLimiter};

    async fn test_app_state() -> Arc<AppState> {
        let server = loon_sdk::Server::builder()
            .build()
            .await
            .expect("build server");
        Arc::new(AppState {
            server: Arc::new(server),
            auth: Arc::new(NoopAuthProvider),
            rate_limiter: Arc::new(RateLimiter::new(RateLimitConfig::default())),
        })
    }

    #[tokio::test]
    async fn customer_full_lifecycle() {
        let state = test_app_state().await;

        let created = create_customer(
            State(state.clone()),
            Json(CreateCustomerRequest {
                name: "alice".into(),
            }),
        )
        .await
        .expect("create ok");
        let id = created.data.id.0.clone();

        let listed = list_customers(State(state.clone())).await.expect("list ok");
        assert_eq!(listed.total, 1);

        let read = read_customer(Path(id.clone()), State(state.clone()))
            .await
            .expect("read ok");
        assert_eq!(read.data.name, "alice");

        let updated = update_customer(
            Path(id.clone()),
            State(state.clone()),
            Json(UpdateCustomerRequest {
                name: Some("alice@v2".into()),
            }),
        )
        .await
        .expect("update ok");
        assert_eq!(updated.data.name, "alice@v2");

        let status = delete_customer(Path(id.clone()), State(state.clone()))
            .await
            .expect("delete ok");
        assert_eq!(status, StatusCode::NO_CONTENT);

        let read_missing = read_customer(Path(id), State(state)).await;
        assert!(matches!(read_missing, Err(ApiError::NotFound(_, _))));
    }
}

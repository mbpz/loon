//! `tags` resource routes.
//!
//! Backed by `TagStore`. The store does not expose `update`, so the
//! PATCH handler returns 400 to make the limitation explicit.

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::Deserialize;

use crate::api::common::{ApiError, ApiListResponse, ApiResponse};
use crate::app::AppState;
use loon_core::TagId;
use loon_sdk::Tag;

pub async fn list_tags(
    State(s): State<Arc<AppState>>,
) -> Result<Json<ApiListResponse<Tag>>, ApiError> {
    let items = s
        .server
        .queries
        .tag_store
        .list()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(ApiListResponse {
        total: items.len(),
        items,
    }))
}

#[derive(Debug, Deserialize)]
pub struct CreateTagRequest {
    pub name: String,
}

pub async fn create_tag(
    State(s): State<Arc<AppState>>,
    Json(req): Json<CreateTagRequest>,
) -> Result<Json<ApiResponse<Tag>>, ApiError> {
    let t = Tag::new(req.name);
    let stored = s
        .server
        .queries
        .tag_store
        .create(t)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(ApiResponse {
        data: stored,
        meta: None,
    }))
}

pub async fn read_tag(
    Path(id): Path<String>,
    State(s): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Tag>>, ApiError> {
    let tid = TagId(id.clone());
    let t = s
        .server
        .queries
        .tag_store
        .read(&tid)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("tag {id}"), "TAG_NOT_FOUND".into()))?;
    Ok(Json(ApiResponse {
        data: t,
        meta: None,
    }))
}

#[derive(Debug, Deserialize)]
pub struct UpdateTagRequest {
    #[serde(default)]
    pub _unused: Option<String>,
}

pub async fn update_tag(
    Path(_id): Path<String>,
    State(_s): State<Arc<AppState>>,
    Json(_req): Json<UpdateTagRequest>,
) -> Result<Json<ApiResponse<Tag>>, ApiError> {
    Err(ApiError::InvalidArgument(
        "tags do not support update".into(),
        "TAG_UPDATE_NOT_SUPPORTED".into(),
    ))
}

pub async fn delete_tag(
    Path(id): Path<String>,
    State(s): State<Arc<AppState>>,
) -> Result<StatusCode, ApiError> {
    let tid = TagId(id);
    s.server
        .queries
        .tag_store
        .delete(&tid)
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
    async fn tag_create_read_delete() {
        let state = test_app_state().await;

        let created = create_tag(
            State(state.clone()),
            Json(CreateTagRequest {
                name: "vip".into(),
            }),
        )
        .await
        .expect("create ok");
        let id = created.data.id.0.clone();

        let listed = list_tags(State(state.clone())).await.expect("list ok");
        assert_eq!(listed.total, 1);

        let read = read_tag(Path(id.clone()), State(state.clone()))
            .await
            .expect("read ok");
        assert_eq!(read.data.name, "vip");

        let upd = update_tag(
            Path(id.clone()),
            State(state.clone()),
            Json(UpdateTagRequest { _unused: None }),
        )
        .await;
        assert!(matches!(upd, Err(ApiError::InvalidArgument(_, _))));

        let status = delete_tag(Path(id.clone()), State(state.clone()))
            .await
            .expect("delete ok");
        assert_eq!(status, StatusCode::NO_CONTENT);

        let read_missing = read_tag(Path(id), State(state)).await;
        assert!(matches!(read_missing, Err(ApiError::NotFound(_, _))));
    }
}

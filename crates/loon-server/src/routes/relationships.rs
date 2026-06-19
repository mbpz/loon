//! `relationships` resource routes.
//!
//! Read-only (no create) per the parlcant API. The list endpoint is
//! scoped to a single entity (`?entity_kind=Guideline&entity_id=X`)
//! because `RelationshipStore::list_for` requires a focal entity.
//! Without query params the list returns empty.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

use crate::api::common::{ApiError, ApiListResponse, ApiResponse};
use crate::app::AppState;
use loon_core::{RelationshipEntity, RelationshipEntityKind, RelationshipId};
use loon_sdk::Relationship;

#[derive(Debug, Deserialize)]
pub struct ListRelationshipsQuery {
    pub entity_kind: Option<RelationshipEntityKind>,
    pub entity_id: Option<String>,
}

pub async fn list_relationships(
    Query(q): Query<ListRelationshipsQuery>,
    State(s): State<Arc<AppState>>,
) -> Result<Json<ApiListResponse<Relationship>>, ApiError> {
    let items = match (q.entity_kind, q.entity_id) {
        (Some(kind), Some(id)) => s
            .server
            .queries
            .relationship_store
            .list_for(&RelationshipEntity { kind, id })
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?,
        _ => Vec::new(),
    };
    Ok(Json(ApiListResponse {
        total: items.len(),
        items,
    }))
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

pub async fn read_relationship(
    Path(id): Path<String>,
    State(s): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Relationship>>, ApiError> {
    let rid = RelationshipId(id.clone());
    let r = s
        .server
        .queries
        .relationship_store
        .read(&rid)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| {
            ApiError::NotFound(
                format!("relationship {id}"),
                "RELATIONSHIP_NOT_FOUND".into(),
            )
        })?;
    Ok(Json(ApiResponse {
        data: r,
        meta: None,
    }))
}

pub async fn delete_relationship(
    Path(id): Path<String>,
    State(s): State<Arc<AppState>>,
) -> Result<StatusCode, ApiError> {
    let rid = RelationshipId(id);
    s.server
        .queries
        .relationship_store
        .delete(&rid)
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
    async fn relationship_seed_read_delete() {
        // Seed the store directly (no public create endpoint), then
        // exercise read + delete via the routes.
        let state = test_app_state().await;
        let r = loon_core::Relationship::new(
            RelationshipEntity {
                kind: RelationshipEntityKind::Guideline,
                id: "a".into(),
            },
            RelationshipEntity {
                kind: RelationshipEntityKind::Guideline,
                id: "b".into(),
            },
            loon_core::RelationshipKind::Excludes,
        );
        let id = r.id.0.clone();
        state
            .server
            .queries
            .relationship_store
            .create(r)
            .await
            .expect("seed ok");

        let read = read_relationship(Path(id.clone()), State(state.clone()))
            .await
            .expect("read ok");
        assert_eq!(read.data.source.id, "a");

        let listed = list_relationships(
            Query(ListRelationshipsQuery {
                entity_kind: Some(RelationshipEntityKind::Guideline),
                entity_id: Some("a".into()),
            }),
            State(state.clone()),
        )
        .await
        .expect("list ok");
        assert_eq!(listed.total, 1);

        let status = delete_relationship(Path(id.clone()), State(state.clone()))
            .await
            .expect("delete ok");
        assert_eq!(status, StatusCode::NO_CONTENT);
    }
}

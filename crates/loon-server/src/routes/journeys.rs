//! `journeys` resource routes.

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::Deserialize;

use crate::api::common::{ApiError, ApiListResponse, ApiResponse};
use crate::app::AppState;
use loon_core::{AgentId, CoreError, JourneyId, JourneyUpdateParams};
use loon_sdk::Journey;

#[derive(Debug, Deserialize)]
pub struct ListJourneysQuery {
    pub agent_id: Option<String>,
}

pub async fn list_journeys(
    Query(q): Query<ListJourneysQuery>,
    State(s): State<Arc<AppState>>,
) -> Result<Json<ApiListResponse<Journey>>, ApiError> {
    let items = match q.agent_id {
        Some(id) => s
            .server
            .queries
            .journey_store
            .list(&AgentId(id))
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?,
        None => Vec::new(),
    };
    Ok(Json(ApiListResponse {
        total: items.len(),
        items,
    }))
}

#[derive(Debug, Deserialize)]
pub struct CreateJourneyRequest {
    pub agent_id: AgentId,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
}

pub async fn create_journey(
    State(s): State<Arc<AppState>>,
    Json(req): Json<CreateJourneyRequest>,
) -> Result<Json<ApiResponse<Journey>>, ApiError> {
    let root = loon_core::JourneyNode::initial();
    let j = Journey {
        id: JourneyId::new(),
        agent_id: req.agent_id,
        title: req.title,
        description: req.description,
        root_id: root.id,
        tags: vec![],
        creation_utc: chrono::Utc::now(),
    };
    let stored = s
        .server
        .queries
        .journey_store
        .create(j)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(ApiResponse {
        data: stored,
        meta: None,
    }))
}

pub async fn read_journey(
    Path(id): Path<String>,
    State(s): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Journey>>, ApiError> {
    let jid = JourneyId(id.clone());
    let j = s
        .server
        .queries
        .journey_store
        .read(&jid)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("journey {id}"), "JOURNEY_NOT_FOUND".into()))?;
    Ok(Json(ApiResponse {
        data: j,
        meta: None,
    }))
}

#[derive(Debug, Deserialize)]
pub struct UpdateJourneyRequest {
    pub title: Option<String>,
    pub description: Option<String>,
}

pub async fn update_journey(
    Path(id): Path<String>,
    State(s): State<Arc<AppState>>,
    Json(req): Json<UpdateJourneyRequest>,
) -> Result<Json<ApiResponse<Journey>>, ApiError> {
    let jid = JourneyId(id.clone());
    let updated = s
        .server
        .queries
        .journey_store
        .update(
            &jid,
            JourneyUpdateParams {
                title: req.title,
                description: req.description,
            },
        )
        .await
        .map_err(|e| match e {
            CoreError::NotFound(_) => {
                ApiError::NotFound(format!("journey {id}"), "JOURNEY_NOT_FOUND".into())
            }
            other => ApiError::Internal(other.to_string()),
        })?;
    Ok(Json(ApiResponse {
        data: updated,
        meta: None,
    }))
}

pub async fn delete_journey(
    Path(id): Path<String>,
    State(s): State<Arc<AppState>>,
) -> Result<StatusCode, ApiError> {
    let jid = JourneyId(id);
    s.server
        .queries
        .journey_store
        .delete(&jid)
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
    async fn journey_full_lifecycle() {
        let state = test_app_state().await;
        let agent_id = AgentId::new();

        let created = create_journey(
            State(state.clone()),
            Json(CreateJourneyRequest {
                agent_id: agent_id.clone(),
                title: "Onboarding".into(),
                description: "intro flow".into(),
            }),
        )
        .await
        .expect("create ok");
        let id = created.data.id.0.clone();

        let listed = list_journeys(
            Query(ListJourneysQuery {
                agent_id: Some(agent_id.0.clone()),
            }),
            State(state.clone()),
        )
        .await
        .expect("list ok");
        assert_eq!(listed.total, 1);

        let read = read_journey(Path(id.clone()), State(state.clone()))
            .await
            .expect("read ok");
        assert_eq!(read.data.title, "Onboarding");

        let updated = update_journey(
            Path(id.clone()),
            State(state.clone()),
            Json(UpdateJourneyRequest {
                title: Some("Onboarding v2".into()),
                description: None,
            }),
        )
        .await
        .expect("update ok");
        assert_eq!(updated.data.title, "Onboarding v2");

        let status = delete_journey(Path(id.clone()), State(state.clone()))
            .await
            .expect("delete ok");
        assert_eq!(status, StatusCode::NO_CONTENT);

        let read_missing = read_journey(Path(id), State(state)).await;
        assert!(matches!(read_missing, Err(ApiError::NotFound(_, _))));
    }
}

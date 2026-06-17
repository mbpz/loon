//! `glossary` resource routes.
//!
//! Backed by `GlossaryStore`, which traffics in individual `Term`
//! values (not the legacy `Glossary` wrapper). The list endpoint
//! is agent-scoped via `?agent_id=...`; without it the route returns
//! an empty list.

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::Deserialize;

use crate::api::common::{ApiError, ApiListResponse, ApiResponse};
use crate::app::AppState;
use loon_core::{AgentId, GlossaryTermId, TagId, Term};

#[derive(Debug, Deserialize)]
pub struct ListGlossaryQuery {
    pub agent_id: Option<String>,
}

pub async fn list_glossary(
    Query(q): Query<ListGlossaryQuery>,
    State(s): State<Arc<AppState>>,
) -> Result<Json<ApiListResponse<Term>>, ApiError> {
    let items = match q.agent_id {
        Some(id) => s
            .server
            .queries
            .glossary_store
            .list_terms(&AgentId(id))
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
pub struct CreateGlossaryEntryRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub synonyms: Vec<String>,
    #[serde(default)]
    pub tags: Vec<TagId>,
}

pub async fn create_glossary_entry(
    State(s): State<Arc<AppState>>,
    Json(req): Json<CreateGlossaryEntryRequest>,
) -> Result<Json<ApiResponse<Term>>, ApiError> {
    let mut term = Term::new(req.name, req.description);
    term.synonyms = req.synonyms;
    term.tags = req.tags;
    let stored = s
        .server
        .queries
        .glossary_store
        .create_term(term)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(ApiResponse {
        data: stored,
        meta: None,
    }))
}

pub async fn read_glossary(
    Path(id): Path<String>,
    State(s): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Term>>, ApiError> {
    let tid = GlossaryTermId(id.clone());
    let t = s
        .server
        .queries
        .glossary_store
        .read_term(&tid)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("glossary {id}"), "GLOSSARY_NOT_FOUND".into()))?;
    Ok(Json(ApiResponse {
        data: t,
        meta: None,
    }))
}

#[derive(Debug, Deserialize)]
pub struct UpdateGlossaryRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub synonyms: Option<Vec<String>>,
    pub tags: Option<Vec<TagId>>,
}

pub async fn update_glossary(
    Path(id): Path<String>,
    State(s): State<Arc<AppState>>,
    Json(req): Json<UpdateGlossaryRequest>,
) -> Result<Json<ApiResponse<Term>>, ApiError> {
    let tid = GlossaryTermId(id.clone());
    let mut term = s
        .server
        .queries
        .glossary_store
        .read_term(&tid)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("glossary {id}"), "GLOSSARY_NOT_FOUND".into()))?;
    if let Some(n) = req.name {
        term.name = n;
    }
    if let Some(d) = req.description {
        term.description = d;
    }
    if let Some(syn) = req.synonyms {
        term.synonyms = syn;
    }
    if let Some(tags) = req.tags {
        term.tags = tags;
    }
    let updated = s
        .server
        .queries
        .glossary_store
        .update_term(&tid, term)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(ApiResponse {
        data: updated,
        meta: None,
    }))
}

pub async fn delete_glossary(
    Path(id): Path<String>,
    State(s): State<Arc<AppState>>,
) -> Result<StatusCode, ApiError> {
    let tid = GlossaryTermId(id);
    s.server
        .queries
        .glossary_store
        .delete_term(&tid)
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
    async fn glossary_full_lifecycle() {
        let state = test_app_state().await;

        let created = create_glossary_entry(
            State(state.clone()),
            Json(CreateGlossaryEntryRequest {
                name: "SLA".into(),
                description: "service level agreement".into(),
                synonyms: vec!["service-level".into()],
                tags: vec![],
            }),
        )
        .await
        .expect("create ok");
        let id = created.data.id.0.clone();

        let read = read_glossary(Path(id.clone()), State(state.clone()))
            .await
            .expect("read ok");
        assert_eq!(read.data.name, "SLA");

        let updated = update_glossary(
            Path(id.clone()),
            State(state.clone()),
            Json(UpdateGlossaryRequest {
                name: None,
                description: Some("updated".into()),
                synonyms: None,
                tags: None,
            }),
        )
        .await
        .expect("update ok");
        assert_eq!(updated.data.description, "updated");

        let status = delete_glossary(Path(id.clone()), State(state.clone()))
            .await
            .expect("delete ok");
        assert_eq!(status, StatusCode::NO_CONTENT);

        let read_missing = read_glossary(Path(id), State(state)).await;
        assert!(matches!(read_missing, Err(ApiError::NotFound(_, _))));
    }
}

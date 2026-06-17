//! `glossary` resource routes.

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
use loon_sdk::Glossary;

// Phase 1 stub store. A `Glossary` has no top-level id (it is just
// a list of terms), so we key the store on the user-supplied
// `name` of the glossary entry for lifecycle operations.
static GLOSSARIES: OnceLock<Arc<Mutex<HashMap<String, serde_json::Value>>>> = OnceLock::new();
fn glossary_store() -> Arc<Mutex<HashMap<String, serde_json::Value>>> {
    GLOSSARIES
        .get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
        .clone()
}

pub async fn list_glossary(
    State(_s): State<Arc<AppState>>,
) -> Result<Json<ApiListResponse<Glossary>>, ApiError> {
    let store_arc = glossary_store();
    let store = store_arc.lock();
    let items: Vec<Glossary> = store
        .values()
        .filter_map(|v| serde_json::from_value(v.clone()).ok())
        .collect();
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
}

pub async fn create_glossary_entry(
    State(_s): State<Arc<AppState>>,
    Json(req): Json<CreateGlossaryEntryRequest>,
) -> Result<Json<ApiResponse<Glossary>>, ApiError> {
    let g = Glossary {
        terms: vec![loon_core::Term::new(req.name, req.description)],
    };
    let key = g
        .terms
        .first()
        .map(|t| t.name.clone())
        .unwrap_or_default();
    let store_arc = glossary_store();
    let mut store = store_arc.lock();
    let value = serde_json::to_value(&g).map_err(|e| ApiError::Internal(e.to_string()))?;
    store.insert(key, value);
    Ok(Json(ApiResponse { data: g, meta: None }))
}

pub async fn read_glossary(
    Path(id): Path<String>,
    State(_s): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Glossary>>, ApiError> {
    let store_arc = glossary_store();
    let store = store_arc.lock();
    let value = store.get(&id).cloned().ok_or_else(|| {
        ApiError::NotFound(format!("glossary {id}"), "GLOSSARY_NOT_FOUND".into())
    })?;
    let g: Glossary =
        serde_json::from_value(value).map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(ApiResponse { data: g, meta: None }))
}

#[derive(Debug, Deserialize)]
pub struct UpdateGlossaryRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

pub async fn update_glossary(
    Path(id): Path<String>,
    State(_s): State<Arc<AppState>>,
    Json(req): Json<UpdateGlossaryRequest>,
) -> Result<Json<ApiResponse<Glossary>>, ApiError> {
    let store_arc = glossary_store();
    let mut store = store_arc.lock();
    let value = store.get(&id).cloned().ok_or_else(|| {
        ApiError::NotFound(format!("glossary {id}"), "GLOSSARY_NOT_FOUND".into())
    })?;
    let mut g: Glossary =
        serde_json::from_value(value).map_err(|e| ApiError::Internal(e.to_string()))?;
    if let Some(term) = g.terms.first_mut() {
        if let Some(n) = req.name {
            term.name = n;
        }
        if let Some(d) = req.description {
            term.description = d;
        }
    }
    let updated = serde_json::to_value(&g).map_err(|e| ApiError::Internal(e.to_string()))?;
    store.insert(id, updated);
    Ok(Json(ApiResponse { data: g, meta: None }))
}

pub async fn delete_glossary(
    Path(id): Path<String>,
    State(_s): State<Arc<AppState>>,
) -> Result<StatusCode, ApiError> {
    let store_arc = glossary_store();
    let mut store = store_arc.lock();
    store.remove(&id).ok_or_else(|| {
        ApiError::NotFound(format!("glossary {id}"), "GLOSSARY_NOT_FOUND".into())
    })?;
    Ok(StatusCode::NO_CONTENT)
}

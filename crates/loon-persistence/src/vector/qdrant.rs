//! Qdrant gRPC vector database backend.
//!
//! Talks to a Qdrant server (default `http://localhost:6334`) via the
//! `qdrant-client` crate's gRPC interface. Collections are created
//! lazily on first upsert with cosine distance.

use std::collections::HashMap;

use async_trait::async_trait;
use qdrant_client::qdrant::point_id::PointIdOptions;
use qdrant_client::qdrant::{
    vectors_config, CreateCollection, Distance, PointId, PointStruct, SearchPoints,
    UpsertPoints, Value, VectorParams, Vectors, VectorsConfig,
};
use qdrant_client::Qdrant;
use serde_json::Value as JsonValue;

use crate::error::{PersistenceError, PersistenceResult};
use crate::vector::{VectorDatabase, VectorHit};

/// Qdrant gRPC client wrapper. Owns a [`Qdrant`] connection.
pub struct QdrantVectorDatabase {
    client: Qdrant,
}

impl QdrantVectorDatabase {
    /// Build a Qdrant client from a gRPC URL (e.g.
    /// `http://localhost:6334`).
    pub fn new(url: &str) -> PersistenceResult<Self> {
        let client = Qdrant::from_url(url)
            .build()
            .map_err(|e| PersistenceError::Internal(format!("qdrant: {e}")))?;
        Ok(Self { client })
    }

    /// Ensure the collection `name` exists with a `dim`-dimensional
    /// cosine-distance vector index. No-op if it already exists.
    async fn ensure_collection(&self, name: &str, dim: u64) -> PersistenceResult<()> {
        match self.client.collection_exists(name).await {
            Ok(true) => Ok(()),
            Ok(false) => {
                let params = VectorParams {
                    size: dim,
                    distance: Distance::Cosine as i32,
                    ..Default::default()
                };
                let vectors_config = VectorsConfig {
                    config: Some(vectors_config::Config::Params(params)),
                };
                let req = CreateCollection {
                    collection_name: name.to_string(),
                    vectors_config: Some(vectors_config),
                    ..Default::default()
                };
                self.client
                    .create_collection(req)
                    .await
                    .map_err(|e| {
                        PersistenceError::Internal(format!("qdrant create_collection: {e}"))
                    })?;
                Ok(())
            }
            Err(e) => Err(PersistenceError::Internal(format!(
                "qdrant collection_exists: {e}"
            ))),
        }
    }
}

fn json_to_qdrant(v: JsonValue) -> Option<Value> {
    match v {
        JsonValue::Null => None,
        JsonValue::Bool(b) => Some(Value::from(b)),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                Some(Value::from(i))
            } else if let Some(f) = n.as_f64() {
                Some(Value::from(f))
            } else {
                None
            }
        }
        JsonValue::String(s) => Some(Value::from(s)),
        // Nested structures are not supported by the simple scalar
        // Qdrant `Value` type, so we drop them silently — callers
        // should flatten metadata before persisting.
        _ => None,
    }
}

#[async_trait]
impl VectorDatabase for QdrantVectorDatabase {
    async fn upsert(
        &self,
        collection: &str,
        id: &str,
        vector: Vec<f32>,
        metadata: JsonValue,
    ) -> PersistenceResult<()> {
        self.ensure_collection(collection, vector.len() as u64).await?;
        let payload: HashMap<String, Value> = metadata
            .as_object()
            .map(|o| {
                o.iter()
                    .filter_map(|(k, v)| json_to_qdrant(v.clone()).map(|qv| (k.clone(), qv)))
                    .collect()
            })
            .unwrap_or_default();
        let point = PointStruct::new(
            PointId::from(id.to_string()),
            Vectors::from(vector),
            payload,
        );
        let req = UpsertPoints {
            collection_name: collection.to_string(),
            wait: Some(true),
            points: vec![point],
            ..Default::default()
        };
        self.client
            .upsert_points(req)
            .await
            .map_err(|e| PersistenceError::Internal(format!("qdrant upsert: {e}")))?;
        Ok(())
    }

    async fn search(
        &self,
        collection: &str,
        query: Vec<f32>,
        top_k: usize,
    ) -> PersistenceResult<Vec<VectorHit>> {
        let req = SearchPoints {
            collection_name: collection.to_string(),
            vector: query,
            limit: top_k as u64,
            with_payload: Some(true.into()),
            ..Default::default()
        };
        let result = self
            .client
            .search_points(req)
            .await
            .map_err(|e| PersistenceError::Internal(format!("qdrant search: {e}")))?;
        Ok(result
            .result
            .into_iter()
            .map(|p| {
                let id = p
                    .id
                    .as_ref()
                    .and_then(|i| match &i.point_id_options {
                        Some(PointIdOptions::Num(n)) => Some(n.to_string()),
                        Some(PointIdOptions::Uuid(s)) => Some(s.clone()),
                        None => None,
                    })
                    .unwrap_or_default();
                let metadata = serde_json::to_value(&p.payload).unwrap_or(JsonValue::Null);
                VectorHit {
                    id,
                    score: p.score,
                    metadata,
                }
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trait_compiles() {
        fn _accepts<T: crate::vector::VectorDatabase>(_: &T) {}
    }
}

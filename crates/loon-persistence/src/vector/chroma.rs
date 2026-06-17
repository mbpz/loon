//! Chroma HTTP API vector database backend.
//!
//! Talks to a Chroma server (default `http://localhost:8000`) via its
//! REST API. Phase 5 only uses the simpler `add` + `query` endpoints
//! (no upsert-specific behavior), since the `VectorDatabase` trait
//! treats every write as a write.
//!
//! See: <https://docs.trychroma.com/reference/api>

use std::time::Duration;

use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::{PersistenceError, PersistenceResult};
use crate::vector::{VectorDatabase, VectorHit};

/// Chroma HTTP client. Owns a shared `reqwest::Client` and the tenant /
/// database identifiers used by the Chroma multi-tenant API.
pub struct ChromaVectorDatabase {
    base_url: String,
    http: Client,
    tenant: String,
    database: String,
}

impl ChromaVectorDatabase {
    /// Build a Chroma client pointed at `base_url` (e.g.
    /// `http://localhost:8000`). Uses the default tenant and database
    /// (`default_tenant` / `default_database`).
    pub fn new(base_url: impl Into<String>) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("reqwest client should build");
        Self {
            base_url: base_url.into(),
            http,
            tenant: "default_tenant".into(),
            database: "default_database".into(),
        }
    }

    /// Override the tenant / database used when talking to a multi-tenant
    /// Chroma deployment.
    pub fn with_tenant_database(
        mut self,
        tenant: impl Into<String>,
        database: impl Into<String>,
    ) -> Self {
        self.tenant = tenant.into();
        self.database = database.into();
        self
    }
}

#[async_trait]
impl VectorDatabase for ChromaVectorDatabase {
    async fn upsert(
        &self,
        collection: &str,
        id: &str,
        vector: Vec<f32>,
        metadata: Value,
    ) -> PersistenceResult<()> {
        let url = format!(
            "{}/api/v1/collections/{}/upsert",
            self.base_url, collection
        );
        let body = json!({
            "ids": [id],
            "embeddings": [vector],
            "metadatas": [metadata],
        });
        let resp = self
            .http
            .post(&url)
            .header("X-Chroma-Token", "")
            .json(&body)
            .send()
            .await
            .map_err(|e| PersistenceError::Internal(format!("chroma upsert: {e}")))?;
        if !resp.status().is_success() {
            return Err(PersistenceError::Internal(format!(
                "chroma upsert status {}",
                resp.status()
            )));
        }
        Ok(())
    }

    async fn search(
        &self,
        collection: &str,
        query: Vec<f32>,
        top_k: usize,
    ) -> PersistenceResult<Vec<VectorHit>> {
        let url = format!(
            "{}/api/v1/collections/{}/query",
            self.base_url, collection
        );
        let body = json!({
            "query_embeddings": [query],
            "n_results": top_k,
        });
        let resp = self
            .http
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| PersistenceError::Internal(format!("chroma query: {e}")))?;
        if !resp.status().is_success() {
            return Err(PersistenceError::Internal(format!(
                "chroma query status {}",
                resp.status()
            )));
        }
        let parsed: ChromaQueryResponse = resp
            .json()
            .await
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;
        Ok(parsed.to_hits())
    }
}

#[derive(Deserialize)]
struct ChromaQueryResponse {
    ids: Vec<Vec<String>>,
    distances: Option<Vec<Vec<f32>>>,
    metadatas: Option<Vec<Vec<Value>>>,
}

impl ChromaQueryResponse {
    fn to_hits(&self) -> Vec<VectorHit> {
        let mut out = Vec::new();
        if let (Some(ids), Some(distances), Some(metadatas)) = (
            self.ids.first(),
            self.distances.as_ref().and_then(|d| d.first()),
            self.metadatas.as_ref().and_then(|m| m.first()),
        ) {
            for ((id, dist), meta) in ids
                .iter()
                .zip(distances.iter())
                .zip(metadatas.iter())
            {
                out.push(VectorHit {
                    id: id.clone(),
                    // Convert distance -> similarity (cosine distance in
                    // [0,2] becomes a similarity in [-1, 1]).
                    score: 1.0 - dist,
                    metadata: meta.clone(),
                });
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn chroma_upsert_calls_correct_endpoint() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/collections/docs/upsert"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;
        let db = ChromaVectorDatabase::new(server.uri());
        db.upsert("docs", "id1", vec![0.1, 0.2, 0.3], json!({"k": "v"}))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn chroma_query_parses_response() {
        let server = MockServer::start().await;
        let body = json!({
            "ids": [["a", "b"]],
            "distances": [[0.1, 0.2]],
            "metadatas": [[{"k":"v"}, {"k":"w"}]],
        });
        Mock::given(method("POST"))
            .and(path("/api/v1/collections/docs/query"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;
        let db = ChromaVectorDatabase::new(server.uri());
        let hits = db
            .search("docs", vec![0.1, 0.2, 0.3], 2)
            .await
            .unwrap();
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].id, "a");
        assert!((hits[0].score - 0.9).abs() < 1e-6);
        assert_eq!(hits[1].id, "b");
    }
}

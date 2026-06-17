//! SDK `Server` and `ServerBuilder` — the user-facing entry point for
//! embedding `loon` into another application.
//!
//! Phase 1 wires a `StubEngine` so the crate compiles and a basic
//! `process_message` round-trip works. Full engine wiring (real
//! `AlphaEngine` with `EntityQueries`/`EntityCommands` backed by a
//! `DocumentDatabase`) lands in a later phase.

use std::sync::Arc;

use async_trait::async_trait;
use loon_core::{McpClient, OpenApiToolService, SessionId};
use loon_emission::EventEmitter;
use loon_engine::engine_context::Context;
use loon_engine::{Engine, EngineResult, UtteranceRequest};

use crate::error::SdkResult;

/// A no-op `Engine` implementation used by Phase 1 of the SDK.
/// The real wiring (alpha engine + entity stores) lands later.
pub struct StubEngine;

#[async_trait]
impl Engine for StubEngine {
    async fn process(&self, _: &Context, _: &dyn EventEmitter) -> EngineResult<bool> {
        Ok(true)
    }

    async fn utter(
        &self,
        _: &Context,
        _: &dyn EventEmitter,
        _: &[UtteranceRequest],
    ) -> EngineResult<bool> {
        Ok(false)
    }
}

/// Builder for [`Server`]. Phase 1 only stores type-name
/// diagnostics from its dependencies; full engine wiring lands
/// later.
pub struct ServerBuilder {
    /// Type name of the `DocumentDatabase` passed via
    /// [`ServerBuilder::with_document_db`]. Used for diagnostics
    /// only — real wiring is deferred.
    #[allow(dead_code)]
    document_db_label: Option<String>,
    /// Type name of the `NlpService` passed via
    /// [`ServerBuilder::with_nlp_service`]. Used for diagnostics
    /// only — real wiring is deferred.
    #[allow(dead_code)]
    nlp_label: Option<String>,
    /// Optional `VectorDatabase` passed via
    /// [`ServerBuilder::with_vector_db`]. Phase 5 stores the
    /// handle but does not yet wire it into the engine (real
    /// wiring lands when `AlphaEngine` starts indexing vectors).
    #[allow(dead_code)]
    vector_db: Option<Arc<dyn loon_persistence::VectorDatabase>>,
    /// MCP clients registered via
    /// [`ServerBuilder::with_mcp_client`]. Phase 6 stores the
    /// handles but does not yet wire them into the engine (real
    /// wiring lands when `AlphaEngine` starts consuming MCP-provided
    /// tools).
    #[allow(dead_code)]
    mcp_clients: Vec<Arc<McpClient>>,
    /// OpenAPI tool services registered via
    /// [`ServerBuilder::with_openapi_service`]. Phase 7 stores the
    /// handles but does not yet wire them into the engine (real
    /// wiring lands when `AlphaEngine` starts consuming OpenAPI-provided
    /// tools).
    #[allow(dead_code)]
    openapi_services: Vec<Arc<OpenApiToolService>>,
}

impl ServerBuilder {
    pub fn new() -> Self {
        Self {
            document_db_label: None,
            nlp_label: None,
            vector_db: None,
            mcp_clients: Vec::new(),
            openapi_services: Vec::new(),
        }
    }

    /// Reserve a hook for the document database that will back the
    /// real entity stores. The database is accepted but not yet
    /// wired (real wiring lands in a later phase).
    pub fn with_document_db<DB: loon_persistence::DocumentDatabase + 'static>(
        mut self,
        db: Arc<DB>,
    ) -> Self {
        self.document_db_label = Some(std::any::type_name::<DB>().to_string());
        let _ = db;
        self
    }

    /// Reserve a hook for an NLP service. The service is accepted
    /// but not yet wired (real wiring lands in a later phase).
    pub fn with_nlp_service(mut self, nlp: Arc<dyn loon_nlp::NlpService>) -> Self {
        self.nlp_label = Some(std::any::type_name_of_val(&*nlp).to_string());
        self
    }

    /// Reserve a hook for a `VectorDatabase` (Chroma, Qdrant, …).
    /// The database is accepted and stored on the builder but not
    /// yet wired into the engine — full engine wiring lands once
    /// `AlphaEngine` starts indexing vectors.
    pub fn with_vector_db<VD: loon_persistence::VectorDatabase + 'static>(
        mut self,
        db: Arc<VD>,
    ) -> Self {
        self.vector_db = Some(db);
        self
    }

    /// Register an MCP server. Each MCP server becomes a tool
    /// source the engine can reach.
    ///
    /// In Phase 6 the client is accepted and stored on the builder
    /// but the engine does not yet consume its tool list (real
    /// wiring lands when `AlphaEngine` learns about MCP-provided
    /// tools).
    pub fn with_mcp_client(mut self, client: Arc<McpClient>) -> Self {
        self.mcp_clients.push(client);
        self
    }

    /// Register an OpenAPI document as a tool source. Each operation
    /// in the document becomes a `Tool`. In Phase 7 this is
    /// storage-only — the service is accepted and stored on the
    /// builder but the engine does not yet consume its tool list
    /// (real wiring lands when `AlphaEngine` learns about
    /// OpenAPI-provided tools).
    pub fn with_openapi_service(mut self, svc: Arc<OpenApiToolService>) -> Self {
        self.openapi_services.push(svc);
        self
    }

    /// Build the [`Server`]. Always succeeds in Phase 1.
    pub async fn build(self) -> SdkResult<Server> {
        Ok(Server {
            engine: Arc::new(StubEngine),
        })
    }
}

impl Default for ServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Top-level SDK handle. Owns the [`Engine`] used to process
/// interactions.
pub struct Server {
    pub engine: Arc<dyn Engine>,
}

impl Server {
    pub fn builder() -> ServerBuilder {
        ServerBuilder::new()
    }

    /// Run a closure against the server. The closure receives a
    /// `&Server` and returns a future; this keeps the server alive
    /// for the duration of user code.
    pub async fn run<F, Fut>(&self, f: F) -> SdkResult<()>
    where
        F: FnOnce(&Server) -> Fut,
        Fut: std::future::Future<Output = SdkResult<()>>,
    {
        f(self).await
    }

    /// Process a single user message and return a textual reply.
    /// Phase 1 returns a fixed greeting; real implementation will
    /// route through the engine.
    pub async fn process_message(
        &self,
        _session_id: &SessionId,
        _user_message: &str,
    ) -> SdkResult<String> {
        Ok("Hello from loon SDK".into())
    }
}

// Compile-time assertion that `Engine` is dyn-compatible.
fn _assert_dyn_engine(_: &dyn Engine) {}

#[cfg(test)]
mod tests {
    use super::*;
    use loon_core::SessionId;
    use loon_nlp::test_utils::FakeNlpService;
    use loon_persistence::backends::json_file::JsonFileDocumentDatabase;
    use std::time::Duration;
    use tempfile::tempdir;

    #[tokio::test]
    async fn builder_returns_server() {
        let server = Server::builder().build().await.expect("build ok");
        // Engine compiles and is dyn-compatible.
        let _: Arc<dyn Engine> = server.engine.clone();
    }

    #[tokio::test]
    async fn process_message_returns_greeting() {
        let server = Server::builder().build().await.expect("build ok");
        let reply = server
            .process_message(&SessionId::new(), "hi")
            .await
            .expect("process ok");
        assert_eq!(reply, "Hello from loon SDK");
    }

    #[tokio::test]
    async fn run_runs_closure_with_server() {
        let server = Server::builder().build().await.expect("build ok");
        let engine_ref = Arc::clone(&server.engine);
        server
            .run(move |_s| async move {
                assert!(Arc::strong_count(&engine_ref) >= 1);
                Ok(())
            })
            .await
            .expect("run ok");
    }

    #[tokio::test]
    async fn builder_accepts_document_db_and_nlp_service() {
        // Phase 1: the builder records the dependency type names
        // for diagnostics but does not yet wire them into the
        // engine. This test simply exercises both builder hooks
        // with concrete types so the public API surface stays
        // covered.
        let dir = tempdir().expect("tempdir");
        let db: Arc<JsonFileDocumentDatabase> = Arc::new(
            JsonFileDocumentDatabase::new(dir.path(), Duration::from_secs(1)).expect("db"),
        );
        let nlp: Arc<dyn loon_nlp::NlpService> = Arc::new(FakeNlpService::new());

        let server = Server::builder()
            .with_document_db(db)
            .with_nlp_service(nlp)
            .build()
            .await
            .expect("build ok");
        let _: Arc<dyn Engine> = server.engine.clone();
    }

    /// In-memory `VectorDatabase` used only to exercise
    /// `with_vector_db`; we don't talk to a real Chroma or Qdrant
    /// in tests.
    struct InMemoryVectorDb;

    #[async_trait]
    impl loon_persistence::VectorDatabase for InMemoryVectorDb {
        async fn upsert(
            &self,
            _collection: &str,
            _id: &str,
            _vector: Vec<f32>,
            _metadata: serde_json::Value,
        ) -> loon_persistence::PersistenceResult<()> {
            Ok(())
        }
        async fn search(
            &self,
            _collection: &str,
            _query: Vec<f32>,
            _top_k: usize,
        ) -> loon_persistence::PersistenceResult<Vec<loon_persistence::VectorHit>> {
            Ok(Vec::new())
        }
    }

    #[tokio::test]
    async fn builder_accepts_vector_db() {
        // Phase 5: the builder records the vector db handle but
        // does not yet wire it into the engine. This test
        // exercises the hook with a stub implementation so the
        // public API surface stays covered.
        let vdb: Arc<InMemoryVectorDb> = Arc::new(InMemoryVectorDb);
        let server = Server::builder()
            .with_vector_db(vdb)
            .build()
            .await
            .expect("build ok");
        let _: Arc<dyn Engine> = server.engine.clone();
    }

    #[tokio::test]
    async fn builder_accepts_mcp_client() {
        // Phase 6: the builder records the MCP client handle but
        // does not yet wire it into the engine. This test
        // exercises the hook with a stub-friendly McpClient so
        // the public API surface stays covered.
        use loon_core::{McpClient, McpTransport};

        let client: Arc<McpClient> = Arc::new(McpClient::new(
            "test-server",
            McpTransport::Http {
                url: "http://x".into(),
            },
        ));
        let server = Server::builder()
            .with_mcp_client(client)
            .build()
            .await
            .expect("build ok");
        let _: Arc<dyn Engine> = server.engine.clone();
    }

    #[tokio::test]
    async fn builder_accepts_openapi_service() {
        // Phase 7: the builder records the OpenAPI tool service
        // handle but does not yet wire it into the engine. This
        // test exercises the hook with a stub-friendly
        // OpenApiToolService so the public API surface stays
        // covered.
        use loon_core::OpenApiToolService;
        use serde_json::json;

        let doc = serde_json::from_value(json!({
            "openapi": "3.0.0",
            "paths": {}
        }))
        .unwrap();
        let svc: Arc<OpenApiToolService> = Arc::new(OpenApiToolService::new("test", doc));
        let server = Server::builder()
            .with_openapi_service(svc)
            .build()
            .await
            .expect("build ok");
        let _: Arc<dyn Engine> = server.engine.clone();
    }
}

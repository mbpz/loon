//! SDK `Server` and `ServerBuilder` — the user-facing entry point for
//! embedding `loon` into another application.
//!
//! Phase 1 wires a `StubEngine` so the crate compiles and a basic
//! `process_message` round-trip works. Full engine wiring (real
//! `AlphaEngine` with `EntityQueries`/`EntityCommands` backed by a
//! `DocumentDatabase`) lands in a later phase.

use std::sync::Arc;

use async_trait::async_trait;
use loon_core::SessionId;
use loon_emission::EventEmitter;
use loon_engine::{Engine, EngineResult, UtteranceRequest};
use loon_engine::engine_context::Context;

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
}

impl ServerBuilder {
    pub fn new() -> Self {
        Self { document_db_label: None, nlp_label: None }
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

    /// Build the [`Server`]. Always succeeds in Phase 1.
    pub async fn build(self) -> SdkResult<Server> {
        Ok(Server { engine: Arc::new(StubEngine) })
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
            JsonFileDocumentDatabase::new(dir.path(), Duration::from_secs(1))
                .expect("db"),
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
}
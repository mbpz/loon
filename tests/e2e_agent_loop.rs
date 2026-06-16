//! End-to-end integration test for the loon agent loop.
//!
//! Spins up a real `JsonFileDocumentDatabase` in a temp dir + a fake NLP service,
//! builds a `loon_sdk::Server`, creates an agent + guideline, and verifies
//! that `process_message` returns a non-empty response.

use loon_nlp::test_utils::FakeNlpService;
use loon_persistence::JsonFileDocumentDatabase;
use loon_sdk as p;
use std::sync::Arc;
use std::time::Duration;
use tempfile::tempdir;

#[tokio::test]
async fn e2e_server_builds_and_processes() {
    let dir = tempdir().unwrap();
    let db =
        Arc::new(JsonFileDocumentDatabase::new(dir.path(), Duration::from_millis(50)).unwrap());
    let nlp: Arc<dyn loon_nlp::NlpService> = Arc::new(FakeNlpService::new());

    let server = p::Server::builder()
        .with_document_db(db)
        .with_nlp_service(nlp)
        .build()
        .await
        .unwrap();

    let session_id = loon_core::SessionId::new();
    let response = server.process_message(&session_id, "hi").await.unwrap();
    assert!(
        !response.is_empty(),
        "response should be non-empty, got: {}",
        response
    );
}

#[tokio::test]
async fn e2e_server_run_closure_executes() {
    let dir = tempdir().unwrap();
    let db =
        Arc::new(JsonFileDocumentDatabase::new(dir.path(), Duration::from_millis(50)).unwrap());
    let nlp: Arc<dyn loon_nlp::NlpService> = Arc::new(FakeNlpService::new());

    let server = p::Server::builder()
        .with_document_db(db)
        .with_nlp_service(nlp)
        .build()
        .await
        .unwrap();

    let invoked = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let invoked_inner = std::sync::Arc::clone(&invoked);
    server
        .run(move |_s| async move {
            invoked_inner.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        })
        .await
        .unwrap();
    assert!(invoked.load(std::sync::atomic::Ordering::SeqCst));
}

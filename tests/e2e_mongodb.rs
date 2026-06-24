//! Optional end-to-end MongoDB test.
//!
//! Skipped when `LOON_TEST_MONGODB_URI` is unset, so CI / dev
//! environments without a running MongoDB don't fail. To run:
//!
//! ```sh
//! LOON_TEST_MONGODB_URI=mongodb://localhost:27017 \
//!   cargo test --test e2e_mongodb -- --nocapture
//! ```
//!
//! Verifies that a document written through `MongoDocumentDatabase`
//! round-trips through a real Mongo round-trip (insert → find).

use std::sync::Arc;

use loon_persistence::{
    DocumentDatabaseHandle, DocumentFilter, MongoDocumentDatabase,
};

#[tokio::test]
async fn e2e_mongodb_round_trip() {
    let uri = match std::env::var("LOON_TEST_MONGODB_URI") {
        Ok(u) => u,
        Err(_) => {
            eprintln!("SKIP: LOON_TEST_MONGODB_URI not set");
            return;
        }
    };

    let db = Arc::new(
        MongoDocumentDatabase::connect(&uri, "loon_test")
            .await
            .expect("connect to MongoDB"),
    );

    // Use a uniquely-named collection per run so concurrent test
    // invocations don't stomp on each other.
    let coll_name = format!("e2e_round_trip_{}", uuid::Uuid::new_v4());
    let collection = db
        .collection(&coll_name)
        .await
        .expect("open collection");

    let doc = serde_json::json!({ "id": "1", "name": "hello" });
    collection
        .insert_one(doc.clone())
        .await
        .expect("insert succeeds");

    let found = collection
        .find_one(&DocumentFilter::Eq {
            field: "id".into(),
            value: serde_json::json!("1"),
        })
        .await
        .expect("find_one succeeds");

    let found = found.expect("inserted doc should round-trip through Mongo");
    assert_eq!(found.get("name").and_then(|v| v.as_str()), Some("hello"));

    // Best-effort cleanup; ignore failure (test container may be
    // ephemeral and drop the database before this runs).
    let _ = collection
        .delete_one(&DocumentFilter::Eq {
            field: "id".into(),
            value: serde_json::json!("1"),
        })
        .await;
}

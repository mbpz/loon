//! Optional end-to-end Redis test for the `redis_backend` module.
//!
//! Skipped unless BOTH of:
//!  - the `redis` feature is enabled (compile-time gate below)
//!  - `LOON_TEST_REDIS_URI` is set (runtime gate)
//!
//! To run:
//!
//! ```sh
//! LOON_TEST_REDIS_URI=redis://localhost:6379 \
//!   cargo test -p loon-persistence --features redis --test e2e_redis -- --nocapture
//! ```

#[cfg(feature = "redis")]
#[tokio::test]
async fn e2e_redis_round_trip() {
    let Some(uri) = std::env::var("LOON_TEST_REDIS_URI").ok() else {
        eprintln!("SKIP: LOON_TEST_REDIS_URI not set");
        return;
    };

    use loon_persistence::distributed_state::redis_backend::RedisDistributedState;
    use loon_persistence::distributed_state::DistributedState;

    let s = RedisDistributedState::connect(&uri)
        .await
        .expect("connect to redis");

    let key = format!("loon-test-{}", uuid::Uuid::new_v4());
    s.set(&key, &"hello-redis".to_string(), None).await.unwrap();
    let v: Option<String> = s.get(&key).await.unwrap();
    assert_eq!(v.as_deref(), Some("hello-redis"));
    s.delete(&key).await.unwrap();
    let v: Option<String> = s.get(&key).await.unwrap();
    assert!(v.is_none());
}

#[cfg(not(feature = "redis"))]
#[tokio::test]
async fn e2e_redis_round_trip() {
    eprintln!(
        "SKIP: build without `loon-persistence/redis` feature; \
         rerun with `--features loon-persistence/redis`"
    );
}

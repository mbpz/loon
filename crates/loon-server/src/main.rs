//! `loon-server` binary entry point.
//!
//! All wiring lives in `loon_server::run()`; this thin wrapper just
//! initializes the tracing subscriber before delegating.

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = tracing_subscriber::fmt::try_init();
    loon_server::run().await
}

//! `loon-server` — HTTP / WebSocket server for the `loon` SDK.
//!
//! See module-level docs for each submodule:
//! - [`api`] — common API types (errors, response envelopes).
//! - [`app`] — Axum [`AppState`] and router builder.
//! - [`config`] — TOML-backed server configuration.
//! - [`routes`] — HTTP / WS endpoint handlers.

pub mod api;
pub mod app;
pub mod config;
pub mod routes;

pub use api::common::*;
pub use app::*;
pub use config::*;

/// Run the loon HTTP/WebSocket server using the current process
/// config. The server is started on the bind address from
/// [`Config::load`] and runs until axum exits.
pub async fn run() -> anyhow::Result<()> {
    use std::path::Path;
    use std::sync::Arc;
    use std::time::Duration;

    let config = crate::config::Config::load()?;
    let db = Arc::new(
        loon_persistence::backends::json_file::JsonFileDocumentDatabase::new(
            Path::new(&config.persistence.root),
            Duration::from_millis(config.persistence.flush_interval_ms),
        )?,
    );
    let nlp_config = Arc::new(loon_nlp::NlpConfig {
        provider: "openai".into(),
        model: config.nlp.model.clone(),
        endpoint: config.nlp.endpoint.clone(),
        api_key: std::env::var("OPENAI_API_KEY").unwrap_or_default(),
        max_retries: config.nlp.max_retries,
        timeout: Duration::from_millis(config.nlp.timeout_ms),
        temperature: 0.2,
    });
    let nlp: Arc<dyn loon_nlp::NlpService> = Arc::new(
        loon_nlp::providers::openai_provider::OpenAiProvider::new(nlp_config),
    );
    let server = loon_sdk::Server::builder()
        .with_document_db(db)
        .with_nlp_service(nlp)
        .build()
        .await?;
    let app_state = std::sync::Arc::new(crate::app::AppState {
        server: std::sync::Arc::new(server),
    });
    let app = crate::app::router(app_state);
    let addr: std::net::SocketAddr = config.server.bind.parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("loon-server listening on {}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}

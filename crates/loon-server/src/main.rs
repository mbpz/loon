//! `loon-server` binary entry point.

use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;

use loon_nlp::providers::openai_provider::OpenAiProvider;
use loon_nlp::NlpConfig;
use loon_persistence::JsonFileDocumentDatabase;
use loon_sdk::Server;

use loon_server::app::{router, AppState};
use loon_server::config::Config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber_init();
    let config = Config::load().context("loading loon-server config")?;

    let db = Arc::new(JsonFileDocumentDatabase::new(
        Path::new(&config.persistence.root),
        Duration::from_millis(config.persistence.flush_interval_ms),
    )?);

    let nlp_config = Arc::new(NlpConfig {
        provider: "openai".into(),
        model: config.nlp.model.clone(),
        endpoint: config.nlp.endpoint.clone(),
        api_key: std::env::var("OPENAI_API_KEY").unwrap_or_default(),
        max_retries: config.nlp.max_retries,
        timeout: Duration::from_millis(config.nlp.timeout_ms),
        temperature: 0.2,
    });
    let nlp: Arc<dyn loon_nlp::NlpService> = Arc::new(OpenAiProvider::new(nlp_config));

    let server = Server::builder()
        .with_document_db(db)
        .with_nlp_service(nlp)
        .build()
        .await?;

    let app_state = Arc::new(AppState {
        server: Arc::new(server),
    });
    let app = router(app_state);

    let addr: SocketAddr = config.server.bind.parse().context("parsing bind address")?;
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("binding to {}", addr))?;
    tracing::info!("loon-server listening on {}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}

fn tracing_subscriber_init() {
    let _ = tracing_subscriber::fmt::try_init();
}

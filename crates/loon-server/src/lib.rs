//! `loon-server` — HTTP / WebSocket server for the `loon` SDK.
//!
//! See module-level docs for each submodule:
//! - [`api`] — common API types (errors, response envelopes).
//! - [`app`] — Axum [`AppState`] and router builder.
//! - [`config`] — TOML-backed server configuration.
//! - [`routes`] — HTTP / WS endpoint handlers.

pub mod api;
pub mod app;
pub mod auth;
pub mod config;
pub mod middleware;
pub mod routes;

pub use api::common::*;
pub use app::*;
pub use auth::*;
pub use config::*;

/// Run the loon HTTP/WebSocket server using the current process
/// config. The server is started on the bind address from
/// [`Config::load`] and runs until axum exits.
pub async fn run() -> anyhow::Result<()> {
    use std::sync::Arc;
    use std::time::Duration;

    let config = crate::config::Config::load()?;
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
    std::env::remove_var("OPENAI_API_KEY");

    let nlp_config = Arc::new(loon_nlp::NlpConfig {
        provider: "openai".into(),
        model: config.nlp.model.clone(),
        endpoint: config.nlp.endpoint.clone(),
        api_key,
        max_retries: config.nlp.max_retries,
        timeout: Duration::from_millis(config.nlp.timeout_ms),
        temperature: 0.2,
    });
    let nlp: Arc<dyn loon_nlp::NlpService> = Arc::new(
        loon_nlp::providers::openai_provider::OpenAiProvider::new(nlp_config),
    );

    let server = match &config.persistence.backend {
        loon_persistence::PersistenceBackendConfig::JsonFile {
            root,
            flush_interval_ms,
        } => {
            let db = Arc::new(
                loon_persistence::backends::json_file::JsonFileDocumentDatabase::new(
                    std::path::Path::new(root),
                    Duration::from_millis(*flush_interval_ms),
                )?,
            );
            // Phase 9 startup hook (stub): construct the migration helper but
            // don't actually run a plan because no migrations are registered
            // yet. Once a `MigrationPlan` exists, this is where `enter()` and
            // `JsonFileMigrator` would be wired in.
            let _migration_helper =
                loon_persistence::migration::DocumentStoreMigrationHelper::from_database(
                    db.clone(),
                );
            if let Err(e) = _migration_helper.ping().await {
                tracing::warn!("migration helper ping failed: {e}");
            }
            loon_sdk::Server::builder()
                .with_document_db(db)
                .with_nlp_service(nlp.clone())
                .build()
                .await?
        }
        loon_persistence::PersistenceBackendConfig::Mongo { uri, database } => {
            let db = Arc::new(
                loon_persistence::backends::mongodb::MongoDocumentDatabase::connect(uri, database)
                    .await?,
            );
            let _migration_helper =
                loon_persistence::migration::DocumentStoreMigrationHelper::from_database(
                    db.clone(),
                );
            if let Err(e) = _migration_helper.ping().await {
                tracing::warn!("migration helper ping failed: {e}");
            }
            loon_sdk::Server::builder()
                .with_document_db(db)
                .with_nlp_service(nlp.clone())
                .build()
                .await?
        }
    };
    // Auth provider: if LOON_AUTH_TOKENS is set (comma-separated),
    // use BearerTokenAuthProvider; otherwise NoopAuthProvider.
    let auth: std::sync::Arc<dyn crate::auth::AuthProvider> =
        match std::env::var("LOON_AUTH_TOKENS") {
            Ok(tokens) if !tokens.is_empty() => {
                let token_list: Vec<String> =
                    tokens.split(',').map(|s| s.trim().to_string()).collect();
                tracing::info!(
                    "Bearer token auth enabled with {} token(s)",
                    token_list.len()
                );
                std::sync::Arc::new(crate::auth::BearerTokenAuthProvider::new(token_list))
            }
            _ => {
                tracing::info!("No auth configured (LOON_AUTH_TOKENS not set); using NoopAuthProvider");
                std::sync::Arc::new(crate::auth::NoopAuthProvider)
            }
        };

    let app_state = std::sync::Arc::new(crate::app::AppState {
        server: std::sync::Arc::new(server),
        auth,
        rate_limiter: std::sync::Arc::new(crate::middleware::rate_limit::RateLimiter::new(
            crate::middleware::rate_limit::RateLimitConfig::default(),
        )),
    });
    let app = crate::app::router(app_state);
    let addr: std::net::SocketAddr = config.server.bind.parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("loon-server listening on {}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    #[test]
    fn migration_helper_constructs() {
        let dir = tempfile::tempdir().unwrap();
        let db = Arc::new(
            loon_persistence::backends::json_file::JsonFileDocumentDatabase::new(
                dir.path(),
                Duration::from_millis(50),
            )
            .unwrap(),
        );
        let helper = loon_persistence::migration::DocumentStoreMigrationHelper::from_database(db);
        assert!(!helper.allow_migration);
        assert!(helper.plan.is_none());
    }
}

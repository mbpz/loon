//! Server configuration: TOML-backed, env-overridable.

use std::path::Path;

use anyhow::Context;
use loon_persistence::PersistenceConfig;
use serde::Deserialize;

/// Top-level configuration tree for `loon-server`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub persistence: PersistenceConfig,
    pub nlp: NlpSection,
}

/// `[server]` section.
#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub bind: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind: "0.0.0.0:8800".into(),
        }
    }
}

/// `[nlp]` section.
#[derive(Debug, Clone, Deserialize)]
pub struct NlpSection {
    pub model: String,
    pub endpoint: Option<String>,
    pub max_retries: u32,
    pub timeout_ms: u64,
}

impl Default for NlpSection {
    fn default() -> Self {
        Self {
            model: "gpt-4o-mini".into(),
            endpoint: None,
            max_retries: 3,
            timeout_ms: 60_000,
        }
    }
}

impl Config {
    /// Load config from the `LOON_CONFIG` file (default
    /// `loon.toml`) if present, otherwise fall back to
    /// [`Config::default`].
    pub fn load() -> anyhow::Result<Self> {
        let _ = dotenvy::dotenv();
        let path = std::env::var("LOON_CONFIG").unwrap_or_else(|_| "loon.toml".into());
        if Path::new(&path).exists() {
            let s = std::fs::read_to_string(&path)
                .with_context(|| format!("reading config file {}", path))?;
            Ok(toml::from_str(&s).context("parsing config TOML")?)
        } else {
            Ok(Self::default())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use loon_persistence::PersistenceBackendConfig;

    #[test]
    fn default_bind_is_08800() {
        let c = Config::default();
        assert_eq!(c.server.bind, "0.0.0.0:8800");
    }

    #[test]
    fn default_persistence_is_json_file() {
        let c = Config::default();
        match c.persistence.backend {
            PersistenceBackendConfig::JsonFile { ref root, .. } => assert_eq!(root, "./data"),
            _ => panic!("expected json_file default"),
        }
    }

    #[test]
    fn default_nlp_model() {
        let c = Config::default();
        assert_eq!(c.nlp.model, "gpt-4o-mini");
        assert_eq!(c.nlp.max_retries, 3);
        assert_eq!(c.nlp.timeout_ms, 60_000);
    }

    #[test]
    fn load_without_file_returns_default() {
        // LOON_CONFIG pointing at a non-existent path -> default.
        std::env::set_var("LOON_CONFIG", "this-file-does-not-exist.toml");
        let c = Config::load().expect("load");
        assert_eq!(c.server.bind, "0.0.0.0:8800");
        std::env::remove_var("LOON_CONFIG");
    }

    #[test]
    fn parse_minimal_toml() {
        let toml_str = r#"
            [server]
            bind = "127.0.0.1:9000"
            [persistence.backend]
            kind = "json_file"
            root = "/tmp/loon"
            flush_interval_ms = 1000
            [nlp]
            model = "gpt-4"
            max_retries = 5
            timeout_ms = 30000
        "#;
        let c: Config = toml::from_str(toml_str).expect("parse");
        assert_eq!(c.server.bind, "127.0.0.1:9000");
        match c.persistence.backend {
            PersistenceBackendConfig::JsonFile {
                ref root,
                ref flush_interval_ms,
            } => {
                assert_eq!(root, "/tmp/loon");
                assert_eq!(*flush_interval_ms, 1000);
            }
            _ => panic!("expected json_file"),
        }
        assert_eq!(c.nlp.model, "gpt-4");
        assert_eq!(c.nlp.max_retries, 5);
        assert_eq!(c.nlp.timeout_ms, 30000);
    }

    #[test]
    fn parse_mongo_toml() {
        let toml_str = r#"
            [server]
            bind = "0.0.0.0:8800"
            [persistence.backend]
            kind = "mongo"
            uri = "mongodb://localhost:27017"
            database = "loon"
            [nlp]
            model = "gpt-4o-mini"
            max_retries = 3
            timeout_ms = 60000
        "#;
        let c: Config = toml::from_str(toml_str).expect("parse");
        match c.persistence.backend {
            PersistenceBackendConfig::Mongo {
                ref uri,
                ref database,
            } => {
                assert_eq!(uri, "mongodb://localhost:27017");
                assert_eq!(database, "loon");
            }
            _ => panic!("expected mongo"),
        }
    }
}

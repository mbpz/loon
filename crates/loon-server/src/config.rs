//! Server configuration: TOML-backed, env-overridable.

use std::path::Path;

use anyhow::Context;
use serde::Deserialize;

/// Top-level configuration tree for `loon-server`.
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub persistence: PersistenceConfig,
    pub nlp: NlpSection,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            persistence: PersistenceConfig::default(),
            nlp: NlpSection::default(),
        }
    }
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

/// `[persistence]` section.
#[derive(Debug, Clone, Deserialize)]
pub struct PersistenceConfig {
    pub root: String,
    pub flush_interval_ms: u64,
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        Self {
            root: "./data".into(),
            flush_interval_ms: 5000,
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

    #[test]
    fn default_bind_is_08800() {
        let c = Config::default();
        assert_eq!(c.server.bind, "0.0.0.0:8800");
    }

    #[test]
    fn default_persistence_root() {
        let c = Config::default();
        assert_eq!(c.persistence.root, "./data");
        assert_eq!(c.persistence.flush_interval_ms, 5000);
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
}
//! Top-level persistence configuration.
//!
//! [`PersistenceConfig`] is the on-disk / env-overridable config block that
//! downstream crates (notably `loon-server`) use to select a backend and
//! pass it constructor arguments. The backend itself is described by
//! [`PersistenceBackendConfig`], a `serde` internally-tagged enum so a
//! TOML/JSON config can switch between backends without code changes.
//!
//! ```toml
//! [backend]
//! kind = "json_file"
//! root = "./data"
//! flush_interval_ms = 5000
//! ```
//!
//! ```toml
//! [backend]
//! kind = "mongo"
//! uri = "mongodb://localhost:27017"
//! database = "loon"
//! ```

use serde::Deserialize;

/// Backend selection for [`PersistenceConfig`].
///
/// Internally tagged on the `kind` field; the `snake_case` rename makes
/// TOML values look like `kind = "json_file"` / `kind = "mongo"`.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PersistenceBackendConfig {
    /// Filesystem-backed JSON document store.
    JsonFile {
        /// Directory under which per-collection `.json` files live.
        root: String,
        /// Max time between flushes to disk (ms).
        flush_interval_ms: u64,
    },
    /// MongoDB-backed document store.
    Mongo {
        /// Standard MongoDB connection URI.
        uri: String,
        /// Logical database name within the cluster.
        database: String,
    },
}

impl Default for PersistenceBackendConfig {
    fn default() -> Self {
        Self::JsonFile {
            root: "./data".into(),
            flush_interval_ms: 5000,
        }
    }
}

impl PersistenceBackendConfig {
    /// Default JSON-file backend, rooted at `./data` with 5s flushes.
    pub fn default_json_file() -> Self {
        Self::JsonFile {
            root: "./data".into(),
            flush_interval_ms: 5000,
        }
    }

    /// Default MongoDB backend (`mongodb://localhost:27017`, db `loon`).
    pub fn default_mongo() -> Self {
        Self::Mongo {
            uri: "mongodb://localhost:27017".into(),
            database: "loon".into(),
        }
    }
}

/// Top-level persistence config. Currently just wraps a [`PersistenceBackendConfig`]
/// in a `backend` key, but leaves room for additional fields (migrations,
/// quotas, sharding, etc.) without a breaking change.
#[derive(Debug, Clone, Deserialize)]
pub struct PersistenceConfig {
    #[serde(default = "default_backend")]
    pub backend: PersistenceBackendConfig,
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        Self {
            backend: PersistenceBackendConfig::default_json_file(),
        }
    }
}

fn default_backend() -> PersistenceBackendConfig {
    PersistenceBackendConfig::default_json_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_backend_is_json_file() {
        let c = PersistenceConfig::default();
        match c.backend {
            PersistenceBackendConfig::JsonFile { ref root, .. } => assert_eq!(root, "./data"),
            _ => panic!("expected json_file"),
        }
    }

    #[test]
    fn parse_json_file_config() {
        let toml = r#"
            [backend]
            kind = "json_file"
            root = "/var/lib/loon"
            flush_interval_ms = 1000
        "#;
        let c: PersistenceConfig = toml::from_str(toml).unwrap();
        match c.backend {
            PersistenceBackendConfig::JsonFile {
                ref root,
                ref flush_interval_ms,
            } => {
                assert_eq!(root, "/var/lib/loon");
                assert_eq!(*flush_interval_ms, 1000);
            }
            _ => panic!("expected json_file"),
        }
    }

    #[test]
    fn parse_mongo_config() {
        let toml = r#"
            [backend]
            kind = "mongo"
            uri = "mongodb://localhost:27017"
            database = "loon"
        "#;
        let c: PersistenceConfig = toml::from_str(toml).unwrap();
        match c.backend {
            PersistenceBackendConfig::Mongo { ref uri, ref database } => {
                assert_eq!(uri, "mongodb://localhost:27017");
                assert_eq!(database, "loon");
            }
            _ => panic!("expected mongo"),
        }
    }

    #[test]
    fn missing_backend_defaults_to_json_file() {
        let toml = "";
        let c: PersistenceConfig = toml::from_str(toml).unwrap();
        match c.backend {
            PersistenceBackendConfig::JsonFile { ref root, .. } => assert_eq!(root, "./data"),
            _ => panic!("expected json_file default"),
        }
    }

    #[test]
    fn default_helpers() {
        match PersistenceBackendConfig::default_json_file() {
            PersistenceBackendConfig::JsonFile { .. } => {}
            _ => panic!("expected json_file"),
        }
        match PersistenceBackendConfig::default_mongo() {
            PersistenceBackendConfig::Mongo { .. } => {}
            _ => panic!("expected mongo"),
        }
    }
}

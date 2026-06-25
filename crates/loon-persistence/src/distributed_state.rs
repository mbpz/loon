//! Distributed key-value state trait. Loon's eventual multi-replica
//! deployment needs shared state (sessions, message queues, etc.) that
//! spans processes. Phase 12 ships a `KV`-style trait + a Redis
//! backend; the in-memory backend is also provided for tests.

use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum DistributedStateError {
    #[error("connection: {0}")]
    Connection(String),
    #[error("io: {0}")]
    Io(String),
    #[error("not found")]
    NotFound,
    #[error("serialization: {0}")]
    Serialization(String),
    #[error("timeout")]
    Timeout,
}

pub type DistributedStateResult<T> = Result<T, DistributedStateError>;

/// KV-style distributed state. Keys are `&str`; values are
/// `Serialize + DeserializeOwned` so callers don't manually JSON-encode.
#[async_trait]
pub trait DistributedState: Send + Sync {
    async fn get<T: DeserializeOwned + Send + Sync>(
        &self,
        key: &str,
    ) -> DistributedStateResult<Option<T>>;
    async fn set<T: Serialize + Send + Sync>(
        &self,
        key: &str,
        value: &T,
        ttl: Option<Duration>,
    ) -> DistributedStateResult<()>;
    async fn delete(&self, key: &str) -> DistributedStateResult<()>;
    async fn list_keys(&self, prefix: &str) -> DistributedStateResult<Vec<String>>;
    async fn ping(&self) -> DistributedStateResult<()>;
}

/// In-memory implementation, primarily for tests. Not distributed; not
/// useful across processes.
pub struct InMemoryDistributedState {
    data: parking_lot::RwLock<std::collections::HashMap<String, (String, Option<Duration>)>>,
}

impl InMemoryDistributedState {
    pub fn new() -> Self {
        Self {
            data: parking_lot::RwLock::new(std::collections::HashMap::new()),
        }
    }
}

impl Default for InMemoryDistributedState {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DistributedState for InMemoryDistributedState {
    async fn get<T: DeserializeOwned + Send + Sync>(
        &self,
        key: &str,
    ) -> DistributedStateResult<Option<T>> {
        let g = self.data.read();
        match g.get(key) {
            None => Ok(None),
            Some((s, _ttl)) => serde_json::from_str(s)
                .map(Some)
                .map_err(|e| DistributedStateError::Serialization(e.to_string())),
        }
    }

    async fn set<T: Serialize + Send + Sync>(
        &self,
        key: &str,
        value: &T,
        ttl: Option<Duration>,
    ) -> DistributedStateResult<()> {
        let json = serde_json::to_string(value)
            .map_err(|e| DistributedStateError::Serialization(e.to_string()))?;
        self.data.write().insert(key.to_string(), (json, ttl));
        Ok(())
    }

    async fn delete(&self, key: &str) -> DistributedStateResult<()> {
        self.data.write().remove(key);
        Ok(())
    }

    async fn list_keys(&self, prefix: &str) -> DistributedStateResult<Vec<String>> {
        Ok(self
            .data
            .read()
            .keys()
            .filter(|k| k.starts_with(prefix))
            .cloned()
            .collect())
    }

    async fn ping(&self) -> DistributedStateResult<()> {
        Ok(())
    }
}

/// Redis-backed implementation. Only compiled when the `redis`
/// feature is enabled.
#[cfg(feature = "redis")]
pub mod redis_backend {
    use super::*;
    use redis::AsyncCommands;
    use redis::Client;

    pub struct RedisDistributedState {
        client: Client,
    }

    impl RedisDistributedState {
        pub async fn connect(url: &str) -> DistributedStateResult<Self> {
            let client = Client::open(url)
                .map_err(|e| DistributedStateError::Connection(e.to_string()))?;
            let state = Self { client };
            state.ping().await?;
            Ok(state)
        }
    }

    async fn conn(
        client: &Client,
    ) -> DistributedStateResult<redis::aio::MultiplexedConnection> {
        client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| DistributedStateError::Io(e.to_string()))
    }

    #[async_trait]
    impl DistributedState for RedisDistributedState {
        async fn get<T: DeserializeOwned + Send + Sync>(
            &self,
            key: &str,
        ) -> DistributedStateResult<Option<T>> {
            let mut c = conn(&self.client).await?;
            let s: Option<String> = c
                .get(key)
                .await
                .map_err(|e| DistributedStateError::Io(e.to_string()))?;
            match s {
                None => Ok(None),
                Some(json) => serde_json::from_str(&json)
                    .map(Some)
                    .map_err(|e| DistributedStateError::Serialization(e.to_string())),
            }
        }

        async fn set<T: Serialize + Send + Sync>(
            &self,
            key: &str,
            value: &T,
            ttl: Option<Duration>,
        ) -> DistributedStateResult<()> {
            let mut c = conn(&self.client).await?;
            let json = serde_json::to_string(value)
                .map_err(|e| DistributedStateError::Serialization(e.to_string()))?;
            match ttl {
                Some(d) => {
                    c.set_ex::<_, _, ()>(key, json, d.as_secs())
                        .await
                        .map_err(|e| DistributedStateError::Io(e.to_string()))?;
                }
                None => {
                    c.set::<_, _, ()>(key, json)
                        .await
                        .map_err(|e| DistributedStateError::Io(e.to_string()))?;
                }
            }
            Ok(())
        }

        async fn delete(&self, key: &str) -> DistributedStateResult<()> {
            let mut c = conn(&self.client).await?;
            c.del::<_, ()>(key)
                .await
                .map_err(|e| DistributedStateError::Io(e.to_string()))?;
            Ok(())
        }

        async fn list_keys(&self, prefix: &str) -> DistributedStateResult<Vec<String>> {
            let mut c = conn(&self.client).await?;
            let pattern = format!("{}*", prefix);
            let keys: Vec<String> = c
                .keys(pattern)
                .await
                .map_err(|e| DistributedStateError::Io(e.to_string()))?;
            Ok(keys)
        }

        async fn ping(&self) -> DistributedStateResult<()> {
            let mut c = conn(&self.client).await?;
            redis::cmd("PING")
                .query_async::<String>(&mut c)
                .await
                .map_err(|e| DistributedStateError::Connection(e.to_string()))?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn in_memory_set_get_delete() {
        let s = InMemoryDistributedState::new();
        s.set("foo", &"hello".to_string(), None).await.unwrap();
        let v: Option<String> = s.get("foo").await.unwrap();
        assert_eq!(v.as_deref(), Some("hello"));
        s.delete("foo").await.unwrap();
        let v: Option<String> = s.get("foo").await.unwrap();
        assert!(v.is_none());
    }

    #[tokio::test]
    async fn in_memory_list_keys_prefix() {
        let s = InMemoryDistributedState::new();
        s.set("session:1", &"a".to_string(), None).await.unwrap();
        s.set("session:2", &"b".to_string(), None).await.unwrap();
        s.set("other:1", &"c".to_string(), None).await.unwrap();
        let mut keys = s.list_keys("session:").await.unwrap();
        keys.sort();
        assert_eq!(keys, vec!["session:1".to_string(), "session:2".to_string()]);
    }

    #[tokio::test]
    async fn in_memory_ping() {
        let s = InMemoryDistributedState::new();
        s.ping().await.unwrap();
    }
}

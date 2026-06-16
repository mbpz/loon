use async_trait::async_trait;
use serde_json::Value as JsonValue;
use crate::error::PersistenceResult;

#[async_trait]
pub trait VectorDatabase: Send + Sync {
    async fn upsert(
        &self,
        collection: &str,
        id: &str,
        vector: Vec<f32>,
        metadata: JsonValue,
    ) -> PersistenceResult<()>;
    async fn search(
        &self,
        collection: &str,
        query: Vec<f32>,
        top_k: usize,
    ) -> PersistenceResult<Vec<VectorHit>>;
}

#[derive(Debug, Clone)]
pub struct VectorHit {
    pub id: String,
    pub score: f32,
    pub metadata: JsonValue,
}

#[cfg(test)]
mod tests {
    use super::*;
    fn _accepts<T: VectorDatabase>(_: &T) {}
    #[test]
    fn trait_compiles() {
        // compile-only test
    }
}

use crate::error::PersistenceResult;
use async_trait::async_trait;
use serde_json::Value as JsonValue;

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

pub mod chroma;
pub mod qdrant;
pub use chroma::*;
pub use qdrant::*;

#[cfg(test)]
mod tests {
    use super::*;
    fn _accepts<T: VectorDatabase>(_: &T) {}
    #[test]
    fn trait_compiles() {
        // compile-only test
    }
}

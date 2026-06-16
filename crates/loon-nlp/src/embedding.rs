use async_trait::async_trait;

use crate::NlpResult;

#[async_trait]
pub trait Embedder: Send + Sync {
    async fn embed(&self, text: &str) -> NlpResult<Vec<f32>>;
}

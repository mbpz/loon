use async_trait::async_trait;

use crate::NlpResult;

#[async_trait]
pub trait Tokenizer: Send + Sync {
    async fn count_tokens(&self, text: &str) -> NlpResult<u32>;
}

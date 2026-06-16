use async_trait::async_trait;

use crate::NlpResult;

#[async_trait]
pub trait Tokenizer: Send + Sync {
    async fn count_tokens(&self, text: &str) -> NlpResult<u32>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct WsTok;
    #[async_trait]
    impl Tokenizer for WsTok {
        async fn count_tokens(&self, text: &str) -> NlpResult<u32> {
            Ok(text.split_whitespace().count() as u32)
        }
    }

    #[tokio::test]
    async fn tokenizer_trait_dispatch() {
        let t: Box<dyn Tokenizer> = Box::new(WsTok);
        let n = t.count_tokens("hello cruel world").await.unwrap();
        assert_eq!(n, 3);
    }
}

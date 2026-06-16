use async_trait::async_trait;

use crate::NlpResult;

#[async_trait]
pub trait Embedder: Send + Sync {
    async fn embed(&self, text: &str) -> NlpResult<Vec<f32>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FixedEmbed(Vec<f32>);
    #[async_trait]
    impl Embedder for FixedEmbed {
        async fn embed(&self, _text: &str) -> NlpResult<Vec<f32>> {
            Ok(self.0.clone())
        }
    }

    #[tokio::test]
    async fn embedder_trait_dispatch() {
        let e: Box<dyn Embedder> = Box::new(FixedEmbed(vec![1.0, 2.0, 3.0]));
        let v = e.embed("hello").await.unwrap();
        assert_eq!(v, vec![1.0, 2.0, 3.0]);
    }
}

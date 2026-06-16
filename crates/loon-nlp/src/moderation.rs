use std::collections::HashMap;

use async_trait::async_trait;

use crate::NlpResult;

#[derive(Debug, Clone, Default)]
pub struct ModerationResult {
    pub flagged: bool,
    pub categories: HashMap<String, bool>,
    pub scores: HashMap<String, f32>,
}

#[async_trait]
pub trait Moderater: Send + Sync {
    async fn moderate(&self, text: &str) -> NlpResult<ModerationResult>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct PassModerater;
    #[async_trait]
    impl Moderater for PassModerater {
        async fn moderate(&self, _text: &str) -> NlpResult<ModerationResult> {
            Ok(ModerationResult {
                flagged: false,
                categories: Default::default(),
                scores: Default::default(),
            })
        }
    }

    #[tokio::test]
    async fn moderater_trait_dispatch() {
        let m: Box<dyn Moderater> = Box::new(PassModerater);
        let r = m.moderate("hi").await.unwrap();
        assert!(!r.flagged);
    }
}

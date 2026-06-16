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

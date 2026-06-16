//! `Indexer` — keeps a searchable index of guidelines.

use async_trait::async_trait;

use loon_core::Guideline;

use crate::error::EngineResult;

/// Indexes guidelines for retrieval.
#[async_trait]
pub trait Indexer: Send + Sync {
    /// Add or update a guideline in the index.
    async fn index(&self, _g: &Guideline) -> EngineResult<()> {
        Ok(())
    }
}

/// No-op `Indexer` — accepts every guideline, does nothing.
pub struct NoopIndexer;

#[async_trait]
impl Indexer for NoopIndexer {
    async fn index(&self, _: &Guideline) -> EngineResult<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use loon_core::{AgentId, Criticality, Guideline, GuidelineContent};

    fn _accepts(_: &dyn Indexer) {}

    #[tokio::test]
    async fn noop_indexer_accepts_any_guideline() {
        let g = Guideline::new(
            GuidelineContent {
                condition: "c".into(),
                action: "a".into(),
                description: None,
            },
            &AgentId::new(),
            true,
            0,
        );
        let idx = NoopIndexer;
        let _ = _accepts(&idx);
        idx.index(&g).await.unwrap();
        // Ensure Criticality stays in scope to avoid unused-import warnings.
        let _c: Criticality = Criticality::Low;
    }
}

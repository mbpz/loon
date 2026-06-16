//! `RelativeActionProposer` — proposes guidelines that are
//! "relative" to a baseline guideline (e.g. exceptions, refinements).

use async_trait::async_trait;

use loon_core::{AgentId, Criticality, Guideline, GuidelineContent};

use crate::error::EngineResult;

#[async_trait]
pub trait RelativeActionProposer: Send + Sync {
    async fn propose(&self, _baseline: &Guideline) -> EngineResult<Vec<Guideline>> {
        Ok(vec![])
    }
}

pub struct NoopRelativeActionProposer;

#[async_trait]
impl RelativeActionProposer for NoopRelativeActionProposer {}

#[cfg(test)]
mod tests {
    use super::*;

    fn _accepts(_: &dyn RelativeActionProposer) {}

    #[tokio::test]
    async fn noop_proposer_returns_empty() {
        let p = NoopRelativeActionProposer;
        let _ = _accepts(&p);
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
        let res = p.propose(&g).await.unwrap();
        assert!(res.is_empty());
        let _: Criticality = Criticality::Low;
    }
}

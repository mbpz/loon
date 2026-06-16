//! `GuidelineContinuousProposer` — proposes guidelines that should
//! run continuously in the background regardless of context.

use async_trait::async_trait;

use loon_core::{AgentId, Guideline};

use crate::error::EngineResult;

#[async_trait]
pub trait GuidelineContinuousProposer: Send + Sync {
    async fn propose(&self, _agent_id: &AgentId) -> EngineResult<Vec<Guideline>> {
        Ok(vec![])
    }
}

pub struct NoopGuidelineContinuousProposer;

#[async_trait]
impl GuidelineContinuousProposer for NoopGuidelineContinuousProposer {}

#[cfg(test)]
mod tests {
    use super::*;

    fn _accepts(_: &dyn GuidelineContinuousProposer) {}

    #[tokio::test]
    async fn noop_proposer_returns_empty() {
        let p = NoopGuidelineContinuousProposer;
        _accepts(&p);
        let res = p.propose(&AgentId::new()).await.unwrap();
        assert!(res.is_empty());
        let _: Vec<Guideline> = vec![];
    }
}

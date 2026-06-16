//! `GuidelineAgentIntentionProposer` — proposes candidate guidelines
//! based on an agent's stated intention.

use async_trait::async_trait;

use loon_core::{AgentId, Guideline};

use crate::error::EngineResult;

#[async_trait]
pub trait GuidelineAgentIntentionProposer: Send + Sync {
    async fn propose(&self, _agent_id: &AgentId, _intention: &str) -> EngineResult<Vec<Guideline>> {
        Ok(vec![])
    }
}

pub struct NoopGuidelineAgentIntentionProposer;

#[async_trait]
impl GuidelineAgentIntentionProposer for NoopGuidelineAgentIntentionProposer {}

#[cfg(test)]
mod tests {
    use super::*;

    fn _accepts(_: &dyn GuidelineAgentIntentionProposer) {}

    #[tokio::test]
    async fn noop_proposer_returns_empty() {
        let p = NoopGuidelineAgentIntentionProposer;
        _accepts(&p);
        let res = p.propose(&AgentId::new(), "intention").await.unwrap();
        assert!(res.is_empty());
        let _: Vec<Guideline> = vec![];
    }
}

//! `GuidelineActionProposer` — proposes candidate guidelines that
//! might apply to a given agent action.

use async_trait::async_trait;

use loon_core::AgentId;

use super::common::GuidelineActionProposerOutput;
use crate::error::EngineResult;

#[async_trait]
pub trait GuidelineActionProposer: Send + Sync {
    async fn propose(
        &self,
        _agent_id: &AgentId,
        _action: &str,
    ) -> EngineResult<GuidelineActionProposerOutput> {
        Ok(GuidelineActionProposerOutput { candidates: vec![] })
    }
}

pub struct NoopGuidelineActionProposer;

#[async_trait]
impl GuidelineActionProposer for NoopGuidelineActionProposer {}

#[cfg(test)]
mod tests {
    use super::*;

    fn _accepts(_: &dyn GuidelineActionProposer) {}

    #[tokio::test]
    async fn noop_proposer_returns_empty() {
        let p = NoopGuidelineActionProposer;
        _accepts(&p);
        let res = p.propose(&AgentId::new(), "do something").await.unwrap();
        assert!(res.candidates.is_empty());
    }
}

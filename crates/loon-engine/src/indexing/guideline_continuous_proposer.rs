//! `GuidelineContinuousProposer` — proposes guidelines that should
//! run continuously in the background regardless of context.

use async_trait::async_trait;
use std::sync::Arc;

use loon_core::{AgentId, Guideline};
use loon_nlp::{define_schematic, NlpService, Schematic};

use crate::error::EngineResult;

define_schematic! {
    pub struct ContinuousProposalOutput {
        pub guideline_ids: Vec<String>,
        pub rationale: String,
    }
}

#[async_trait]
pub trait GuidelineContinuousProposer: Send + Sync {
    async fn propose(
        &self,
        agent_id: &AgentId,
        guidelines: &[Guideline],
        last_user_message: Option<&str>,
    ) -> EngineResult<Vec<Guideline>>;
}

/// LLM-backed proposer.
pub struct LlmGuidelineContinuousProposer {
    pub nlp: Arc<dyn NlpService>,
}

impl LlmGuidelineContinuousProposer {
    pub fn new(nlp: Arc<dyn NlpService>) -> Self {
        Self { nlp }
    }
}

#[async_trait]
impl GuidelineContinuousProposer for LlmGuidelineContinuousProposer {
    async fn propose(
        &self,
        _agent_id: &AgentId,
        guidelines: &[Guideline],
        last_user_message: Option<&str>,
    ) -> EngineResult<Vec<Guideline>> {
        let msg = last_user_message.unwrap_or("");
        if guidelines.is_empty() || msg.is_empty() {
            return Ok(vec![]);
        }
        let mut prompt = String::from("Given the customer message:\n");
        prompt.push_str(&format!("  {}\n\nAvailable guidelines:\n", msg));
        for (i, g) in guidelines.iter().enumerate() {
            prompt.push_str(&format!(
                "  [{}] condition='{}', action='{}'\n",
                i, g.content.condition, g.content.action
            ));
        }
        prompt.push_str(
            "\nSelect any guidelines (by index) relevant to this message. \
             Return indices as guideline_ids strings.",
        );

        let generator = self
            .nlp
            .schematic_generator(ContinuousProposalOutput::schema())
            .await
            .map_err(|e| crate::error::EngineError::GuidelineMatchingFailed(e.to_string()))?;
        let result = generator
            .generate(prompt, Default::default())
            .await
            .map_err(|e| crate::error::EngineError::GuidelineMatchingFailed(e.to_string()))?;
        let parsed: ContinuousProposalOutput =
            serde_json::from_value(result.value).unwrap_or_default();

        let mut out = Vec::new();
        for gid in &parsed.guideline_ids {
            if let Ok(idx) = gid.parse::<usize>() {
                if idx < guidelines.len() {
                    out.push(guidelines[idx].clone());
                }
            }
        }
        Ok(out)
    }
}

pub struct NoopGuidelineContinuousProposer;

#[async_trait]
impl GuidelineContinuousProposer for NoopGuidelineContinuousProposer {
    async fn propose(
        &self,
        _: &AgentId,
        _: &[Guideline],
        _: Option<&str>,
    ) -> EngineResult<Vec<Guideline>> {
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use loon_nlp::test_utils::FakeNlpService;

    #[tokio::test]
    async fn noop_returns_empty() {
        assert!(NoopGuidelineContinuousProposer
            .propose(&AgentId::new(), &[], None)
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn llm_returns_empty_on_empty_guidelines() {
        let p = LlmGuidelineContinuousProposer::new(std::sync::Arc::new(FakeNlpService::new()));
        assert!(p
            .propose(&AgentId::new(), &[], Some("hi"))
            .await
            .unwrap()
            .is_empty());
    }
}

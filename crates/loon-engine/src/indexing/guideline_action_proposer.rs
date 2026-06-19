//! `GuidelineActionProposer` — proposes candidate guidelines that
//! might apply to a given agent action.

use async_trait::async_trait;
use std::collections::HashSet;

use loon_core::{AgentId, Guideline};

use super::common::GuidelineActionProposerOutput;
use crate::error::EngineResult;

#[async_trait]
pub trait GuidelineActionProposer: Send + Sync {
    async fn propose(
        &self,
        agent_id: &AgentId,
        all_guidelines: &[Guideline],
        action_text: &str,
    ) -> EngineResult<GuidelineActionProposerOutput>;
}

/// Keyword-overlap proposer: scores candidates by word overlap between
/// `action_text` and each guideline's condition + action text.
pub struct KeywordGuidelineActionProposer;

fn tokenize(s: &str) -> HashSet<String> {
    s.split_whitespace()
        .map(|w| {
            w.trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase()
        })
        .collect()
}

#[async_trait]
impl GuidelineActionProposer for KeywordGuidelineActionProposer {
    async fn propose(
        &self,
        _agent_id: &AgentId,
        all_guidelines: &[Guideline],
        action_text: &str,
    ) -> EngineResult<GuidelineActionProposerOutput> {
        let query = tokenize(action_text);
        if query.is_empty() {
            return Ok(GuidelineActionProposerOutput { candidates: vec![] });
        }
        let mut scored: Vec<(usize, Guideline)> = all_guidelines
            .iter()
            .enumerate()
            .filter_map(|(i, g)| {
                let combined = format!("{} {}", g.content.condition, g.content.action);
                let overlap = tokenize(&combined).intersection(&query).count();
                if overlap > 0 {
                    Some((i, g.clone()))
                } else {
                    None
                }
            })
            .collect();
        scored.sort_by_key(|b| std::cmp::Reverse(b.0));
        Ok(GuidelineActionProposerOutput {
            candidates: scored.into_iter().map(|(_, g)| g).collect(),
        })
    }
}

pub struct NoopGuidelineActionProposer;

#[async_trait]
impl GuidelineActionProposer for NoopGuidelineActionProposer {
    async fn propose(
        &self,
        _: &AgentId,
        _: &[Guideline],
        _: &str,
    ) -> EngineResult<GuidelineActionProposerOutput> {
        Ok(GuidelineActionProposerOutput { candidates: vec![] })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use loon_core::{Criticality, GuidelineContent, GuidelineId};

    fn g(action: &str) -> Guideline {
        Guideline {
            id: GuidelineId::new(),
            agent_id: AgentId::new(),
            content: GuidelineContent {
                condition: "x".into(),
                action: action.into(),
                description: None,
            },
            criticality: Criticality::Low,
            enabled: true,
            tags: vec![],
            creation_utc: chrono::Utc::now(),
            metadata: loon_core::JsonValue::Null,
        }
    }

    #[tokio::test]
    async fn keyword_match_finds_overlapping_action() {
        let p = KeywordGuidelineActionProposer;
        let gs = vec![g("greet user"), g("transfer to billing")];
        let r = p
            .propose(&AgentId::new(), &gs, "greet")
            .await
            .unwrap();
        assert_eq!(r.candidates.len(), 1);
        assert!(r.candidates[0].content.action.contains("greet"));
    }
}

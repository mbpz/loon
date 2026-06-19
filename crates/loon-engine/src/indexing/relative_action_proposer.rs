//! `RelativeActionProposer` — proposes guidelines that are
//! "relative" to a baseline guideline (e.g. exceptions, refinements).

use async_trait::async_trait;
use std::collections::HashSet;

use loon_core::Guideline;

use crate::error::EngineResult;

#[async_trait]
pub trait RelativeActionProposer: Send + Sync {
    async fn propose(
        &self,
        baseline: &Guideline,
        all_guidelines: &[Guideline],
    ) -> EngineResult<Vec<Guideline>>;
}

pub struct KeywordRelativeActionProposer;

fn words(s: &str) -> HashSet<String> {
    s.split_whitespace()
        .map(|w| {
            w.trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase()
        })
        .collect()
}

#[async_trait]
impl RelativeActionProposer for KeywordRelativeActionProposer {
    async fn propose(
        &self,
        baseline: &Guideline,
        all_guidelines: &[Guideline],
    ) -> EngineResult<Vec<Guideline>> {
        let bw = words(&baseline.content.condition);
        if bw.len() < 2 {
            return Ok(vec![]);
        }
        Ok(all_guidelines
            .iter()
            .filter(|g| g.id != baseline.id)
            .filter(|g| words(&g.content.condition).intersection(&bw).count() >= 2)
            .cloned()
            .collect())
    }
}

pub struct NoopRelativeActionProposer;

#[async_trait]
impl RelativeActionProposer for NoopRelativeActionProposer {
    async fn propose(
        &self,
        _: &Guideline,
        _: &[Guideline],
    ) -> EngineResult<Vec<Guideline>> {
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use loon_core::{AgentId, Criticality, GuidelineContent, GuidelineId};

    fn g(condition: &str, action: &str) -> Guideline {
        Guideline {
            id: GuidelineId::new(),
            agent_id: AgentId::new(),
            content: GuidelineContent {
                condition: condition.into(),
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
    async fn shares_two_words_triggers_proposal() {
        let p = KeywordRelativeActionProposer;
        let gs = vec![
            g("greet user warmly", "x"),
            g("greet user politely", "y"),
            g("transfer billing", "z"),
        ];
        let r = p.propose(&gs[0], &gs).await.unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].content.action, "y");
    }
}

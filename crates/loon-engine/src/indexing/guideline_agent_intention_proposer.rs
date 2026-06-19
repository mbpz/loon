use async_trait::async_trait;
use std::collections::HashSet;
use loon_core::{Agent, Guideline};
use crate::error::EngineResult;

#[async_trait]
pub trait GuidelineAgentIntentionProposer: Send + Sync {
    async fn propose(&self, agent: &Agent, guidelines: &[Guideline]) -> EngineResult<Vec<Guideline>>;
}

fn keywords(s: &str) -> HashSet<String> {
    s.split_whitespace().map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase()).collect()
}

pub struct KeywordIntentionProposer;
#[async_trait]
impl GuidelineAgentIntentionProposer for KeywordIntentionProposer {
    async fn propose(&self, agent: &Agent, guidelines: &[Guideline]) -> EngineResult<Vec<Guideline>> {
        let profile = keywords(&format!("{} {}", agent.name, agent.description));
        if profile.is_empty() { return Ok(vec![]); }
        let mut scored: Vec<(usize, Guideline)> = guidelines.iter()
            .map(|g| {
                let s = keywords(&format!("{} {}", g.content.condition, g.content.action)).intersection(&profile).count();
                (s, g.clone())
            })
            .filter(|(s, _)| *s > 0)
            .collect();
        scored.sort_by_key(|b| std::cmp::Reverse(b.0));
        Ok(scored.into_iter().map(|(_, g)| g).collect())
    }
}

pub struct NoopGuidelineAgentIntentionProposer;
#[async_trait]
impl GuidelineAgentIntentionProposer for NoopGuidelineAgentIntentionProposer {
    async fn propose(&self, _: &Agent, _: &[Guideline]) -> EngineResult<Vec<Guideline>> { Ok(vec![]) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use loon_core::{AgentId, GuidelineContent, Criticality, GuidelineId};

    fn agent() -> Agent { Agent::new("helpdesk", "customer support and billing") }
    fn g(action: &str) -> Guideline {
        Guideline { id: GuidelineId::new(), agent_id: AgentId::new(), content: GuidelineContent { condition: "x".into(), action: action.into(), description: None }, criticality: Criticality::Low, enabled: true, tags: vec![], creation_utc: chrono::Utc::now(), metadata: loon_core::JsonValue::Null }
    }

    #[tokio::test]
    async fn matches_agent_intention_by_keyword() {
        let p = KeywordIntentionProposer;
        let gs = vec![g("billing dispute"), g("weather report")];
        let hits = p.propose(&agent(), &gs).await.unwrap();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].content.action.contains("billing"));
    }
}

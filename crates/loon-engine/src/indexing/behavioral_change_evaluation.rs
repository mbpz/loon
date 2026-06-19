use async_trait::async_trait;
use std::collections::HashSet;
use loon_core::Guideline;
use crate::error::EngineResult;
use super::common::BehavioralChangeEvaluation;

#[async_trait]
pub trait BehavioralChangeEvaluationTrait: Send + Sync {
    async fn evaluate(&self, changed: &Guideline, existing: &[Guideline]) -> EngineResult<BehavioralChangeEvaluation>;
}

fn tokenize(s: &str) -> HashSet<String> {
    s.split_whitespace().map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase()).collect()
}

pub struct OverlapBehavioralChangeEvaluation;
#[async_trait]
impl BehavioralChangeEvaluationTrait for OverlapBehavioralChangeEvaluation {
    async fn evaluate(&self, changed: &Guideline, existing: &[Guideline]) -> EngineResult<BehavioralChangeEvaluation> {
        let ct = tokenize(&changed.content.condition);
        if existing.is_empty() || ct.is_empty() { return Ok(BehavioralChangeEvaluation { guideline: changed.clone(), estimated_impact: 0.0 }); }
        let max_overlap = existing.iter()
            .filter(|g| g.id != changed.id)
            .map(|g| tokenize(&g.content.condition).intersection(&ct).count())
            .max()
            .unwrap_or(0);
        let impact = (max_overlap as f32 / ct.len() as f32).min(1.0);
        Ok(BehavioralChangeEvaluation { guideline: changed.clone(), estimated_impact: impact })
    }
}

pub struct NoopBehavioralChangeEvaluation;
#[async_trait]
impl BehavioralChangeEvaluationTrait for NoopBehavioralChangeEvaluation {
    async fn evaluate(&self, g: &Guideline, _: &[Guideline]) -> EngineResult<BehavioralChangeEvaluation> {
        Ok(BehavioralChangeEvaluation { guideline: g.clone(), estimated_impact: 0.0 })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use loon_core::{AgentId, GuidelineContent, Criticality, GuidelineId};
    fn g(condition: &str) -> Guideline {
        Guideline { id: GuidelineId::new(), agent_id: AgentId::new(), content: GuidelineContent { condition: condition.into(), action: "x".into(), description: None }, criticality: Criticality::Low, enabled: true, tags: vec![], creation_utc: chrono::Utc::now(), metadata: loon_core::JsonValue::Null }
    }
    #[tokio::test]
    async fn overlap_impact_above_zero() {
        let eval = OverlapBehavioralChangeEvaluation;
        let r = eval.evaluate(&g("greet user warmly"), &[g("greeting user"), g("transfer")]).await.unwrap();
        assert!(r.estimated_impact > 0.0);
    }
}

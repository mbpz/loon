//! `BehavioralChangeEvaluationTrait` — estimates how much a guideline
//! would change user-facing behavior if applied.

use async_trait::async_trait;

use loon_core::{AgentId, Guideline, GuidelineContent};

use super::common::BehavioralChangeEvaluation;
use crate::error::EngineResult;

/// Estimates the impact of applying a guideline.
#[async_trait]
pub trait BehavioralChangeEvaluationTrait: Send + Sync {
    async fn evaluate(&self, _g: &Guideline) -> EngineResult<BehavioralChangeEvaluation> {
        Ok(BehavioralChangeEvaluation {
            guideline: Guideline::new(
                GuidelineContent {
                    condition: "a".into(),
                    action: "b".into(),
                    description: None,
                },
                &AgentId::new(),
                true,
                0,
            ),
            estimated_impact: 0.0,
        })
    }
}

/// No-op implementation that returns zero impact.
pub struct NoopBehavioralChangeEvaluation;

#[async_trait]
impl BehavioralChangeEvaluationTrait for NoopBehavioralChangeEvaluation {}

#[cfg(test)]
mod tests {
    use super::*;

    fn _accepts(_: &dyn BehavioralChangeEvaluationTrait) {}

    #[tokio::test]
    async fn noop_evaluator_returns_zero_impact() {
        let e = NoopBehavioralChangeEvaluation;
        _accepts(&e);
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
        let res = e.evaluate(&g).await.unwrap();
        assert_eq!(res.estimated_impact, 0.0);
    }
}

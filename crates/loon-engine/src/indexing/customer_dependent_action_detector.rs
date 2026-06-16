//! `CustomerDependentActionDetector` — flags guidelines whose
//! action depends on per-customer state.

use async_trait::async_trait;

use loon_core::Guideline;

use crate::error::EngineResult;

#[async_trait]
pub trait CustomerDependentActionDetector: Send + Sync {
    async fn detect(&self, _g: &Guideline) -> EngineResult<bool> {
        Ok(false)
    }
}

pub struct NoopCustomerDependentActionDetector;

#[async_trait]
impl CustomerDependentActionDetector for NoopCustomerDependentActionDetector {}

#[cfg(test)]
mod tests {
    use super::*;
    use loon_core::{AgentId, GuidelineContent};

    fn _accepts(_: &dyn CustomerDependentActionDetector) {}

    #[tokio::test]
    async fn noop_detector_returns_false() {
        let d = NoopCustomerDependentActionDetector;
        _accepts(&d);
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
        assert!(!d.detect(&g).await.unwrap());
    }
}

//! `ToolRunningActionDetector` — flags guidelines whose action
//! invokes a tool.

use async_trait::async_trait;

use loon_core::{AgentId, Guideline, GuidelineContent};

use crate::error::EngineResult;

#[async_trait]
pub trait ToolRunningActionDetector: Send + Sync {
    async fn detect(&self, _g: &Guideline) -> EngineResult<bool> {
        Ok(false)
    }
}

pub struct NoopToolRunningActionDetector;

#[async_trait]
impl ToolRunningActionDetector for NoopToolRunningActionDetector {}

#[cfg(test)]
mod tests {
    use super::*;

    fn _accepts(_: &dyn ToolRunningActionDetector) {}

    #[tokio::test]
    async fn noop_detector_returns_false() {
        let d = NoopToolRunningActionDetector;
        let _ = _accepts(&d);
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

use async_trait::async_trait;
use loon_core::Guideline;
use crate::error::EngineResult;

#[async_trait]
pub trait CustomerDependentActionDetector: Send + Sync {
    async fn detect(&self, guideline: &Guideline) -> EngineResult<bool>;
}

pub struct KeywordCustomerDependencyDetector;
#[async_trait]
impl CustomerDependentActionDetector for KeywordCustomerDependencyDetector {
    async fn detect(&self, guideline: &Guideline) -> EngineResult<bool> {
        let combined = format!("{} {}", guideline.content.condition, guideline.content.action).to_lowercase();
        // Heuristic: customer-dependent guidelines mention customer state.
        for kw in &["customer", "user", "client", "account"] {
            if combined.contains(kw) { return Ok(true); }
        }
        Ok(false)
    }
}

pub struct NoopCustomerDependentActionDetector;
#[async_trait]
impl CustomerDependentActionDetector for NoopCustomerDependentActionDetector {
    async fn detect(&self, _: &Guideline) -> EngineResult<bool> { Ok(false) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use loon_core::{AgentId, GuidelineContent, Criticality, GuidelineId};
    #[tokio::test]
    async fn detects_customer_keywords() {
        let g = Guideline { id: GuidelineId::new(), agent_id: AgentId::new(), content: GuidelineContent { condition: "x".into(), action: "greet customer".into(), description: None }, criticality: Criticality::Low, enabled: true, tags: vec![], creation_utc: chrono::Utc::now(), metadata: loon_core::JsonValue::Null };
        assert!(KeywordCustomerDependencyDetector.detect(&g).await.unwrap());
    }
    #[tokio::test]
    async fn pure_system_action_no_detection() {
        let g = Guideline { id: GuidelineId::new(), agent_id: AgentId::new(), content: GuidelineContent { condition: "x".into(), action: "log metrics".into(), description: None }, criticality: Criticality::Low, enabled: true, tags: vec![], creation_utc: chrono::Utc::now(), metadata: loon_core::JsonValue::Null };
        assert!(!KeywordCustomerDependencyDetector.detect(&g).await.unwrap());
    }
}

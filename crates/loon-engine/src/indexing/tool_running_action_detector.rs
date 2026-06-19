use async_trait::async_trait;
use loon_core::{Guideline, Tool};
use crate::error::EngineResult;

#[async_trait]
pub trait ToolRunningActionDetector: Send + Sync {
    async fn detect(&self, guideline: &Guideline, available_tools: &[Tool]) -> EngineResult<bool>;
}

pub struct NameMatchToolDetector;
#[async_trait]
impl ToolRunningActionDetector for NameMatchToolDetector {
    async fn detect(&self, guideline: &Guideline, tools: &[Tool]) -> EngineResult<bool> {
        let action_lower = guideline.content.action.to_lowercase();
        Ok(tools.iter().any(|t| action_lower.contains(&t.name.to_lowercase())))
    }
}

pub struct NoopToolRunningActionDetector;
#[async_trait]
impl ToolRunningActionDetector for NoopToolRunningActionDetector {
    async fn detect(&self, _: &Guideline, _: &[Tool]) -> EngineResult<bool> { Ok(false) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use loon_core::{AgentId, GuidelineContent, Criticality, GuidelineId, ToolId, ToolKind};
    fn tool(name: &str) -> Tool {
        Tool { id: ToolId::new(), name: name.into(), description: "".into(), parameters_schema: loon_core::JsonValue::Null, kind: ToolKind::Local, creation_utc: chrono::Utc::now() }
    }
    #[tokio::test]
    async fn detects_tool_name_in_action() {
        let d = NameMatchToolDetector;
        let g = Guideline { id: GuidelineId::new(), agent_id: AgentId::new(), content: GuidelineContent { condition: "x".into(), action: "call book_flight tool".into(), description: None }, criticality: Criticality::Low, enabled: true, tags: vec![], creation_utc: chrono::Utc::now(), metadata: loon_core::JsonValue::Null };
        assert!(d.detect(&g, &[tool("book_flight")]).await.unwrap());
    }
    #[tokio::test]
    async fn no_match_when_tool_not_in_action() {
        let d = NameMatchToolDetector;
        let g = Guideline { id: GuidelineId::new(), agent_id: AgentId::new(), content: GuidelineContent { condition: "x".into(), action: "just chat".into(), description: None }, criticality: Criticality::Low, enabled: true, tags: vec![], creation_utc: chrono::Utc::now(), metadata: loon_core::JsonValue::Null };
        assert!(!d.detect(&g, &[tool("book_flight")]).await.unwrap());
    }
}

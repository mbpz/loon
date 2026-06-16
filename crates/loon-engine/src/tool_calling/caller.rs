//! Core `ToolCaller` and `ToolCallBatch` traits plus `ToolExecutionResult`.

use async_trait::async_trait;

use loon_core::{JsonValue, TagId, Tool, ToolId, ToolResult};

use crate::engine_context::{EngineContext, GuidelineMatch, ToolInsights};

use crate::error::EngineResult;

/// Outcome of invoking a single tool.
pub struct ToolExecutionResult {
    pub tool_id: ToolId,
    pub result: ToolResult,
}

/// Decides which tools to call and produces insights during
/// preparation.
#[async_trait]
pub trait ToolCaller: Send + Sync {
    async fn generate_insights(
        &self,
        _ctx: &EngineContext,
        _guidelines: &[GuidelineMatch],
    ) -> EngineResult<ToolInsights> {
        Ok(ToolInsights::default())
    }
    async fn call_tools(
        &self,
        _ctx: &EngineContext,
        _insights: &ToolInsights,
    ) -> EngineResult<Vec<ToolExecutionResult>> {
        Ok(vec![])
    }
}

/// One strategy for batching a slice of tools into actual calls.
#[async_trait]
pub trait ToolCallBatch: Send + Sync {
    async fn decide_and_call(
        &self,
        _tools: &[Tool],
        _ctx: &EngineContext,
    ) -> EngineResult<Vec<ToolExecutionResult>> {
        Ok(vec![])
    }
}

// Reference TagId and JsonValue so the imports are used; downstream
// implementations will need them.
const _: fn() = || {
    let _: TagId = TagId::new();
    let _: JsonValue = serde_json::Value::Null;
};

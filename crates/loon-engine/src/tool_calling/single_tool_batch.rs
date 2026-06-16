//! Strategy that handles tools one at a time.

use async_trait::async_trait;

use loon_core::Tool;

use crate::engine_context::EngineContext;
use crate::error::EngineResult;
use crate::tool_calling::caller::{ToolCallBatch, ToolExecutionResult};

pub struct SingleToolBatch;

#[async_trait]
impl ToolCallBatch for SingleToolBatch {
    async fn decide_and_call(
        &self,
        tools: &[Tool],
        _ctx: &EngineContext,
    ) -> EngineResult<Vec<ToolExecutionResult>> {
        // Phase 1: invoke every tool in sequence with default args.
        Ok(tools
            .iter()
            .map(|t| ToolExecutionResult {
                tool_id: t.id.clone(),
                result: loon_core::ToolResult::default(),
            })
            .collect())
    }
}

//! Strategy that handles overlapping tools (tools whose parameter
//! spaces share data) together in a single batch.

use async_trait::async_trait;

use loon_core::Tool;

use crate::engine_context::EngineContext;
use crate::error::EngineResult;
use crate::tool_calling::caller::{ToolCallBatch, ToolExecutionResult};

pub struct OverlappingToolsBatch;

#[async_trait]
impl ToolCallBatch for OverlappingToolsBatch {
    async fn decide_and_call(
        &self,
        tools: &[Tool],
        _ctx: &EngineContext,
    ) -> EngineResult<Vec<ToolExecutionResult>> {
        // Phase 1 stub: same as single-tool batch. Real impl will
        // group tools by overlapping inputs and merge args.
        Ok(tools
            .iter()
            .map(|t| ToolExecutionResult {
                tool_id: t.id.clone(),
                result: loon_core::ToolResult::default(),
            })
            .collect())
    }
}

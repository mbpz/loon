//! `OptimizationPolicy` — short-circuits expensive preparation
//! steps (tool calls, guideline matching) when their result is
//! known to be unnecessary.

use loon_core::ToolId;

use crate::engine_context::EngineContext;

/// Decides whether to skip expensive preparation work.
pub trait OptimizationPolicy: Send + Sync {
    /// Skip calling `tool_id` for this request.
    fn should_skip_tool(&self, _tool_id: &ToolId, _ctx: &EngineContext) -> bool {
        false
    }
    /// Skip the guideline matching pass entirely.
    fn should_skip_guideline_matching(&self, _ctx: &EngineContext) -> bool {
        false
    }
}

/// Default policy: never skip anything.
pub struct DefaultOptimizationPolicy;

impl OptimizationPolicy for DefaultOptimizationPolicy {}

#[cfg(test)]
mod tests {
    use super::*;
    use loon_core::ToolId;

    #[test]
    fn default_policy_constructs_and_is_object_safe() {
        // Compile-time check: the trait is dyn-compatible and the
        // default policy implements it. We can't easily build an
        // EngineContext in a unit test, so the trait method
        // exercise is deferred to integration tests; the
        // contract is that the default impl never skips.
        let p: Box<dyn OptimizationPolicy> = Box::new(DefaultOptimizationPolicy);
        let _ = p;
        // ToolId is referenced so the import isn't unused.
        let _: ToolId = ToolId::new();
    }
}

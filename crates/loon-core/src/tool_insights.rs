use crate::common::JsonValue;
use crate::ToolId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Aggregated observations / metrics produced by tool executions.
/// Stub for Stage 1; populated in later stages.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolInsights {
    pub per_tool: HashMap<ToolId, JsonValue>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tool_insights_default_empty() {
        let i = ToolInsights::default();
        assert!(i.per_tool.is_empty());
    }
}

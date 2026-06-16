//! `ToolEventGenerator` — packs the LLM-requested tool calls into
//! a `ToolEventData` payload ready for emission.

use loon_core::{ToolCallData, ToolEventData};

/// Phase-1 stub: returns a `ToolEventData` containing the calls
/// verbatim. A future implementation may attach per-tool metadata
/// (timing, status) before emission.
pub struct ToolEventGenerator;

impl ToolEventGenerator {
    pub fn generate_tool_event_data(calls: &[ToolCallData]) -> ToolEventData {
        ToolEventData {
            tool_calls: calls.to_vec(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use loon_core::{JsonValue, ToolId, ToolResult};

    #[test]
    fn generate_tool_event_data_round_trips_calls() {
        let calls = vec![
            ToolCallData {
                tool_id: "tool-1".into(),
                arguments: JsonValue::Null,
                result: Some(ToolResult::default()),
            },
            ToolCallData {
                tool_id: "tool-2".into(),
                arguments: serde_json::json!({"q":"x"}),
                result: None,
            },
        ];
        let data = ToolEventGenerator::generate_tool_event_data(&calls);
        assert_eq!(data.tool_calls.len(), 2);
        assert_eq!(data.tool_calls[0].tool_id, "tool-1");
        assert_eq!(data.tool_calls[1].tool_id, "tool-2");

        // Reference ToolId so the import is used.
        let _: ToolId = ToolId::new();
    }
}

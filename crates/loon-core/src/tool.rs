use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::{ToolId, CannedResponseId, JsonValue};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolKind {
    Local,
    OpenAPI,
    MCP,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tool {
    pub id: ToolId,
    pub name: String,
    pub description: String,
    pub parameters_schema: JsonValue,
    pub kind: ToolKind,
    pub creation_utc: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ToolResult {
    pub data: JsonValue,
    pub metadata: JsonValue,
    pub control: ToolResultControl,
    pub canned_responses: Vec<CannedResponseId>,
    pub canned_response_fields: HashMap<String, String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ToolResultControl {
    pub is_error: bool,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    pub tool_id: ToolId,
    pub arguments: JsonValue,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCallData {
    pub tool_id: String,
    pub arguments: JsonValue,
    pub result: Option<ToolResult>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tool_kind_serializes() {
        let k = ToolKind::Local;
        let s = serde_json::to_string(&k).unwrap();
        assert_eq!(s, "\"Local\"");
    }
    #[test]
    fn tool_result_default_ok() {
        let r = ToolResult::default();
        assert!(!r.control.is_error);
    }
}

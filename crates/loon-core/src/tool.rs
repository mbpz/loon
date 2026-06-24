use crate::{CannedResponseId, JsonValue, ToolId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolKind {
    Local,
    OpenAPI,
    MCP,
}

/// A registrable tool the engine can invoke.
///
/// # Example
///
/// ```
/// # use loon_core::{Tool, ToolId, ToolKind};
/// let tool = Tool {
///     id: ToolId::new(),
///     name: "search_kb".into(),
///     description: "search the knowledge base".into(),
///     parameters_schema: serde_json::Value::Null,
///     kind: ToolKind::Local,
///     creation_utc: chrono::Utc::now(),
/// };
/// assert_eq!(tool.name, "search_kb");
/// assert_eq!(tool.kind, ToolKind::Local);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tool {
    pub id: ToolId,
    pub name: String,
    pub description: String,
    pub parameters_schema: JsonValue,
    pub kind: ToolKind,
    pub creation_utc: DateTime<Utc>,
}

/// Partial-update params for `Tool`. Mutable fields are `name`,
/// `description`, and `parameters_schema`; `id`, `kind`, and
/// `creation_utc` are immutable.
#[derive(Debug, Default, Clone)]
pub struct ToolUpdateParams {
    pub name: Option<String>,
    pub description: Option<String>,
    pub parameters_schema: Option<JsonValue>,
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

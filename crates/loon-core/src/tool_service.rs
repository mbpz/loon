use async_trait::async_trait;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

use crate::async_utils::BoxFuture;
use crate::{CoreResult, JsonValue, Tool, ToolId, ToolResult};

/// Abstraction for invoking a tool by id. Implementations may dispatch
/// to in-process handlers, remote MCP servers, or OpenAPI backends.
#[async_trait]
pub trait ToolService: Send + Sync {
    async fn list_tools(&self) -> CoreResult<Vec<Tool>>;
    async fn call_tool(&self, tool_id: &ToolId, arguments: JsonValue) -> CoreResult<ToolResult>;
}

/// Function pointer for a single tool's in-process handler.
pub type ToolHandler =
    Arc<dyn Fn(JsonValue) -> BoxFuture<'static, CoreResult<ToolResult>> + Send + Sync>;

/// In-process tool service. Holds a static catalogue of `Tool` metadata and
/// a handler map for actually executing tool calls.
pub struct LocalToolService {
    pub tools: Vec<Tool>,
    pub handlers: RwLock<HashMap<ToolId, ToolHandler>>,
}

impl LocalToolService {
    pub fn new(tools: Vec<Tool>) -> Self {
        Self {
            tools,
            handlers: RwLock::new(HashMap::new()),
        }
    }

    pub fn register_handler(&self, id: ToolId, h: ToolHandler) {
        self.handlers.write().insert(id, h);
    }
}

#[async_trait]
impl ToolService for LocalToolService {
    async fn list_tools(&self) -> CoreResult<Vec<Tool>> {
        Ok(self.tools.clone())
    }

    async fn call_tool(&self, id: &ToolId, args: JsonValue) -> CoreResult<ToolResult> {
        let h = self
            .handlers
            .read()
            .get(id)
            .cloned()
            .ok_or_else(|| crate::CoreError::NotFound(crate::UniqueId(id.0.clone())))?;
        h(args).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ToolKind;
    use chrono::Utc;

    fn fake_tool(id: &ToolId) -> Tool {
        Tool {
            id: id.clone(),
            name: "echo".into(),
            description: "echo".into(),
            parameters_schema: JsonValue::Null,
            kind: ToolKind::Local,
            creation_utc: Utc::now(),
        }
    }

    #[tokio::test]
    async fn list_tools_returns_registered_catalogue() {
        let t = ToolId::new();
        let svc = LocalToolService::new(vec![fake_tool(&t)]);
        let tools = svc.list_tools().await.unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].id, t);
    }
}

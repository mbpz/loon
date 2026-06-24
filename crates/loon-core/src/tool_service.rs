use async_trait::async_trait;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

use crate::async_utils::BoxFuture;
use crate::tool_context::ToolContext;
use crate::{CoreResult, JsonValue, Tool, ToolId, ToolResult};

/// Abstraction for invoking a tool by id. Implementations may dispatch
/// to in-process handlers, remote MCP servers, or OpenAPI backends.
#[async_trait]
pub trait ToolService: Send + Sync {
    async fn list_tools(&self) -> CoreResult<Vec<Tool>>;
    async fn call_tool(&self, tool_id: &ToolId, arguments: JsonValue) -> CoreResult<ToolResult>;

    /// Context-aware variant of [`call_tool`]. Default implementation
    /// discards the context and delegates so existing services keep
    /// working unchanged. Implementations that want session/agent ids
    /// at handler invocation time (e.g. [`LocalToolService`]) override
    /// this to route to context-aware handlers when registered.
    async fn call_tool_with_context(
        &self,
        tool_id: &ToolId,
        arguments: JsonValue,
        _ctx: ToolContext,
    ) -> CoreResult<ToolResult> {
        self.call_tool(tool_id, arguments).await
    }
}

/// Function pointer for a single tool's in-process handler.
pub type ToolHandler =
    Arc<dyn Fn(JsonValue) -> BoxFuture<'static, CoreResult<ToolResult>> + Send + Sync>;

/// Context-aware handler: same as `ToolHandler` but also receives a
/// [`ToolContext`] so the handler can read identifying ids.
pub type ToolHandlerWithContext = Arc<
    dyn Fn(JsonValue, ToolContext) -> BoxFuture<'static, CoreResult<ToolResult>> + Send + Sync,
>;

/// In-process tool service. Holds a static catalogue of `Tool` metadata and
/// a handler map for actually executing tool calls.
pub struct LocalToolService {
    pub tools: Vec<Tool>,
    pub handlers: RwLock<HashMap<ToolId, ToolHandler>>,
    pub handlers_with_ctx: RwLock<HashMap<ToolId, ToolHandlerWithContext>>,
}

impl LocalToolService {
    pub fn new(tools: Vec<Tool>) -> Self {
        Self {
            tools,
            handlers: RwLock::new(HashMap::new()),
            handlers_with_ctx: RwLock::new(HashMap::new()),
        }
    }

    pub fn register_handler(&self, id: ToolId, h: ToolHandler) {
        self.handlers.write().insert(id, h);
    }

    /// Register a context-aware handler. Takes precedence over a
    /// plain `register_handler` entry with the same id when the
    /// service is invoked via [`ToolService::call_tool_with_context`].
    pub fn register_handler_with_context(&self, id: ToolId, h: ToolHandlerWithContext) {
        self.handlers_with_ctx.write().insert(id, h);
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

    async fn call_tool_with_context(
        &self,
        id: &ToolId,
        args: JsonValue,
        ctx: ToolContext,
    ) -> CoreResult<ToolResult> {
        // Prefer the context-aware handler if registered; otherwise
        // fall back to the plain handler so existing registrations
        // keep working unchanged.
        let ctx_handler = self.handlers_with_ctx.read().get(id).cloned();
        if let Some(h) = ctx_handler {
            return h(args, ctx).await;
        }
        self.call_tool(id, args).await
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

    #[tokio::test]
    async fn context_aware_handler_receives_ids() {
        use crate::{AgentId, SessionId};

        let t = ToolId::new();
        let svc = LocalToolService::new(vec![fake_tool(&t)]);
        let session_id = SessionId::new();
        let agent_id = AgentId::new();
        let expect_session = session_id.clone();
        let expect_agent = agent_id.clone();

        svc.register_handler_with_context(
            t.clone(),
            Arc::new(move |_args, ctx| {
                let expect_session = expect_session.clone();
                let expect_agent = expect_agent.clone();
                Box::pin(async move {
                    assert_eq!(ctx.session_id, expect_session);
                    assert_eq!(ctx.agent_id, expect_agent);
                    Ok(ToolResult {
                        data: serde_json::json!({"ok": true}),
                        ..Default::default()
                    })
                })
            }),
        );

        let ctx = ToolContext {
            agent_id,
            session_id,
            customer_id: None,
        };
        let r = svc
            .call_tool_with_context(&t, JsonValue::Null, ctx)
            .await
            .unwrap();
        assert_eq!(r.data, serde_json::json!({"ok": true}));
    }

    #[tokio::test]
    async fn call_tool_with_context_falls_back_to_plain_handler() {
        use crate::{AgentId, SessionId};

        let t = ToolId::new();
        let svc = LocalToolService::new(vec![fake_tool(&t)]);
        svc.register_handler(
            t.clone(),
            Arc::new(|_args| {
                Box::pin(async {
                    Ok(ToolResult {
                        data: serde_json::json!({"plain": true}),
                        ..Default::default()
                    })
                })
            }),
        );
        let ctx = ToolContext {
            agent_id: AgentId::new(),
            session_id: SessionId::new(),
            customer_id: None,
        };
        let r = svc
            .call_tool_with_context(&t, JsonValue::Null, ctx)
            .await
            .unwrap();
        assert_eq!(r.data, serde_json::json!({"plain": true}));
    }
}

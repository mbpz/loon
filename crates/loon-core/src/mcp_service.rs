//! Adapter that lets an [`McpClient`] participate in the
//! [`ServiceRegistry`](crate::ServiceRegistry) as a [`ToolService`].
//!
//! Use this when the MCP server should appear alongside other tool
//! sources (e.g. `LocalToolService`, OpenAPI tools). The registry
//! stores one `Box<dyn ToolService>` per server name.

use async_trait::async_trait;
use std::sync::Arc;

use crate::mcp_client::McpClient;
use crate::{CoreResult, JsonValue, Tool, ToolId, ToolResult, ToolService};

/// Bridges an `McpClient` into a `ToolService`-shaped facade.
pub struct McpToolServiceAdapter {
    pub client: Arc<McpClient>,
}

#[async_trait]
impl ToolService for McpToolServiceAdapter {
    async fn list_tools(&self) -> CoreResult<Vec<Tool>> {
        self.client.list_tools().await
    }

    async fn call_tool(&self, tool_id: &ToolId, arguments: JsonValue) -> CoreResult<ToolResult> {
        self.client.call_tool(tool_id, arguments).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{McpClient, McpTransport};

    #[tokio::test]
    async fn adapter_list_tools_propagates_phase1_error() {
        // Adapter mirrors the underlying client's contract: in
        // Phase 1 the MCP transport is unwired, so the adapter
        // surfaces the same `MCP list_tools not yet implemented`
        // error the client returns. The previous version asserted
        // an empty list, which lied about transport state.
        let client = Arc::new(McpClient::new(
            "test",
            McpTransport::Http {
                url: "http://x".into(),
            },
        ));
        let adapter = McpToolServiceAdapter { client };
        let err = adapter
            .list_tools()
            .await
            .expect_err("phase 1 list_tools should error");
        let msg = err.to_string();
        assert!(
            msg.contains("MCP list_tools not yet implemented"),
            "got: {msg}"
        );
    }

    #[tokio::test]
    async fn adapter_call_tool_returns_internal_error_in_phase1() {
        let client = Arc::new(McpClient::new(
            "test",
            McpTransport::Http {
                url: "http://x".into(),
            },
        ));
        let adapter = McpToolServiceAdapter { client };
        let res = adapter
            .call_tool(&ToolId::new(), serde_json::json!({}))
            .await;
        let err = res.expect_err("phase 1 call_tool should error");
        let msg = err.to_string();
        assert!(
            msg.contains("MCP call_tool not yet implemented"),
            "got: {msg}"
        );
    }

    #[test]
    fn adapter_exposes_client_field() {
        let client = Arc::new(McpClient::new(
            "test",
            McpTransport::Http {
                url: "http://x".into(),
            },
        ));
        let adapter = McpToolServiceAdapter {
            client: client.clone(),
        };
        assert_eq!(adapter.client.name(), "test");
    }
}

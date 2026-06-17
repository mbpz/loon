//! MCP (Model Context Protocol) client.
//!
//! Connects to a single MCP server (stdio or HTTP transport),
//! lists the tools it provides, and forwards `call_tool` requests.

use async_trait::async_trait;
use parking_lot::Mutex;
use serde_json::Value as JsonValue;

use crate::id_generator::IdGenerator;
use crate::{CoreError, CoreResult, Tool, ToolId, ToolResult, ToolService};

/// How to reach an MCP server.
///
/// Phase 1 only stores the configuration; real transport wiring lands
/// once the `mcp` crate exposes a stable async API.
#[allow(dead_code)]
pub enum McpTransport {
    Stdio { command: String, args: Vec<String> },
    Http { url: String },
}

/// Client for a single MCP server.
///
/// Phase 1 focuses on the *shape* of the API (stable tool IDs, a
/// `ToolService` impl, caching). It does not actually open a transport
/// or call `tools/list` against a live server yet.
pub struct McpClient {
    name: String,
    /// Stored for future transport wiring; unused in Phase 1.
    #[allow(dead_code)]
    transport: McpTransport,
    /// Lazily initialized tool list (server-provided tool metadata).
    tools_cache: Mutex<Option<Vec<Tool>>>,
    /// ID generator for stable tool IDs.
    id_gen: Mutex<IdGenerator>,
}

impl McpClient {
    pub fn new(name: impl Into<String>, transport: McpTransport) -> Self {
        Self {
            name: name.into(),
            transport,
            tools_cache: Mutex::new(None),
            id_gen: Mutex::new(IdGenerator::new()),
        }
    }

    /// The configured server name (e.g. for use as a registry key).
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Compute the stable tool ID for a given tool name.
    /// Format: `mcp:<server>:<tool_name>` so it's deterministic.
    pub fn tool_id(&self, tool_name: &str) -> ToolId {
        let mut gen = self.id_gen.lock();
        let key = format!("mcp:{}:{}", self.name, tool_name);
        ToolId(gen.generate(&key).0)
    }

    /// Fetch the list of tools from the MCP server.
    /// In Phase 1 (no real MCP), this is a stub that returns an empty list.
    /// Future phases will use the `mcp` crate to call `tools/list`.
    pub async fn fetch_tools(&self) -> CoreResult<Vec<Tool>> {
        // Phase 1: no live MCP. Real impl lands when mcp crate integration
        // surfaces a stable async API; today we expose the trait and let
        // callers see an empty list.
        Ok(vec![])
    }
}

#[async_trait]
impl ToolService for McpClient {
    async fn list_tools(&self) -> CoreResult<Vec<Tool>> {
        {
            let cache = self.tools_cache.lock();
            if let Some(tools) = cache.as_ref() {
                return Ok(tools.clone());
            }
        }
        let tools = self.fetch_tools().await?;
        *self.tools_cache.lock() = Some(tools.clone());
        Ok(tools)
    }

    async fn call_tool(&self, _tool_id: &ToolId, _arguments: JsonValue) -> CoreResult<ToolResult> {
        // Phase 1: stub. Real impl lands when mcp crate integration
        // surfaces a stable async API.
        Err(CoreError::Internal("MCP call_tool not yet implemented".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn tool_id_is_deterministic() {
        let c1 = McpClient::new(
            "server-a",
            McpTransport::Http {
                url: "http://x".into(),
            },
        );
        let c2 = McpClient::new(
            "server-a",
            McpTransport::Http {
                url: "http://x".into(),
            },
        );
        assert_eq!(c1.tool_id("ping").0, c2.tool_id("ping").0);
    }

    #[test]
    fn tool_id_different_servers_yield_different_ids() {
        let c1 = McpClient::new(
            "server-a",
            McpTransport::Http {
                url: "http://x".into(),
            },
        );
        let c2 = McpClient::new(
            "server-b",
            McpTransport::Http {
                url: "http://x".into(),
            },
        );
        assert_ne!(c1.tool_id("ping").0, c2.tool_id("ping").0);
    }

    #[tokio::test]
    async fn list_tools_returns_empty_in_phase1() {
        let c = McpClient::new(
            "test",
            McpTransport::Http {
                url: "http://x".into(),
            },
        );
        let tools = c.list_tools().await.unwrap();
        assert!(tools.is_empty());
    }

    #[test]
    fn tool_kind_mcp_is_declared() {
        // Sanity-check: ensure the MCP variant of ToolKind exists and
        // is distinct from Local/OpenAPI. Future code will mark tools
        // returned by `McpClient` as `ToolKind::MCP`.
        use crate::ToolKind;
        assert_ne!(ToolKind::MCP, ToolKind::Local);
        assert_ne!(ToolKind::MCP, ToolKind::OpenAPI);
    }

    #[allow(dead_code)]
    fn _ensure_arc_constructible() {
        // The adapter (and registry) need to hold McpClient behind an Arc.
        let _arc: Arc<McpClient> = Arc::new(McpClient::new(
            "x",
            McpTransport::Http {
                url: "http://x".into(),
            },
        ));
    }
}

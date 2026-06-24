//! Tool execution context — passed to local tool handlers so they
//! can read session/agent state and write back via the
//! `EntityCommands` write path.
//!
//! Parlcant's `parlant.sdk.ToolContext` is the wrapping argument
//! every local tool handler receives. The Rust shape mirrors that:
//! identifying ids + a handle to `EntityCommands` for any state
//! mutations the tool wants to make as a side effect.

use loon_core::{AgentId, CustomerId, SessionId};
use std::sync::Arc;

/// What a local tool handler sees during invocation.
///
/// Construct this at engine-invocation time (see `DefaultToolCallBatcher`).
/// Local tool handlers can:
/// - read the current ids via the public fields
/// - mutate session labels / context-variable values via `commands`
///
/// `ToolContext` is `Clone` so handlers can move the necessary parts
/// into spawned tasks. The inner `EntityCommands` is reference-counted
/// (`Arc`), so cloning is cheap.
#[derive(Clone)]
pub struct ToolContext {
    pub agent_id: AgentId,
    pub session_id: SessionId,
    pub customer_id: Option<CustomerId>,
    /// Write-side handle for session / context-variable mutations.
    pub commands: Arc<loon_core::entity_cq::EntityCommands>,
}

impl std::fmt::Debug for ToolContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolContext")
            .field("agent_id", &self.agent_id)
            .field("session_id", &self.session_id)
            .field("customer_id", &self.customer_id)
            .field("commands", &"<EntityCommands>")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use loon_core::entity_cq::{EntityCommands, EntityQueries};

    #[test]
    fn tool_context_is_clone_and_debug() {
        let queries = EntityQueries::in_memory();
        let commands = Arc::new(EntityCommands {
            session_store: queries.session_store.clone(),
            context_variable_store: queries.context_variable_store.clone(),
        });
        let ctx = ToolContext {
            agent_id: AgentId::new(),
            session_id: SessionId::new(),
            customer_id: None,
            commands,
        };
        let cloned = ctx.clone();
        assert_eq!(ctx.agent_id, cloned.agent_id);
        assert!(format!("{:?}", ctx).contains("ToolContext"));
    }
}

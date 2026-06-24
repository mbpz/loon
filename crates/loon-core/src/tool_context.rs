//! Tool execution context — minimal identifying fields passed to
//! handlers that need to know which session / agent / customer they
//! are running for.
//!
//! Parlcant's `parlant.sdk.ToolContext` is the wrapping argument
//! every local tool handler receives. The Rust shape mirrors that:
//! a small bag of ids the handler can read.
//!
//! Stored in `loon-core` so both the engine (when constructing
//! invocation context) and the SDK (when registering handlers) can
//! reach it without crossing crates.

use crate::{AgentId, CustomerId, SessionId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolContext {
    pub agent_id: AgentId,
    pub session_id: SessionId,
    pub customer_id: Option<CustomerId>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_context_is_clone_and_debug() {
        let ctx = ToolContext {
            agent_id: AgentId::new(),
            session_id: SessionId::new(),
            customer_id: None,
        };
        let cloned = ctx.clone();
        assert_eq!(ctx, cloned);
        assert!(format!("{:?}", ctx).contains("ToolContext"));
    }
}

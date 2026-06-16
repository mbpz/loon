//! Engine hook points — 15 lifecycle events into which callers can
//! inject custom behaviour.

use std::collections::HashMap;
use std::sync::Arc;

use loon_core::async_utils::BoxFuture;
use loon_core::{GuidelineId, JourneyId, JsonValue};

use crate::engine_context::EngineContext;
use crate::error::EngineResult;

/// Outcome a hook can return.
pub enum EngineHookResult {
    /// Continue running the rest of the chain.
    CallNext,
    /// Short-circuit the chain and resolve with the engine's default
    /// behaviour.
    Resolve,
    /// Abort processing entirely.
    Bail,
}

/// Signature of a single hook. Receives the current `EngineContext`, an
/// optional data payload, and an optional error from the previous
/// stage.
pub type EngineHook = Arc<
    dyn Fn(
            EngineContext,
            Option<JsonValue>,
            Option<anyhow::Error>,
        ) -> BoxFuture<'static, EngineResult<EngineHookResult>>
        + Send
        + Sync,
>;

/// Container of all 15 hook points.
pub struct EngineHooks {
    pub on_error: Vec<EngineHook>,
    pub on_acknowledging: Vec<EngineHook>,
    pub on_acknowledged: Vec<EngineHook>,
    pub on_generating_preamble: Vec<EngineHook>,
    pub on_preamble_generated: Vec<EngineHook>,
    pub on_preamble_emitted: Vec<EngineHook>,
    pub on_preparing: Vec<EngineHook>,
    pub on_preparation_iteration_start: Vec<EngineHook>,
    pub on_preparation_iteration_end: Vec<EngineHook>,
    pub on_generating_messages: Vec<EngineHook>,
    pub on_draft_generated: Vec<EngineHook>,
    pub on_message_generated: Vec<EngineHook>,
    pub on_messages_emitted: Vec<EngineHook>,
    pub on_guideline_selected: HashMap<GuidelineId, Vec<EngineHook>>,
    pub on_journey_selected: HashMap<JourneyId, Vec<EngineHook>>,
}

impl Default for EngineHooks {
    fn default() -> Self {
        let v: Vec<EngineHook> = vec![];
        Self {
            on_error: v.clone(),
            on_acknowledging: v.clone(),
            on_acknowledged: v.clone(),
            on_generating_preamble: v.clone(),
            on_preamble_generated: v.clone(),
            on_preamble_emitted: v.clone(),
            on_preparing: v.clone(),
            on_preparation_iteration_start: v.clone(),
            on_preparation_iteration_end: v.clone(),
            on_generating_messages: v.clone(),
            on_draft_generated: v.clone(),
            on_message_generated: v.clone(),
            on_messages_emitted: v.clone(),
            on_guideline_selected: HashMap::new(),
            on_journey_selected: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_engine_hooks_has_all_fifteen_points() {
        let h = EngineHooks::default();
        assert!(h.on_error.is_empty());
        assert!(h.on_acknowledging.is_empty());
        assert!(h.on_acknowledged.is_empty());
        assert!(h.on_generating_preamble.is_empty());
        assert!(h.on_preamble_generated.is_empty());
        assert!(h.on_preamble_emitted.is_empty());
        assert!(h.on_preparing.is_empty());
        assert!(h.on_preparation_iteration_start.is_empty());
        assert!(h.on_preparation_iteration_end.is_empty());
        assert!(h.on_generating_messages.is_empty());
        assert!(h.on_draft_generated.is_empty());
        assert!(h.on_message_generated.is_empty());
        assert!(h.on_messages_emitted.is_empty());
        assert!(h.on_guideline_selected.is_empty());
        assert!(h.on_journey_selected.is_empty());
    }
}

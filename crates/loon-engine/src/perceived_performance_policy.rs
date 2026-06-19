//! `PerceivedPerformancePolicy` — controls the agent's
//! user-facing latency behaviour (e.g. partial responses,
//! progress signals, optimistic UI). Phase 1 is a no-op stub.

use crate::engine_context::EngineContext;

pub struct PerceivedPerformancePolicy;

impl PerceivedPerformancePolicy {
    pub fn new() -> Self {
        Self
    }

    /// Whether to emit a "thinking..." preamble before the engine
    /// starts the preparation loop.
    pub fn should_emit_preamble(&self, _ctx: &EngineContext) -> bool {
        // Phase 1: always emit a preamble when we have messages in
        // the interaction (i.e. this is not the very first message).
        !_ctx.interaction.messages().is_empty()
    }

    /// Whether to insert a brief pause between preparation
    /// iterations to smooth perceived latency.
    pub fn should_pace(&self, _ctx: &EngineContext) -> bool {
        true
    }
}

impl Default for PerceivedPerformancePolicy {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine_context::{EngineContext, Interaction};
    use loon_core::{Event, EventKind, EventSource};

    #[test]
    fn default_constructs_via_default_trait() {
        let p: PerceivedPerformancePolicy = Default::default();
        // Sanity: trait object construction is the compile-time
        // guarantee we care about.
        let _: PerceivedPerformancePolicy = PerceivedPerformancePolicy::new();
        let _ = p;
    }

    #[test]
    fn preamble_when_has_history() {
        let p = PerceivedPerformancePolicy::new();
        // placeholder has empty interaction -> should be false
        let mut ctx = EngineContext::placeholder();
        ctx.interaction = Interaction::new(vec![]);
        assert!(!p.should_emit_preamble(&ctx));

        // With a message event in the interaction -> should be true
        let event = Event {
            id: loon_core::EventId::new(),
            source: EventSource::Customer,
            kind: EventKind::Message,
            trace_id: "t1".into(),
            data: serde_json::json!({"message": "hello"}),
            metadata: None,
            creation_utc: chrono::Utc::now(),
        };
        ctx.interaction = Interaction::new(vec![event]);
        assert!(p.should_emit_preamble(&ctx));
    }

    #[test]
    fn should_pace_always_true() {
        assert!(PerceivedPerformancePolicy::new().should_pace(&EngineContext::placeholder()));
    }
}

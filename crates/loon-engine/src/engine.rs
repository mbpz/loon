//! `Engine` trait — top-level entry point for processing an
//! interaction and emitting responses.

use async_trait::async_trait;

use loon_emission::EventEmitter;

use crate::engine_context::Context;
use crate::error::EngineResult;

/// A request to utter a specific action and rationale.
#[derive(Debug, Clone)]
pub struct UtteranceRequest {
    pub action: String,
    pub rationale: String,
}

/// Why the engine is being asked to utter something. Currently a
/// Phase-1 marker; richer types will arrive in later stages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UtteranceRationale {
    Unspecified,
    BuyTime,
    FollowUp,
}

/// Top-level engine trait: process an interaction and (optionally)
/// emit a follow-up utterance.
#[async_trait]
pub trait Engine: Send + Sync {
    /// Process the interaction in `context` and emit events through
    /// `event_emitter`. Returns `true` when the engine handled the
    /// request, `false` when it deliberately yielded.
    async fn process(
        &self,
        context: &Context,
        event_emitter: &dyn EventEmitter,
    ) -> EngineResult<bool>;

    /// Emit one or more utterances in response to a prior
    /// `process` invocation. Returns `true` if any utterance was
    /// emitted.
    async fn utter(
        &self,
        context: &Context,
        event_emitter: &dyn EventEmitter,
        requests: &[UtteranceRequest],
    ) -> EngineResult<bool>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utterance_request_constructs() {
        let r = UtteranceRequest { action: "say hi".into(), rationale: "ack".into() };
        assert_eq!(r.action, "say hi");
    }

    #[test]
    fn utterance_rationale_distinct() {
        assert_ne!(UtteranceRationale::Unspecified, UtteranceRationale::BuyTime);
    }
}

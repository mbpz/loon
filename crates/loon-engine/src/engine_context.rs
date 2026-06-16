//! Engine context types: `Interaction`, `IterationState`, `ResponseState`,
//! `ToolInsights`, `GuidelineMatch`, and the top-level `EngineContext`.

use std::collections::HashMap;
use std::sync::Arc;

use loon_core::{
    Agent, AgentId, Customer, Event, EventKind, EventSource, Journey, JsonValue, Logger, Session,
    SessionId, Stopwatch, ToolId, ToolResult, Tracer,
};
use loon_emission::{EmittedEvent, EventEmitter};

use crate::error::EngineResult;

/// Identifies which session/agent an `EngineContext` is bound to.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Context {
    pub session_id: SessionId,
    pub agent_id: AgentId,
}

/// In-memory sequence of session events used for matching and generation.
#[derive(Clone)]
pub struct Interaction {
    pub events: Vec<Event>,
}

impl Interaction {
    pub fn new(events: Vec<Event>) -> Self {
        Self { events }
    }

    /// Extract only the message-type events, projected into
    /// `InteractionMessage`.
    pub fn messages(&self) -> Vec<InteractionMessage> {
        self.events
            .iter()
            .filter_map(|e| match e.kind {
                EventKind::Message => Some(InteractionMessage {
                    source: e.source,
                    participant: loon_core::Participant::default(),
                    trace_id: e.trace_id.clone(),
                    content: e
                        .data
                        .get("message")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    creation_utc: e.creation_utc,
                }),
                _ => None,
            })
            .collect()
    }

    /// Last message whose source is the customer.
    pub fn last_customer_message(&self) -> Option<InteractionMessage> {
        self.messages()
            .into_iter()
            .rev()
            .find(|m| matches!(m.source, EventSource::Customer))
    }
}

/// A flattened projection of a message event.
pub struct InteractionMessage {
    pub source: EventSource,
    pub participant: loon_core::Participant,
    pub trace_id: String,
    pub content: String,
    pub creation_utc: chrono::DateTime<chrono::Utc>,
}

/// State produced by one preparation iteration.
#[derive(Clone)]
pub struct IterationState {
    pub matched_guidelines: Vec<GuidelineMatch>,
    pub tool_insights: ToolInsights,
    pub executed_tools: Vec<ToolId>,
}

/// State accumulated across all preparation iterations for a single
/// response.
#[derive(Clone, Default)]
pub struct ResponseState {
    pub iterations: Vec<IterationState>,
    pub prepared_to_respond: bool,
    pub tool_insights: ToolInsights,
    pub ordinary_guideline_matches: Vec<GuidelineMatch>,
    pub usable_guidelines: Vec<loon_core::Guideline>,
    pub journeys: Vec<Journey>,
    pub message_events: Vec<EmittedEvent>,
    pub tool_events: Vec<EmittedEvent>,
}

/// Aggregate insights about the tools considered during preparation.
#[derive(Clone, Default)]
pub struct ToolInsights {
    pub evaluations: HashMap<ToolId, ToolCallEvaluation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolCallEvaluation {
    NeedsToRun,
    DataAlreadyInContext,
    Skipped,
}

/// A single guideline considered relevant for the current interaction,
/// along with matcher confidence and rationale.
#[derive(Clone)]
pub struct GuidelineMatch {
    pub guideline: loon_core::Guideline,
    pub confidence: f32,
    pub rationale: String,
}

/// Per-request engine context.
pub struct EngineContext {
    pub info: Context,
    pub logger: Arc<dyn Logger>,
    pub tracer: Arc<dyn Tracer>,
    pub agent: Agent,
    pub customer: Customer,
    pub session: Session,
    pub session_event_emitter: Arc<dyn EventEmitter>,
    pub response_event_emitter: Arc<dyn EventEmitter>,
    pub interaction: Interaction,
    pub state: parking_lot::Mutex<ResponseState>,
    pub creation: Stopwatch,
}

impl Clone for EngineContext {
    fn clone(&self) -> Self {
        Self {
            info: self.info.clone(),
            logger: self.logger.clone(),
            tracer: self.tracer.clone(),
            agent: self.agent.clone(),
            customer: self.customer.clone(),
            session: self.session.clone(),
            session_event_emitter: self.session_event_emitter.clone(),
            response_event_emitter: self.response_event_emitter.clone(),
            interaction: Interaction::new(self.interaction.events.clone()),
            state: parking_lot::Mutex::new(self.state.lock().clone()),
            creation: self.creation,
        }
    }
}

impl EngineContext {
    /// Phase-1 stub: real implementation will emit a tool event via
    /// `response_event_emitter` once preparation is wired up.
    pub async fn add_tool_event(
        &mut self,
        _tool_id: &ToolId,
        _args: JsonValue,
        _result: ToolResult,
    ) -> EngineResult<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use loon_core::{EventId, SessionId};

    fn msg_event(content: &str, source: EventSource) -> Event {
        Event {
            id: EventId::new(),
            source,
            kind: EventKind::Message,
            trace_id: "trace-1".into(),
            data: serde_json::json!({ "message": content }),
            metadata: None,
            creation_utc: chrono::Utc::now(),
        }
    }

    #[test]
    fn interaction_messages_filters_message_kind() {
        let mut events = vec![
            Event {
                id: EventId::new(),
                source: EventSource::System,
                kind: EventKind::Status,
                trace_id: "t".into(),
                data: serde_json::json!({}),
                metadata: None,
                creation_utc: chrono::Utc::now(),
            },
            msg_event("hello", EventSource::Customer),
            msg_event("hi there", EventSource::AiAgent),
        ];
        let interaction = Interaction::new(std::mem::take(&mut events));
        let msgs = interaction.messages();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].content, "hello");
        assert_eq!(msgs[1].content, "hi there");

        let last_customer = interaction.last_customer_message();
        assert!(last_customer.is_some());
        assert_eq!(last_customer.unwrap().content, "hello");

        // Silence the unused warning on SessionId - prove it's reexported.
        let _id = SessionId::new();
    }
}

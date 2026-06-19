//! Engine context types: `Interaction`, `IterationState`, `ResponseState`,
//! `ToolInsights`, `GuidelineMatch`, and the top-level `EngineContext`.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
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
    /// Build a no-op `EngineContext` for tests / placeholder
    /// callers. Every field is populated with a no-op default
    /// (a fresh `Agent`, empty `Customer`, empty `Session`, empty
    /// `Interaction`, and a pair of stub emitters).
    pub fn placeholder() -> Self {
        use loon_core::basic_tracer::BasicTracer;
        use loon_core::console_logger::ConsoleLogger;

        let agent_id = loon_core::AgentId::new();
        let session_id = loon_core::SessionId::new();
        let agent = loon_core::Agent::new("placeholder", "placeholder");
        let customer = loon_core::Customer::new("placeholder");
        let session = loon_core::Session::new(&agent_id);
        let emitter: Arc<dyn loon_emission::EventEmitter> = Arc::new(NoopEmitter);
        Self {
            info: Context {
                session_id,
                agent_id,
            },
            logger: Arc::new(ConsoleLogger),
            tracer: Arc::new(BasicTracer::new()),
            agent,
            customer,
            session,
            session_event_emitter: emitter.clone(),
            response_event_emitter: emitter,
            interaction: Interaction::new(vec![]),
            state: parking_lot::Mutex::new(ResponseState::default()),
            creation: loon_core::Stopwatch::start(),
        }
    }

    /// Load a fully-populated `EngineContext` from `EntityQueries`.
    ///
    /// - Reads agent and session by id from the supplied queries.
    /// - Reads the customer referenced by the session if any, otherwise
    ///   defaults to a stub `Customer::new("anonymous")`.
    /// - Reads the session's events and projects them into an
    ///   [`Interaction`].
    /// - Wires both the session and response `EventEmitter`s from the
    ///   supplied `Arc`s.
    pub async fn from_queries(
        queries: &loon_core::entity_cq::EntityQueries,
        info: Context,
        session_event_emitter: Arc<dyn EventEmitter>,
        response_event_emitter: Arc<dyn EventEmitter>,
        logger: Arc<dyn Logger>,
        tracer: Arc<dyn Tracer>,
    ) -> EngineResult<Self> {
        let agent = queries
            .read_agent(&info.agent_id)
            .await
            .map_err(|e| crate::error::EngineError::ContextLoadFailed(e.to_string()))?;
        let session = queries
            .read_session(&info.session_id)
            .await
            .map_err(|e| crate::error::EngineError::ContextLoadFailed(e.to_string()))?;
        let customer = if let Some(cid) = &session.customer_id {
            queries
                .read_customer(cid)
                .await
                .map_err(|e| crate::error::EngineError::ContextLoadFailed(e.to_string()))?
        } else {
            loon_core::Customer::new("anonymous")
        };
        let events = queries
            .find_events(&info.session_id)
            .await
            .map_err(|e| crate::error::EngineError::ContextLoadFailed(e.to_string()))?;
        let interaction = Interaction::new(events);
        Ok(Self {
            info,
            logger,
            tracer,
            agent,
            customer,
            session,
            session_event_emitter,
            response_event_emitter,
            interaction,
            state: parking_lot::Mutex::new(ResponseState::default()),
            creation: loon_core::Stopwatch::start(),
        })
    }

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

/// No-op `EventEmitter` used by `EngineContext::placeholder` and
/// wired in by `AlphaEngine` whenever it needs an emitter that
/// satisfies the trait without performing any side effects.
pub struct NoopEmitter;
#[async_trait]
impl loon_emission::EventEmitter for NoopEmitter {
    async fn emit_status_event(
        &self,
        _trace_id: &str,
        _data: loon_core::StatusEventData,
        _metadata: Option<std::collections::HashMap<String, loon_core::JsonValue>>,
    ) -> loon_emission::EmissionResult<loon_emission::EmittedEvent> {
        Ok(loon_emission::EmittedEvent {
            source: loon_core::EventSource::System,
            kind: loon_core::EventKind::Status,
            trace_id: String::new(),
            data: loon_core::JsonValue::Null,
            metadata: None,
        })
    }
    async fn emit_message_event(
        &self,
        _trace_id: &str,
        _data: loon_emission::MessageEmitData,
        _metadata: Option<std::collections::HashMap<String, loon_core::JsonValue>>,
    ) -> loon_emission::EmissionResult<loon_emission::MessageEventHandle> {
        use loon_core::async_utils::BoxFuture;
        let update: loon_emission::EventUpdater = std::sync::Arc::new(|_d| {
            let fut: BoxFuture<'static, loon_emission::EmissionResult<loon_emission::MessageEventHandle>> =
                Box::pin(async {
                    Err(loon_emission::EmissionError::Serialization("noop".into()))
                });
            fut
        });
        Ok(loon_emission::MessageEventHandle {
            event: loon_emission::EmittedEvent {
                source: loon_core::EventSource::AiAgent,
                kind: loon_core::EventKind::Message,
                trace_id: String::new(),
                data: loon_core::JsonValue::Null,
                metadata: None,
            },
            update,
        })
    }
    async fn emit_tool_event(
        &self,
        _trace_id: &str,
        _data: loon_core::ToolEventData,
        _metadata: Option<std::collections::HashMap<String, loon_core::JsonValue>>,
    ) -> loon_emission::EmissionResult<loon_emission::EmittedEvent> {
        Ok(loon_emission::EmittedEvent {
            source: loon_core::EventSource::System,
            kind: loon_core::EventKind::Tool,
            trace_id: String::new(),
            data: loon_core::JsonValue::Null,
            metadata: None,
        })
    }
    async fn emit_custom_event(
        &self,
        _trace_id: &str,
        _data: loon_core::JsonValue,
        _metadata: Option<std::collections::HashMap<String, loon_core::JsonValue>>,
    ) -> loon_emission::EmissionResult<loon_emission::EmittedEvent> {
        Ok(loon_emission::EmittedEvent {
            source: loon_core::EventSource::System,
            kind: loon_core::EventKind::Custom,
            trace_id: String::new(),
            data: loon_core::JsonValue::Null,
            metadata: None,
        })
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

    #[test]
    fn placeholder_constructs_with_defaults() {
        let ctx = EngineContext::placeholder();
        assert_eq!(ctx.agent.name, "placeholder");
        assert_eq!(ctx.customer.name, "placeholder");
        // Verify state mutex initializes empty
        assert!(ctx.state.lock().iterations.is_empty());
        assert!(!ctx.state.lock().prepared_to_respond);
    }

    #[tokio::test]
    async fn from_queries_loads_real_context() {
        use loon_core::basic_tracer::BasicTracer;
        use loon_core::console_logger::ConsoleLogger;
        use loon_core::entity_cq::EntityQueries;
        use loon_core::{Agent, Session};

        let queries = EntityQueries::in_memory();
        let agent = Agent::new("agent", "test agent");
        let agent_id = agent.id.clone();
        queries.agent_store.create(agent).await.unwrap();
        let session = Session::new(&agent_id);
        let session_id = session.id.clone();
        queries.session_store.create(session).await.unwrap();

        let info = Context {
            session_id: session_id.clone(),
            agent_id: agent_id.clone(),
        };
        let logger: Arc<dyn Logger> = Arc::new(ConsoleLogger);
        let tracer: Arc<dyn Tracer> = Arc::new(BasicTracer::new());
        let emitter: Arc<dyn loon_emission::EventEmitter> = Arc::new(NoopEmitter);

        let ctx = EngineContext::from_queries(
            &queries,
            info,
            emitter.clone(),
            emitter,
            logger,
            tracer,
        )
        .await
        .unwrap();

        assert_eq!(ctx.agent.name, "agent");
        assert_eq!(ctx.customer.name, "anonymous");
        assert_eq!(ctx.interaction.events.len(), 0);
        assert_eq!(ctx.info.session_id, session_id);
        assert_eq!(ctx.info.agent_id, agent_id);
    }
}

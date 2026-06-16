//! `MessageGenerator` — produces a final `MessageEventData` for an
//! `EngineContext` in three modes: fluid (LLM-generated), strict
//! (canned-response), and streaming.

use std::pin::Pin;
use std::sync::Arc;

use futures::stream::{self, Stream};
use loon_core::{CannedResponse, MessageEventData, Participant};
use loon_nlp::NlpService;

use crate::canned_response_generator::CannedResponseGenerator;
use crate::engine_context::{EngineContext, Interaction};
use crate::error::EngineResult;
use crate::prompt_builder::PromptBuilder;

/// Produces the agent's reply for a fully-prepared `EngineContext`.
pub struct MessageGenerator {
    pub nlp: Arc<dyn NlpService>,
    pub prompt_builder: Arc<PromptBuilder>,
    pub canned_response_generator: Arc<CannedResponseGenerator>,
}

impl MessageGenerator {
    pub fn new(
        nlp: Arc<dyn NlpService>,
        prompt_builder: Arc<PromptBuilder>,
        canned_response_generator: Arc<CannedResponseGenerator>,
    ) -> Self {
        Self {
            nlp,
            prompt_builder,
            canned_response_generator,
        }
    }
}

impl MessageGenerator {
    /// Fluid composition: produce a free-form LLM-generated
    /// message. Phase 1 returns a static placeholder reply.
    pub async fn generate_fluid_message(
        &self,
        _ctx: &EngineContext,
    ) -> EngineResult<Vec<MessageEventData>> {
        Ok(vec![MessageEventData {
            message: "Hello, how can I help?".into(),
            participant: Participant::default(),
            updated: false,
        }])
    }

    /// Strict composition: pick a canned response, fill its
    /// template, and emit the result. Falls back to fluid if no
    /// canned responses are available.
    pub async fn generate_strict_message(
        &self,
        ctx: &EngineContext,
        canned_responses: &[CannedResponse],
    ) -> EngineResult<Vec<MessageEventData>> {
        let agent = loon_core::Agent::new("a", "b");
        let interaction = Interaction::new(vec![]);
        if let Some(sel) = self
            .canned_response_generator
            .select_best(canned_responses, "", &agent, &interaction)
            .await?
        {
            let filled = CannedResponseGenerator::fill_template(
                &sel.canned_response.value,
                &sel.filled_fields,
            );
            return Ok(vec![MessageEventData {
                message: filled,
                participant: Participant::default(),
                updated: false,
            }]);
        }
        self.generate_fluid_message(ctx).await
    }

    /// Streaming composition: returns a stream of text chunks
    /// emitted token-by-token. Phase 1 returns a fixed two-chunk
    /// stream.
    pub async fn generate_streaming(
        &self,
        _ctx: &EngineContext,
    ) -> EngineResult<Pin<Box<dyn Stream<Item = EngineResult<String>> + Send>>> {
        let s = stream::iter(vec![Ok("Hello, ".to_string()), Ok("how can I help?".to_string())]);
        Ok(Box::pin(s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use loon_core::async_utils::BoxFuture;
    use loon_core::basic_tracer::BasicTracer;
    use loon_core::console_logger::ConsoleLogger;
    use loon_core::{
        AgentId, Customer, Session, SessionId, StatusEventData, ToolEventData,
    };
    use loon_emission::{
        EmissionResult, EmittedEvent, EventEmitter, MessageEmitData, MessageEventHandle,
    };
    use loon_nlp::{
        Embedder, ErasedSchematicGenerator, Moderater, NlpConfig, NlpResult, Tokenizer,
        StreamingTextGenerator,
    };
    use std::collections::HashMap;

    struct DummyNlp;
    #[async_trait]
    impl NlpService for DummyNlp {
        fn config(&self) -> &NlpConfig {
            unimplemented!()
        }
        async fn text_generator(&self) -> NlpResult<Box<dyn StreamingTextGenerator>> {
            unimplemented!()
        }
        async fn schematic_generator(
            &self,
            _: serde_json::Value,
        ) -> NlpResult<Box<dyn ErasedSchematicGenerator>> {
            unimplemented!()
        }
        async fn embedder(&self) -> NlpResult<Box<dyn Embedder>> {
            unimplemented!()
        }
        async fn tokenizer(&self) -> NlpResult<Box<dyn Tokenizer>> {
            unimplemented!()
        }
        async fn moderater(&self) -> NlpResult<Box<dyn Moderater>> {
            unimplemented!()
        }
    }

    struct WsTok;
    #[async_trait]
    impl Tokenizer for WsTok {
        async fn count_tokens(&self, text: &str) -> NlpResult<u32> {
            Ok(text.split_whitespace().count() as u32)
        }
    }

    struct NoopEmitter;
    #[async_trait]
    impl EventEmitter for NoopEmitter {
        async fn emit_status_event(
            &self,
            _trace_id: &str,
            _data: StatusEventData,
            _metadata: Option<HashMap<String, serde_json::Value>>,
        ) -> EmissionResult<EmittedEvent> {
            Ok(EmittedEvent {
                source: loon_core::EventSource::System,
                kind: loon_core::EventKind::Status,
                trace_id: "t".into(),
                data: serde_json::Value::Null,
                metadata: None,
            })
        }
        async fn emit_message_event(
            &self,
            _trace_id: &str,
            _data: MessageEmitData,
            _metadata: Option<HashMap<String, serde_json::Value>>,
        ) -> EmissionResult<MessageEventHandle> {
            let update: loon_emission::EventUpdater = Arc::new(|_d| {
                let fut: BoxFuture<'static, EmissionResult<MessageEventHandle>> = Box::pin(async {
                    Err(loon_emission::EmissionError::Serialization("noop".into()))
                });
                fut
            });
            Ok(MessageEventHandle {
                event: EmittedEvent {
                    source: loon_core::EventSource::AiAgent,
                    kind: loon_core::EventKind::Message,
                    trace_id: "t".into(),
                    data: serde_json::Value::Null,
                    metadata: None,
                },
                update,
            })
        }
        async fn emit_tool_event(
            &self,
            _trace_id: &str,
            _data: ToolEventData,
            _metadata: Option<HashMap<String, serde_json::Value>>,
        ) -> EmissionResult<EmittedEvent> {
            Ok(EmittedEvent {
                source: loon_core::EventSource::System,
                kind: loon_core::EventKind::Tool,
                trace_id: "t".into(),
                data: serde_json::Value::Null,
                metadata: None,
            })
        }
        async fn emit_custom_event(
            &self,
            _trace_id: &str,
            _data: serde_json::Value,
            _metadata: Option<HashMap<String, serde_json::Value>>,
        ) -> EmissionResult<EmittedEvent> {
            Ok(EmittedEvent {
                source: loon_core::EventSource::System,
                kind: loon_core::EventKind::Custom,
                trace_id: "t".into(),
                data: serde_json::Value::Null,
                metadata: None,
            })
        }
    }

    fn mk_gen() -> MessageGenerator {
        let nlp: Arc<dyn NlpService> = Arc::new(DummyNlp);
        let tok: Arc<dyn Tokenizer> = Arc::new(WsTok);
        let pb = Arc::new(PromptBuilder::new(tok, 1000));
        let crg = Arc::new(CannedResponseGenerator::new(nlp.clone()));
        MessageGenerator::new(nlp, pb, crg)
    }

    fn mk_ctx() -> EngineContext {
        let agent_id = AgentId::new();
        EngineContext {
            info: crate::engine_context::Context {
                session_id: SessionId::new(),
                agent_id: agent_id.clone(),
            },
            logger: Arc::new(ConsoleLogger),
            tracer: Arc::new(BasicTracer::new()),
            agent: loon_core::Agent::new("a", "b"),
            customer: Customer::new("alice"),
            session: Session::new(&agent_id),
            session_event_emitter: Arc::new(NoopEmitter),
            response_event_emitter: Arc::new(NoopEmitter),
            interaction: Interaction::new(vec![]),
            state: parking_lot::Mutex::new(crate::engine_context::ResponseState::default()),
            creation: loon_core::Stopwatch::start(),
        }
    }

    #[tokio::test]
    async fn generate_fluid_message_returns_non_empty_vec() {
        let gen = mk_gen();
        let ctx = mk_ctx();
        let msgs = gen.generate_fluid_message(&ctx).await.unwrap();
        assert!(!msgs.is_empty());
        assert_eq!(msgs[0].message, "Hello, how can I help?");
    }
}

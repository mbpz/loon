//! `AlphaEngine` — the Phase-1 concrete implementation of the
//! `Engine` trait. Wires every strategy trait (matcher, tool caller,
//! planner, message generator, etc.) into a single struct and
//! drives a 4-stage pipeline (acknowledge, prepare, generate,
//! emit).

use std::sync::Arc;

use async_trait::async_trait;
use loon_core::entity_cq::{EntityCommands, EntityQueries};
use loon_core::stores::SessionStore;
use loon_emission::EventEmitter;
use loon_nlp::NlpService;

use crate::engine::{Engine, UtteranceRequest};
use crate::engine_context::Context;
use crate::error::EngineResult;
use crate::guideline_matching::GuidelineMatcher;
use crate::hooks::EngineHooks;
use crate::message_generator::MessageGenerator;
use crate::optimization_policy::OptimizationPolicy;
use crate::perceived_performance_policy::PerceivedPerformancePolicy;
use crate::planner::Planner;
use crate::relational_resolver::RelationalResolver;
use crate::tool_calling::ToolCaller;

/// Default `Engine` implementation. The full pipeline lands in
/// Task 6.13; for now `process` always returns `Ok(true)` and
/// `utter` always returns `Ok(false)`.
pub struct AlphaEngine {
    pub queries: Arc<EntityQueries>,
    pub commands: Arc<EntityCommands>,
    pub matcher: Arc<dyn GuidelineMatcher>,
    pub tool_caller: Arc<dyn ToolCaller>,
    pub planner: Arc<dyn Planner>,
    pub message_generator: Arc<MessageGenerator>,
    pub relational_resolver: Arc<RelationalResolver>,
    pub hooks: EngineHooks,
    pub optimization_policy: Arc<dyn OptimizationPolicy>,
    pub performance_policy: Arc<PerceivedPerformancePolicy>,
    pub session_store: Arc<dyn SessionStore>,
    pub nlp: Arc<dyn NlpService>,
}

#[async_trait]
impl Engine for AlphaEngine {
    /// 4-stage pipeline:
    ///   0. acknowledge (status event)
    ///   1. load agent + session
    ///   2. context fill (parallel queries)
    ///   3. preparation loop (max 5 iterations)
    ///   4. generate message (fluid or strict)
    ///   5. emit message events
    ///   6. "done" status
    async fn process(
        &self,
        context: &Context,
        event_emitter: &dyn EventEmitter,
    ) -> EngineResult<bool> {
        use loon_core::{MessageOutputMode, Participant, StatusEventData};
        use loon_emission::MessageEmitData;

        let trace_id = context.session_id.0.clone();

        // 0. Acknowledge
        let _ = event_emitter
            .emit_status_event(
                &trace_id,
                StatusEventData {
                    stage: "acknowledging".into(),
                    details: None,
                },
                None,
            )
            .await;

        // 1. Load agent + session
        let agent = self.queries.read_agent(&context.agent_id).await?;
        let session = self.queries.read_session(&context.session_id).await?;

        // 2. Context fill (parallel)
        let (guidelines, _ctx_vars, _journeys, _capabilities, canned_responses) = tokio::try_join!(
            self.queries.find_guidelines_for_context(&agent.id, &[]),
            self.queries.find_context_variables_for_context(&agent.id),
            self.queries.find_journeys_for_context(&agent.id),
            self.queries.find_capabilities_for_agent(&agent.id, "", 5),
            async { Ok::<_, loon_core::CoreError>(Vec::<loon_core::CannedResponse>::new()) },
        )?;

        // 3. Preparation loop (max 5 iterations)
        let mut iterations = 0;
        loop {
            iterations += 1;
            if iterations > 5 {
                break;
            }
            let match_ctx = crate::guideline_matching::GuidelineMatchingContext {
                agent: agent.clone(),
                session: session.clone(),
                interaction: crate::engine_context::Interaction::new(vec![]),
                guidelines: guidelines.clone(),
                glossary_terms: vec![],
                nlp: self.nlp.clone(),
            };
            let matched = self.matcher.match_guidelines(&match_ctx).await?;
            let resolved = self
                .relational_resolver
                .resolve(matched, &guidelines)
                .await?;

            let insights = self
                .tool_caller
                .generate_insights(&empty_engine_context(), &resolved)
                .await?;
            let executed = self
                .tool_caller
                .call_tools(&empty_engine_context(), &insights)
                .await?;
            if executed.is_empty() {
                // Persist the resolved list on the response state so
                // message generation can read it. Phase 1 just
                // lets the loop exit; a future phase will use the
                // resolved matches to drive composition.
                let _usable: Vec<loon_core::Guideline> =
                    resolved.iter().map(|m| m.guideline.clone()).collect();
                break;
            }
        }

        // 4. Generate message
        let messages = match agent.message_output_mode {
            MessageOutputMode::Canned => {
                self.message_generator
                    .generate_strict_message(&empty_engine_context(), &canned_responses)
                    .await?
            }
            MessageOutputMode::Fluid => {
                self.message_generator
                    .generate_fluid_message(&empty_engine_context())
                    .await?
            }
        };

        // 5. Emit message events
        for m in messages {
            let _ = event_emitter
                .emit_message_event(
                    &trace_id,
                    MessageEmitData::Structured(loon_core::MessageEventData {
                        message: m.message,
                        participant: Participant::default(),
                        updated: m.updated,
                    }),
                    None,
                )
                .await;
        }

        // 6. Done
        let _ = event_emitter
            .emit_status_event(
                &trace_id,
                StatusEventData {
                    stage: "done".into(),
                    details: None,
                },
                None,
            )
            .await;

        Ok(true)
    }

    async fn utter(
        &self,
        _context: &Context,
        _event_emitter: &dyn EventEmitter,
        _requests: &[UtteranceRequest],
    ) -> EngineResult<bool> {
        Ok(false)
    }
}

/// Build a no-op `EngineContext` for use during preparation /
/// generation calls. Mirrors the test-side helper in
/// `message_generator.rs` so the engine can keep advancing through
/// the pipeline without spinning up a real request context.
fn empty_engine_context() -> crate::engine_context::EngineContext {
    crate::engine_context::EngineContext::placeholder()
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use loon_core::stores::{
        AgentStore, CannedResponseStore, CapabilityStore, ContextVariableStore, CustomerStore,
        GlossaryStore, GuidelineStore, GuidelineToolAssociationStore, JourneyStore,
        RelationshipStore, RetrieverStore, SessionStore,
    };
    use loon_core::{
        async_utils::BoxFuture, AgentId, CoreError, CoreResult, Guideline, SessionId,
        SessionUpdateParams,
    };
    use loon_emission::{
        EmissionResult, EmittedEvent, EventEmitter, MessageEmitData, MessageEventHandle,
    };
    use loon_nlp::{
        Embedder, ErasedSchematicGenerator, Moderater, NlpConfig, NlpResult,
        StreamingTextGenerator, Tokenizer,
    };
    use std::collections::HashMap;

    use crate::engine_context::{EngineContext, GuidelineMatch, ToolInsights};
    use crate::guideline_matching::GuidelineMatchingContext;
    use crate::planner::{Plan, Planner as PlannerTrait};

    // ---- Stub stores ---------------------------------------------------

    /// Agent store that returns a `Default` agent for any read so the
    /// engine can load the agent referenced by the request `Context`.
    struct EmptyAgentStore;
    #[async_trait]
    impl AgentStore for EmptyAgentStore {
        async fn create(&self, a: loon_core::Agent) -> CoreResult<loon_core::Agent> {
            Ok(a)
        }
        async fn read(&self, _id: &AgentId) -> CoreResult<Option<loon_core::Agent>> {
            Ok(Some(loon_core::Agent::new("stub", "stub")))
        }
        async fn update(
            &self,
            _id: &AgentId,
            _p: loon_core::AgentUpdateParams,
        ) -> CoreResult<loon_core::Agent> {
            Err(CoreError::Internal("n/a".into()))
        }
        async fn delete(&self, _id: &AgentId) -> CoreResult<()> {
            Ok(())
        }
        async fn list(&self, _tags: &[loon_core::TagId]) -> CoreResult<Vec<loon_core::Agent>> {
            Ok(vec![])
        }
    }

    #[allow(unused_macros)]
    macro_rules! empty_store {
        ($name:ident, $trait:ident, $create_ret:ty) => {
            struct $name;
            #[async_trait]
            impl $trait for $name {
                async fn create(&self, x: $create_ret) -> CoreResult<$create_ret> {
                    Ok(x)
                }
                async fn read(&self, _: &loon_core::SessionId) -> CoreResult<Option<$create_ret>> {
                    Ok(None)
                }
                async fn update(
                    &self,
                    _: &loon_core::SessionId,
                    _: SessionUpdateParams,
                ) -> CoreResult<$create_ret> {
                    Err(CoreError::Internal("n/a".into()))
                }
                async fn delete(&self, _: &loon_core::SessionId) -> CoreResult<()> {
                    Ok(())
                }
                async fn list(&self, _: &AgentId) -> CoreResult<Vec<$create_ret>> {
                    Ok(vec![])
                }
                async fn find_events(&self, _: &SessionId) -> CoreResult<Vec<loon_core::Event>> {
                    Ok(vec![])
                }
            }
        };
    }

    struct EmptySessionStore;
    #[async_trait]
    impl SessionStore for EmptySessionStore {
        async fn create(&self, s: loon_core::Session) -> CoreResult<loon_core::Session> {
            Ok(s)
        }
        async fn read(&self, id: &SessionId) -> CoreResult<Option<loon_core::Session>> {
            Ok(Some(loon_core::Session::new(&AgentId::new())))
        }
        async fn update(
            &self,
            _: &SessionId,
            _: SessionUpdateParams,
        ) -> CoreResult<loon_core::Session> {
            Err(CoreError::Internal("n/a".into()))
        }
        async fn delete(&self, _: &SessionId) -> CoreResult<()> {
            Ok(())
        }
        async fn list(
            &self,
            _: Option<&AgentId>,
            _: Option<&loon_core::CustomerId>,
        ) -> CoreResult<Vec<loon_core::Session>> {
            Ok(vec![])
        }
        async fn create_event(
            &self,
            _: SessionId,
            _: loon_core::Event,
        ) -> CoreResult<loon_core::Event> {
            Err(CoreError::Internal("n/a".into()))
        }
        async fn update_event(
            &self,
            _: &SessionId,
            _: &loon_core::EventId,
            _: loon_core::EventUpdateParams,
        ) -> CoreResult<loon_core::Event> {
            Err(CoreError::Internal("n/a".into()))
        }
        async fn read_events(&self, _: &SessionId) -> CoreResult<Vec<loon_core::Event>> {
            Ok(vec![])
        }
        async fn find_events(&self, _: &SessionId) -> CoreResult<Vec<loon_core::Event>> {
            Ok(vec![])
        }
    }

    struct EmptyGuidelineStore;
    #[async_trait]
    impl GuidelineStore for EmptyGuidelineStore {
        async fn create(&self, g: Guideline) -> CoreResult<Guideline> {
            Ok(g)
        }
        async fn read(&self, _: &loon_core::GuidelineId) -> CoreResult<Option<Guideline>> {
            Ok(None)
        }
        async fn update(
            &self,
            _: &loon_core::GuidelineId,
            _: loon_core::GuidelineUpdateParams,
        ) -> CoreResult<Guideline> {
            Err(CoreError::Internal("n/a".into()))
        }
        async fn delete(&self, _: &loon_core::GuidelineId) -> CoreResult<()> {
            Ok(())
        }
        async fn list(&self, _: &AgentId, _: &[loon_core::TagId]) -> CoreResult<Vec<Guideline>> {
            Ok(vec![])
        }
    }

    struct EmptyCustomerStore;
    #[async_trait]
    impl CustomerStore for EmptyCustomerStore {
        async fn create(&self, c: loon_core::Customer) -> CoreResult<loon_core::Customer> {
            Ok(c)
        }
        async fn read(&self, _: &loon_core::CustomerId) -> CoreResult<Option<loon_core::Customer>> {
            Ok(None)
        }
        async fn update(
            &self,
            _: &loon_core::CustomerId,
            _: loon_core::CustomerUpdateParams,
        ) -> CoreResult<loon_core::Customer> {
            Err(CoreError::Internal("n/a".into()))
        }
        async fn delete(&self, _: &loon_core::CustomerId) -> CoreResult<()> {
            Ok(())
        }
        async fn list(&self, _: &[loon_core::TagId]) -> CoreResult<Vec<loon_core::Customer>> {
            Ok(vec![])
        }
    }

    struct EmptyCtxVarStore;
    #[async_trait]
    impl ContextVariableStore for EmptyCtxVarStore {
        async fn create(
            &self,
            v: loon_core::ContextVariable,
        ) -> CoreResult<loon_core::ContextVariable> {
            Ok(v)
        }
        async fn read(
            &self,
            _: &loon_core::ContextVariableId,
        ) -> CoreResult<Option<loon_core::ContextVariable>> {
            Ok(None)
        }
        async fn update(
            &self,
            _: &loon_core::ContextVariableId,
            _: loon_core::ContextVariableUpdateParams,
        ) -> CoreResult<loon_core::ContextVariable> {
            Err(CoreError::Internal("n/a".into()))
        }
        async fn delete(&self, _: &loon_core::ContextVariableId) -> CoreResult<()> {
            Ok(())
        }
        async fn list(&self, _: &AgentId) -> CoreResult<Vec<loon_core::ContextVariable>> {
            Ok(vec![])
        }
        async fn upsert_value(
            &self,
            _: &loon_core::ContextVariableId,
            _: &str,
            _: loon_core::JsonValue,
        ) -> CoreResult<loon_core::ContextVariableValue> {
            Err(CoreError::Internal("n/a".into()))
        }
    }

    struct EmptyRelStore;
    #[async_trait]
    impl RelationshipStore for EmptyRelStore {
        async fn create(&self, r: loon_core::Relationship) -> CoreResult<loon_core::Relationship> {
            Ok(r)
        }
        async fn read(
            &self,
            _: &loon_core::RelationshipId,
        ) -> CoreResult<Option<loon_core::Relationship>> {
            Ok(None)
        }
        async fn delete(&self, _: &loon_core::RelationshipId) -> CoreResult<()> {
            Ok(())
        }
        async fn list_for(
            &self,
            _: &loon_core::RelationshipEntity,
        ) -> CoreResult<Vec<loon_core::Relationship>> {
            Ok(vec![])
        }
    }

    struct EmptyGtStore;
    #[async_trait]
    impl GuidelineToolAssociationStore for EmptyGtStore {
        async fn create(
            &self,
            a: loon_core::GuidelineToolAssociation,
        ) -> CoreResult<loon_core::GuidelineToolAssociation> {
            Ok(a)
        }
        async fn read(
            &self,
            _: &loon_core::GuidelineToolAssociationId,
        ) -> CoreResult<Option<loon_core::GuidelineToolAssociation>> {
            Ok(None)
        }
        async fn delete(&self, _: &loon_core::GuidelineToolAssociationId) -> CoreResult<()> {
            Ok(())
        }
        async fn list_for_tool(
            &self,
            _: &loon_core::ToolId,
        ) -> CoreResult<Vec<loon_core::GuidelineToolAssociation>> {
            Ok(vec![])
        }
        async fn list_for_guideline(
            &self,
            _: &loon_core::GuidelineId,
        ) -> CoreResult<Vec<loon_core::GuidelineToolAssociation>> {
            Ok(vec![])
        }
    }

    struct EmptyGlossary;
    #[async_trait]
    impl GlossaryStore for EmptyGlossary {
        async fn create_term(&self, t: loon_core::Term) -> CoreResult<loon_core::Term> {
            Ok(t)
        }
        async fn read_term(
            &self,
            _: &loon_core::GlossaryTermId,
        ) -> CoreResult<Option<loon_core::Term>> {
            Ok(None)
        }
        async fn update_term(
            &self,
            _: &loon_core::GlossaryTermId,
            _: loon_core::Term,
        ) -> CoreResult<loon_core::Term> {
            Err(CoreError::Internal("n/a".into()))
        }
        async fn delete_term(&self, _: &loon_core::GlossaryTermId) -> CoreResult<()> {
            Ok(())
        }
        async fn list_terms(&self, _: &AgentId) -> CoreResult<Vec<loon_core::Term>> {
            Ok(vec![])
        }
    }

    struct EmptyJourney;
    #[async_trait]
    impl JourneyStore for EmptyJourney {
        async fn create(&self, j: loon_core::Journey) -> CoreResult<loon_core::Journey> {
            Ok(j)
        }
        async fn read(&self, _: &loon_core::JourneyId) -> CoreResult<Option<loon_core::Journey>> {
            Ok(None)
        }
        async fn update(
            &self,
            _: &loon_core::JourneyId,
            _: loon_core::JourneyUpdateParams,
        ) -> CoreResult<loon_core::Journey> {
            Err(CoreError::Internal("n/a".into()))
        }
        async fn delete(&self, _: &loon_core::JourneyId) -> CoreResult<()> {
            Ok(())
        }
        async fn list(&self, _: &AgentId) -> CoreResult<Vec<loon_core::Journey>> {
            Ok(vec![])
        }
    }

    struct EmptyCanned;
    #[async_trait]
    impl CannedResponseStore for EmptyCanned {
        async fn create(
            &self,
            c: loon_core::CannedResponse,
        ) -> CoreResult<loon_core::CannedResponse> {
            Ok(c)
        }
        async fn read(
            &self,
            _: &loon_core::CannedResponseId,
        ) -> CoreResult<Option<loon_core::CannedResponse>> {
            Ok(None)
        }
        async fn update(
            &self,
            _: &loon_core::CannedResponseId,
            _: loon_core::CannedResponseUpdateParams,
        ) -> CoreResult<loon_core::CannedResponse> {
            Err(CoreError::Internal("n/a".into()))
        }
        async fn delete(&self, _: &loon_core::CannedResponseId) -> CoreResult<()> {
            Ok(())
        }
        async fn list(&self, _: &AgentId) -> CoreResult<Vec<loon_core::CannedResponse>> {
            Ok(vec![])
        }
    }

    struct EmptyCap;
    #[async_trait]
    impl CapabilityStore for EmptyCap {
        async fn create(&self, c: loon_core::Capability) -> CoreResult<loon_core::Capability> {
            Ok(c)
        }
        async fn read(
            &self,
            _: &loon_core::CapabilityId,
        ) -> CoreResult<Option<loon_core::Capability>> {
            Ok(None)
        }
        async fn update(
            &self,
            _: &loon_core::CapabilityId,
            _: loon_core::CapabilityUpdateParams,
        ) -> CoreResult<loon_core::Capability> {
            Err(CoreError::Internal("n/a".into()))
        }
        async fn delete(&self, _: &loon_core::CapabilityId) -> CoreResult<()> {
            Ok(())
        }
        async fn list(&self, _: &AgentId) -> CoreResult<Vec<loon_core::Capability>> {
            Ok(vec![])
        }
    }

    struct EmptyRetriever;
    #[async_trait]
    impl RetrieverStore for EmptyRetriever {
        async fn create(&self, r: loon_core::Retriever) -> CoreResult<loon_core::Retriever> {
            Ok(r)
        }
        async fn read(
            &self,
            _: &loon_core::RetrieverId,
        ) -> CoreResult<Option<loon_core::Retriever>> {
            Ok(None)
        }
        async fn delete(&self, _: &loon_core::RetrieverId) -> CoreResult<()> {
            Ok(())
        }
        async fn list(&self, _: &AgentId) -> CoreResult<Vec<loon_core::Retriever>> {
            Ok(vec![])
        }
    }

    // ---- Strategy stubs ------------------------------------------------

    struct EmptyMatcher;
    #[async_trait]
    impl GuidelineMatcher for EmptyMatcher {
        async fn match_guidelines(
            &self,
            _: &GuidelineMatchingContext,
        ) -> EngineResult<Vec<GuidelineMatch>> {
            Ok(vec![])
        }
    }

    struct EmptyToolCaller;
    #[async_trait]
    impl ToolCaller for EmptyToolCaller {
        async fn generate_insights(
            &self,
            _: &EngineContext,
            _: &[GuidelineMatch],
        ) -> EngineResult<ToolInsights> {
            Ok(ToolInsights::default())
        }
        async fn call_tools(
            &self,
            _: &EngineContext,
            _: &ToolInsights,
        ) -> EngineResult<Vec<crate::tool_calling::caller::ToolExecutionResult>> {
            Ok(vec![])
        }
    }

    struct DonePlanner;
    #[async_trait]
    impl PlannerTrait for DonePlanner {
        async fn plan(&self, _: &EngineContext) -> EngineResult<Plan> {
            Ok(Plan::Done)
        }
    }

    struct StubNlp;
    #[async_trait]
    impl NlpService for StubNlp {
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

    struct NoopEmitter;
    #[async_trait]
    impl EventEmitter for NoopEmitter {
        async fn emit_status_event(
            &self,
            _: &str,
            _: loon_core::StatusEventData,
            _: Option<HashMap<String, serde_json::Value>>,
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
            _: &str,
            _: MessageEmitData,
            _: Option<HashMap<String, serde_json::Value>>,
        ) -> EmissionResult<MessageEventHandle> {
            let update: loon_emission::EventUpdater = Arc::new(|_| {
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
            _: &str,
            _: loon_core::ToolEventData,
            _: Option<HashMap<String, serde_json::Value>>,
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
            _: &str,
            _: serde_json::Value,
            _: Option<HashMap<String, serde_json::Value>>,
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

    fn make_queries() -> Arc<EntityQueries> {
        let jgp = Arc::new(
            loon_core::journey_guideline_projection::JourneyGuidelineProjection::new(
                Arc::new(EmptyJourney),
                Arc::new(EmptyGuidelineStore),
            ),
        );
        Arc::new(EntityQueries {
            agent_store: Arc::new(EmptyAgentStore),
            session_store: Arc::new(EmptySessionStore),
            guideline_store: Arc::new(EmptyGuidelineStore),
            customer_store: Arc::new(EmptyCustomerStore),
            context_variable_store: Arc::new(EmptyCtxVarStore),
            relationship_store: Arc::new(EmptyRelStore),
            guideline_tool_association_store: Arc::new(EmptyGtStore),
            glossary_store: Arc::new(EmptyGlossary),
            journey_store: Arc::new(EmptyJourney),
            canned_response_store: Arc::new(EmptyCanned),
            capability_store: Arc::new(EmptyCap),
            retriever_store: Arc::new(EmptyRetriever),
            journey_guideline_projection: jgp,
        })
    }

    fn make_commands() -> Arc<EntityCommands> {
        Arc::new(EntityCommands {
            session_store: Arc::new(EmptySessionStore),
            context_variable_store: Arc::new(EmptyCtxVarStore),
        })
    }

    fn make_engine() -> AlphaEngine {
        let nlp: Arc<dyn NlpService> = Arc::new(StubNlp);
        let matcher: Arc<dyn GuidelineMatcher> = Arc::new(EmptyMatcher);
        let tool_caller: Arc<dyn ToolCaller> = Arc::new(EmptyToolCaller);
        let planner: Arc<dyn Planner> = Arc::new(DonePlanner);
        let resolver = Arc::new(RelationalResolver::new(Arc::new(EmptyRelStore)));

        // Use a minimal MessageGenerator built via the public constructor
        // with stub NLP, prompt builder, and canned response generator.
        let prompt_builder = Arc::new(crate::prompt_builder::PromptBuilder::new(
            // Tokenizer is dyn Tokenizer; supply a whitespace tokenizer.
            {
                struct WsTok;
                #[async_trait]
                impl Tokenizer for WsTok {
                    async fn count_tokens(&self, text: &str) -> NlpResult<u32> {
                        Ok(text.split_whitespace().count() as u32)
                    }
                }
                Arc::new(WsTok) as Arc<dyn Tokenizer>
            },
            1000,
        ));
        let crg =
            Arc::new(crate::canned_response_generator::CannedResponseGenerator::new(nlp.clone()));
        let message_generator = Arc::new(crate::message_generator::MessageGenerator::new(
            nlp.clone(),
            prompt_builder,
            crg,
        ));

        let optimization_policy: Arc<dyn OptimizationPolicy> =
            Arc::new(crate::optimization_policy::DefaultOptimizationPolicy);
        let performance_policy = Arc::new(PerceivedPerformancePolicy::new());

        AlphaEngine {
            queries: make_queries(),
            commands: make_commands(),
            matcher,
            tool_caller,
            planner,
            message_generator,
            relational_resolver: resolver,
            hooks: EngineHooks::default(),
            optimization_policy,
            performance_policy,
            session_store: Arc::new(EmptySessionStore),
            nlp,
        }
    }

    #[tokio::test]
    async fn alpha_engine_process_returns_true() {
        let engine = make_engine();
        let agent_id = AgentId::new();
        let ctx = Context {
            session_id: SessionId::new(),
            agent_id: agent_id.clone(),
        };
        let emitter = NoopEmitter;
        let result = engine.process(&ctx, &emitter).await.unwrap();
        assert!(result);
    }

    #[tokio::test]
    async fn alpha_engine_utter_returns_false() {
        let engine = make_engine();
        let ctx = Context {
            session_id: SessionId::new(),
            agent_id: AgentId::new(),
        };
        let emitter = NoopEmitter;
        let reqs = vec![UtteranceRequest {
            action: "a".into(),
            rationale: "r".into(),
        }];
        let result = engine.utter(&ctx, &emitter, &reqs).await.unwrap();
        assert!(!result);
    }

    /// EventEmitter that records the kind of every emission so we
    /// can assert the pipeline emits the expected sequence of
    /// status + message events.
    struct CountingEmitter {
        events: parking_lot::Mutex<Vec<String>>,
    }
    #[async_trait]
    impl EventEmitter for CountingEmitter {
        async fn emit_status_event(
            &self,
            _t: &str,
            _d: loon_core::StatusEventData,
            _m: Option<HashMap<String, serde_json::Value>>,
        ) -> EmissionResult<EmittedEvent> {
            self.events.lock().push("status".into());
            Ok(EmittedEvent {
                source: loon_core::EventSource::AiAgent,
                kind: loon_core::EventKind::Status,
                trace_id: "t".into(),
                data: serde_json::Value::Null,
                metadata: None,
            })
        }
        async fn emit_message_event(
            &self,
            _t: &str,
            _d: MessageEmitData,
            _m: Option<HashMap<String, serde_json::Value>>,
        ) -> EmissionResult<MessageEventHandle> {
            self.events.lock().push("message".into());
            let update: loon_emission::EventUpdater = Arc::new(|_d| {
                let fut: loon_core::async_utils::BoxFuture<
                    'static,
                    EmissionResult<MessageEventHandle>,
                > = Box::pin(async {
                    Err(loon_emission::EmissionError::Serialization(
                        "counting".into(),
                    ))
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
            _t: &str,
            _d: loon_core::ToolEventData,
            _m: Option<HashMap<String, serde_json::Value>>,
        ) -> EmissionResult<EmittedEvent> {
            self.events.lock().push("tool".into());
            Ok(EmittedEvent {
                source: loon_core::EventSource::AiAgent,
                kind: loon_core::EventKind::Tool,
                trace_id: "t".into(),
                data: serde_json::Value::Null,
                metadata: None,
            })
        }
        async fn emit_custom_event(
            &self,
            _t: &str,
            _d: serde_json::Value,
            _m: Option<HashMap<String, serde_json::Value>>,
        ) -> EmissionResult<EmittedEvent> {
            self.events.lock().push("custom".into());
            Ok(EmittedEvent {
                source: loon_core::EventSource::System,
                kind: loon_core::EventKind::Custom,
                trace_id: "t".into(),
                data: serde_json::Value::Null,
                metadata: None,
            })
        }
    }

    #[tokio::test]
    async fn alpha_engine_process_emits_message_event() {
        let engine = make_engine();
        let agent_id = AgentId::new();
        let ctx = Context {
            session_id: SessionId::new(),
            agent_id: agent_id.clone(),
        };
        let emitter = CountingEmitter {
            events: parking_lot::Mutex::new(Vec::new()),
        };
        let result = engine.process(&ctx, &emitter).await.unwrap();
        assert!(result);
        let events = emitter.events.lock().clone();
        // The pipeline must emit at least one status event
        // (acknowledging + done) and at least one message event.
        assert!(
            events.iter().any(|e| e == "status"),
            "expected at least one status event, got {:?}",
            events
        );
        assert!(
            events.iter().any(|e| e == "message"),
            "expected at least one message event, got {:?}",
            events
        );
    }
}

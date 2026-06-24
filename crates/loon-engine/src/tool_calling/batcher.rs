//! Default tool-call batcher used by the engine.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;

use loon_core::ServiceRegistry;
use loon_nlp::{define_schematic, ErasedSchematicGenerator, NlpService, Schematic};

use crate::engine_context::{EngineContext, GuidelineMatch, ToolCallEvaluation, ToolInsights};
use crate::error::EngineResult;
use crate::tool_calling::caller::{ToolCaller, ToolExecutionResult};

define_schematic! {
    /// Schematic for the LLM to generate tool-call arguments.
    /// `arguments_json` is a JSON-encoded string matching the tool's
    /// own parameters schema; the engine deserializes it into a
    /// `loon_core::JsonValue` before invoking the tool service.
    pub struct ToolArgsOutput {
        pub arguments_json: String,
        pub rationale: String,
    }
}

pub struct DefaultToolCallBatcher {
    pub nlp: Arc<dyn NlpService>,
    pub registry: Arc<dyn ServiceRegistry>,
    pub queries: Arc<loon_core::entity_cq::EntityQueries>,
    /// Service name to look up in `registry`. Defaults to "local"
    /// which matches `LocalToolService`'s typical registration name.
    pub service_name: String,
}

impl DefaultToolCallBatcher {
    pub fn new(
        nlp: Arc<dyn NlpService>,
        registry: Arc<dyn ServiceRegistry>,
        queries: Arc<loon_core::entity_cq::EntityQueries>,
    ) -> Self {
        Self {
            nlp,
            registry,
            queries,
            service_name: "local".into(),
        }
    }

    /// Prompt the LLM for tool-specific arguments. Returns a
    /// `JsonValue` containing the argument map. Uses
    /// `ToolArgsOutput.arguments_json` (a JSON-encoded string) to
    /// keep the schematic shape stable across tools — the actual
    /// argument shape varies per tool.
    async fn generate_tool_args(
        &self,
        tool: &loon_core::Tool,
        ctx: &EngineContext,
    ) -> EngineResult<loon_core::JsonValue> {
        let last_msg = ctx
            .interaction
            .last_customer_message()
            .map(|m| m.content)
            .unwrap_or_default();

        let prompt = format!(
            "Generate arguments for the `{}` tool.\n\
             Description: {}\n\
             Parameters schema: {}\n\n\
             Customer message: \"{}\"\n\n\
             Return a JSON-encoded string (in `arguments_json`) matching the schema.",
            tool.name,
            tool.description,
            serde_json::to_string(&tool.parameters_schema).unwrap_or_default(),
            last_msg,
        );

        let gen: Box<dyn ErasedSchematicGenerator> = self
            .nlp
            .schematic_generator(ToolArgsOutput::schema())
            .await
            .map_err(|e| crate::error::EngineError::ToolCallFailed(tool.id.clone(), e.to_string()))?;

        let raw = gen
            .generate(prompt, Default::default())
            .await
            .map_err(|e| crate::error::EngineError::ToolCallFailed(tool.id.clone(), e.to_string()))?;

        // The fake/stub generator may return `null`; fall back to an
        // empty `ToolArgsOutput` in that case.
        let parsed: ToolArgsOutput = if raw.value.is_null() {
            ToolArgsOutput::default()
        } else {
            serde_json::from_value(raw.value).map_err(|e| {
                crate::error::EngineError::ToolCallFailed(tool.id.clone(), e.to_string())
            })?
        };

        let args: loon_core::JsonValue = if parsed.arguments_json.is_empty() {
            loon_core::JsonValue::Object(Default::default())
        } else {
            serde_json::from_str(&parsed.arguments_json)
                .unwrap_or(loon_core::JsonValue::Object(Default::default()))
        };

        Ok(args)
    }
}

#[async_trait]
impl ToolCaller for DefaultToolCallBatcher {
    async fn generate_insights(
        &self,
        _ctx: &EngineContext,
        guidelines: &[GuidelineMatch],
    ) -> EngineResult<ToolInsights> {
        let mut evaluations: HashMap<loon_core::ToolId, ToolCallEvaluation> = HashMap::new();
        for m in guidelines {
            let associations = self
                .queries
                .guideline_tool_association_store
                .list_for_guideline(&m.guideline.id)
                .await
                .map_err(|e| crate::error::EngineError::ContextLoadFailed(e.to_string()))?;
            for a in associations {
                // Once we've decided a tool needs to run, don't downgrade it.
                evaluations
                    .entry(a.tool_id)
                    .or_insert(ToolCallEvaluation::NeedsToRun);
            }
        }
        Ok(ToolInsights { evaluations })
    }

    async fn call_tools(
        &self,
        ctx: &EngineContext,
        insights: &ToolInsights,
    ) -> EngineResult<Vec<ToolExecutionResult>> {
        let mut out = Vec::new();
        for (tool_id, eval) in &insights.evaluations {
            if !matches!(eval, ToolCallEvaluation::NeedsToRun) {
                continue;
            }

            // Look up the tool from the store.
            let tool = self
                .queries
                .tool_store
                .read(tool_id)
                .await
                .map_err(|e| {
                    crate::error::EngineError::ToolCallFailed(tool_id.clone(), e.to_string())
                })?;
            let Some(tool) = tool else {
                continue;
            };

            // Prompt LLM for arguments.
            let args = self.generate_tool_args(&tool, ctx).await?;

            // Find the service that hosts this tool.
            let service = self
                .registry
                .read_tool_service(&self.service_name)
                .await
                .map_err(|e| {
                    crate::error::EngineError::ToolCallFailed(tool_id.clone(), e.to_string())
                })?;

            // Construct an invocation-scoped `ToolContext` from the
            // engine context so handlers registered via
            // `LocalToolService::register_handler_with_context` can
            // see which session / agent / customer they're running
            // for. Services without a context-aware handler will
            // transparently fall back to the plain handler via the
            // default `ToolService::call_tool_with_context` impl.
            let tool_ctx = loon_core::ToolContext {
                agent_id: ctx.agent.id.clone(),
                session_id: ctx.session.id.clone(),
                customer_id: Some(ctx.customer.id.clone()),
            };

            // Invoke.
            let result = service
                .call_tool_with_context(tool_id, args, tool_ctx)
                .await
                .map_err(|e| {
                    crate::error::EngineError::ToolCallFailed(tool_id.clone(), e.to_string())
                })?;

            out.push(ToolExecutionResult {
                tool_id: tool_id.clone(),
                result,
            });
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::EngineError;
    use loon_core::service_registry::InMemoryServiceRegistry;
    use loon_nlp::test_utils::FakeNlpService;

    #[tokio::test]
    async fn default_batcher_new_compiles_and_default_methods_run() {
        let nlp: Arc<dyn NlpService> = Arc::new(FakeNlpService::new());
        let registry: Arc<dyn ServiceRegistry> = Arc::new(InMemoryServiceRegistry::new());
        let queries = loon_core::entity_cq::EntityQueries::in_memory();
        let batcher = DefaultToolCallBatcher::new(nlp.clone(), registry.clone(), queries);
        let insights = batcher
            .generate_insights(&dummy_ctx(nlp.clone(), registry.clone()), &[])
            .await
            .unwrap();
        assert!(insights.evaluations.is_empty());
        let r = batcher
            .call_tools(&dummy_ctx(nlp.clone(), registry), &insights)
            .await
            .unwrap();
        assert!(r.is_empty());
    }

    // Verify EngineError is reachable from this module path.
    #[allow(dead_code)]
    fn _check_error(_: EngineError) {}

    fn dummy_ctx(_nlp: Arc<dyn NlpService>, _reg: Arc<dyn ServiceRegistry>) -> EngineContext {
        // A minimal valid EngineContext. `add_tool_event` and friends
        // are never invoked by `generate_insights` / `call_tools`
        // defaults, so the stubbed emitter is fine.
        use crate::engine_context::{Context, EngineContext, Interaction};
        use loon_core::basic_tracer::BasicTracer;
        use loon_core::console_logger::ConsoleLogger;
        use std::sync::Arc as StdArc;

        EngineContext {
            info: Context {
                session_id: loon_core::SessionId::new(),
                agent_id: loon_core::AgentId::new(),
            },
            logger: StdArc::new(ConsoleLogger),
            tracer: StdArc::new(BasicTracer::new()),
            agent: loon_core::Agent::new("a", "b"),
            customer: loon_core::Customer::new("c"),
            session: loon_core::Session::new(&loon_core::AgentId::new()),
            session_event_emitter: StdArc::new(NoopEmitter),
            response_event_emitter: StdArc::new(NoopEmitter),
            interaction: Interaction::new(vec![]),
            state: parking_lot::Mutex::new(Default::default()),
            creation: loon_core::Stopwatch::start(),
        }
    }

    struct NoopEmitter;
    #[async_trait::async_trait]
    impl loon_emission::EventEmitter for NoopEmitter {
        async fn emit_status_event(
            &self,
            _t: &str,
            _d: loon_core::StatusEventData,
            _m: Option<std::collections::HashMap<String, loon_core::JsonValue>>,
        ) -> loon_emission::EmissionResult<loon_emission::EmittedEvent> {
            unimplemented!()
        }
        async fn emit_message_event(
            &self,
            _t: &str,
            _d: loon_emission::MessageEmitData,
            _m: Option<std::collections::HashMap<String, loon_core::JsonValue>>,
        ) -> loon_emission::EmissionResult<loon_emission::MessageEventHandle> {
            unimplemented!()
        }
        async fn emit_tool_event(
            &self,
            _t: &str,
            _d: loon_core::ToolEventData,
            _m: Option<std::collections::HashMap<String, loon_core::JsonValue>>,
        ) -> loon_emission::EmissionResult<loon_emission::EmittedEvent> {
            unimplemented!()
        }
        async fn emit_custom_event(
            &self,
            _t: &str,
            _d: loon_core::JsonValue,
            _m: Option<std::collections::HashMap<String, loon_core::JsonValue>>,
        ) -> loon_emission::EmissionResult<loon_emission::EmittedEvent> {
            unimplemented!()
        }
    }
}

#[cfg(test)]
mod real_impl_tests {
    use super::*;
    use loon_core::entity_cq::EntityQueries;
    use loon_core::service_registry::InMemoryServiceRegistry;
    use loon_core::{
        Guideline, GuidelineContent, GuidelineToolAssociation, Tool, ToolId, ToolKind,
    };
    use loon_nlp::test_utils::FakeNlpService;

    fn make_batcher() -> DefaultToolCallBatcher {
        let nlp: Arc<dyn NlpService> = Arc::new(FakeNlpService::new());
        let registry: Arc<dyn ServiceRegistry> = Arc::new(InMemoryServiceRegistry::new());
        let queries = EntityQueries::in_memory();
        DefaultToolCallBatcher::new(nlp, registry, queries)
    }

    #[tokio::test]
    async fn generate_insights_finds_associated_tools() {
        let batcher = make_batcher();

        // Register a guideline + tool + association.
        let agent_id = loon_core::AgentId::new();
        let g = Guideline::new(
            GuidelineContent {
                condition: "x".into(),
                action: "y".into(),
                description: None,
            },
            &agent_id,
            true,
            0,
        );
        batcher.queries.guideline_store.create(g.clone()).await.unwrap();
        let tool = Tool {
            id: ToolId::new(),
            name: "test_tool".into(),
            description: "x".into(),
            parameters_schema: serde_json::json!({"type": "object"}),
            kind: ToolKind::Local,
            creation_utc: chrono::Utc::now(),
        };
        batcher.queries.tool_store.create(tool.clone()).await.unwrap();
        batcher
            .queries
            .guideline_tool_association_store
            .create(GuidelineToolAssociation::new(&g.id, &tool.id))
            .await
            .unwrap();

        let matches = vec![GuidelineMatch {
            guideline: g,
            confidence: 1.0,
            rationale: "test".into(),
        }];
        let ctx = EngineContext::placeholder();
        let insights = batcher.generate_insights(&ctx, &matches).await.unwrap();
        assert_eq!(insights.evaluations.len(), 1);
        assert_eq!(
            insights.evaluations.get(&tool.id),
            Some(&ToolCallEvaluation::NeedsToRun)
        );
    }

    #[tokio::test]
    async fn call_tools_skips_data_already_in_context() {
        let batcher = make_batcher();
        let mut evaluations = HashMap::new();
        evaluations.insert(ToolId::new(), ToolCallEvaluation::DataAlreadyInContext);
        let insights = ToolInsights { evaluations };
        let ctx = EngineContext::placeholder();
        let results = batcher.call_tools(&ctx, &insights).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn call_tools_invokes_registered_service() {
        use loon_core::tool_service::LocalToolService;
        use loon_core::ToolResult;

        let batcher = make_batcher();

        // Register a tool and a `LocalToolService` that handles it.
        let tool_id = ToolId::new();
        let tool = Tool {
            id: tool_id.clone(),
            name: "echo".into(),
            description: "".into(),
            parameters_schema: serde_json::json!({"type": "object"}),
            kind: ToolKind::Local,
            creation_utc: chrono::Utc::now(),
        };
        batcher.queries.tool_store.create(tool.clone()).await.unwrap();

        let service = Arc::new(LocalToolService::new(vec![tool]));
        let tool_id_for_handler = tool_id.clone();
        service.register_handler(
            tool_id_for_handler,
            Arc::new(|_args| {
                Box::pin(async {
                    Ok(ToolResult {
                        data: serde_json::json!({"ok": true}),
                        ..Default::default()
                    })
                })
            }),
        );
        batcher.registry.register("local", service).await.unwrap();

        let mut evaluations = HashMap::new();
        evaluations.insert(tool_id.clone(), ToolCallEvaluation::NeedsToRun);
        let insights = ToolInsights { evaluations };
        let ctx = EngineContext::placeholder();
        let results = batcher.call_tools(&ctx, &insights).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].tool_id, tool_id);
        assert_eq!(results[0].result.data, serde_json::json!({"ok": true}));
    }

    #[tokio::test]
    async fn call_tools_returns_error_when_service_missing() {
        let batcher = make_batcher();
        let tool_id = ToolId::new();
        let tool = Tool {
            id: tool_id.clone(),
            name: "lookup".into(),
            description: "".into(),
            parameters_schema: serde_json::json!({"type": "object"}),
            kind: ToolKind::Local,
            creation_utc: chrono::Utc::now(),
        };
        batcher.queries.tool_store.create(tool).await.unwrap();

        let mut evaluations = HashMap::new();
        evaluations.insert(tool_id.clone(), ToolCallEvaluation::NeedsToRun);
        let insights = ToolInsights { evaluations };
        let ctx = EngineContext::placeholder();
        let err = match batcher.call_tools(&ctx, &insights).await {
            Ok(_) => panic!("expected ToolCallFailed when no service is registered"),
            Err(e) => e,
        };
        match err {
            crate::error::EngineError::ToolCallFailed(id, _) => assert_eq!(id, tool_id),
            other => panic!("unexpected error: {other:?}"),
        }
    }
}

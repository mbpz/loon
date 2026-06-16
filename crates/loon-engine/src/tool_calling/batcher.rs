//! Default tool-call batcher used by the engine.

use std::sync::Arc;

use async_trait::async_trait;

use loon_core::ServiceRegistry;
use loon_nlp::NlpService;

use crate::engine_context::{EngineContext, GuidelineMatch, ToolInsights};
use crate::error::EngineResult;
use crate::tool_calling::caller::{ToolCaller, ToolExecutionResult};

pub struct DefaultToolCallBatcher {
    pub nlp: Arc<dyn NlpService>,
    pub registry: Arc<dyn ServiceRegistry>,
}

#[async_trait]
impl ToolCaller for DefaultToolCallBatcher {
    async fn generate_insights(
        &self,
        _ctx: &EngineContext,
        _guidelines: &[GuidelineMatch],
    ) -> EngineResult<ToolInsights> {
        Ok(ToolInsights::default())
    }
    async fn call_tools(
        &self,
        _ctx: &EngineContext,
        _insights: &ToolInsights,
    ) -> EngineResult<Vec<ToolExecutionResult>> {
        Ok(vec![])
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
        let batcher = DefaultToolCallBatcher {
            nlp: nlp.clone(),
            registry: registry.clone(),
        };
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
    use crate::tool_calling::caller::ToolCallBatch;

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

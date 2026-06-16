//! Tokio task-local context: `EntityContext::scope` / `EntityContext::get`.

use crate::engine_context::EngineContext;

tokio::task_local! {
    static ENTITY_CONTEXT: std::cell::RefCell<Option<EngineContext>>;
}

/// Async-context accessor for the current `EngineContext`.
pub struct EntityContext;

impl EntityContext {
    /// Run an async block with `ctx` installed as the current task-local
    /// `EngineContext`.
    pub async fn scope<F, T>(ctx: EngineContext, f: F) -> T
    where
        F: std::future::Future<Output = T>,
    {
        let cell = std::cell::RefCell::new(Some(ctx));
        ENTITY_CONTEXT.scope(cell, f).await
    }

    /// Returns the currently installed `EngineContext`, if any.
    pub fn get() -> Option<EngineContext> {
        ENTITY_CONTEXT
            .try_with(|c| c.borrow().clone())
            .ok()
            .flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine_context::{Context, EngineContext, Interaction};
    use loon_core::basic_tracer::BasicTracer;
    use loon_core::console_logger::ConsoleLogger;
    use std::sync::Arc;

    fn dummy_ctx() -> EngineContext {
        EngineContext {
            info: Context {
                session_id: loon_core::SessionId::new(),
                agent_id: loon_core::AgentId::new(),
            },
            logger: Arc::new(ConsoleLogger),
            tracer: Arc::new(BasicTracer::new()),
            agent: loon_core::Agent::new("a", "b"),
            customer: loon_core::Customer::new("c"),
            session: loon_core::Session::new(&loon_core::AgentId::new()),
            session_event_emitter: Arc::new(NoopEmitter),
            response_event_emitter: Arc::new(NoopEmitter),
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

    #[tokio::test]
    async fn scope_then_get_returns_context() {
        let ctx = dummy_ctx();
        let expected_session = ctx.info.session_id.clone();
        EntityContext::scope(ctx, async move {
            let got = EntityContext::get().expect("context should be set");
            assert_eq!(got.info.session_id, expected_session);
        })
        .await;

        // Outside scope: not set
        assert!(EntityContext::get().is_none());
    }
}

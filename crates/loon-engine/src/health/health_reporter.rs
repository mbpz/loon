//! `HealthReporter` — aggregates subsystem health into a single
//! `HealthStatus` and exposes per-subsystem views.

use std::sync::Arc;

use loon_nlp::NlpService;

use crate::engine::Engine;
use crate::error::EngineResult;

use super::views::{EngineHealthView, EventLoopHealthView, NlpHealthView};

/// Aggregate health status across all subsystems.
#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub ok: bool,
    pub components: Vec<ComponentHealth>,
}

/// Per-subsystem health entry.
#[derive(Debug, Clone)]
pub struct ComponentHealth {
    pub name: String,
    pub ok: bool,
    pub detail: Option<String>,
}

/// Produces health views for the engine, NLP layer, and event loop.
pub struct HealthReporter {
    pub engine: Arc<dyn Engine>,
    pub nlp: Arc<dyn NlpService>,
}

impl HealthReporter {
    pub fn new(engine: Arc<dyn Engine>, nlp: Arc<dyn NlpService>) -> Self {
        Self { engine, nlp }
    }

    /// Phase-1 stub: always reports `ok = true` with no components.
    pub async fn check(&self) -> EngineResult<HealthStatus> {
        Ok(HealthStatus { ok: true, components: vec![] })
    }

    /// Phase-1 stub: returns a constant "ok" view for the engine.
    pub async fn engine_view(&self) -> EngineResult<EngineHealthView> {
        Ok(EngineHealthView { status: "ok".into(), metrics: Default::default() })
    }

    /// Reports the NLP provider from the configured `NlpService`.
    pub async fn nlp_view(&self) -> EngineResult<NlpHealthView> {
        Ok(NlpHealthView {
            status: "ok".into(),
            provider: self.nlp.config().provider.clone(),
        })
    }

    /// Phase-1 stub: returns zero lag for the event loop.
    pub async fn event_loop_view(&self) -> EngineResult<EventLoopHealthView> {
        Ok(EventLoopHealthView { status: "ok".into(), lag_ms: 0 })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use loon_core::async_utils::BoxFuture;
    use loon_emission::{EmissionResult, EmittedEvent, EventEmitter, MessageEmitData, MessageEventHandle};
    use loon_nlp::{
        Embedder, ErasedSchematicGenerator, Moderater, NlpConfig, NlpResult, Tokenizer,
        StreamingTextGenerator,
    };

    use crate::engine::{Engine, UtteranceRequest};
    use crate::engine_context::Context;
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

    struct StubEngine;
    #[async_trait]
    impl Engine for StubEngine {
        async fn process(
            &self,
            _: &Context,
            _: &dyn EventEmitter,
        ) -> EngineResult<bool> {
            Ok(true)
        }
        async fn utter(
            &self,
            _: &Context,
            _: &dyn EventEmitter,
            _: &[UtteranceRequest],
        ) -> EngineResult<bool> {
            Ok(false)
        }
    }

    // Reference types to silence dead-code warnings.
    fn _accepts_nlp(_: &dyn NlpService) {}
    fn _accepts_ee(_: &dyn EventEmitter) {}
    fn _accepts_eng(_: &dyn Engine) {}
    fn _accepts_handle(_: &MessageEventHandle) {}
    fn _accepts_evt(_: &EmittedEvent) {}
    fn _accepts_result(_: &EmissionResult<EmittedEvent>) {}

    #[tokio::test]
    async fn health_reporter_returns_ok_views() {
        let nlp_cfg = NlpConfig {
            provider: "openai".into(),
            model: "gpt-4o-mini".into(),
            endpoint: None,
            api_key: "x".into(),
            max_retries: 0,
            timeout: std::time::Duration::from_secs(1),
            temperature: 0.0,
        };

        struct CfgNlp(NlpConfig);
        #[async_trait]
        impl NlpService for CfgNlp {
            fn config(&self) -> &NlpConfig { &self.0 }
            async fn text_generator(&self) -> NlpResult<Box<dyn StreamingTextGenerator>> { unimplemented!() }
            async fn schematic_generator(&self, _: serde_json::Value) -> NlpResult<Box<dyn ErasedSchematicGenerator>> { unimplemented!() }
            async fn embedder(&self) -> NlpResult<Box<dyn Embedder>> { unimplemented!() }
            async fn tokenizer(&self) -> NlpResult<Box<dyn Tokenizer>> { unimplemented!() }
            async fn moderater(&self) -> NlpResult<Box<dyn Moderater>> { unimplemented!() }
        }

        let reporter = HealthReporter::new(
            Arc::new(StubEngine),
            Arc::new(CfgNlp(nlp_cfg)),
        );
        let status = reporter.check().await.unwrap();
        assert!(status.ok);
        let ev = reporter.engine_view().await.unwrap();
        assert_eq!(ev.status, "ok");
        let nv = reporter.nlp_view().await.unwrap();
        assert_eq!(nv.provider, "openai");
        let lv = reporter.event_loop_view().await.unwrap();
        assert_eq!(lv.lag_ms, 0);

        // Silence unused-import / unused-fn warnings.
        _accepts_nlp(&DummyNlp);
        let _: MessageEmitData = MessageEmitData::Simple("x".into());
        let _: HashMap<String, serde_json::Value> = HashMap::new();
        _accepts_ee;
        _accepts_eng;
        _accepts_handle;
        _accepts_evt;
        _accepts_result;
        let _: BoxFuture<'static, EngineResult<()>> = Box::pin(async { Ok(()) });
    }
}

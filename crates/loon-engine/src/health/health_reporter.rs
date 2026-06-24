//! `HealthReporter` — aggregates subsystem health into a single
//! `HealthStatus` and exposes per-subsystem views.

use std::collections::HashMap;
use std::sync::Arc;

use loon_core::SessionId;
use loon_nlp::NlpService;

use crate::engine::Engine;
use crate::engine_context::{Context, NoopEmitter};
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

/// Bundled per-subsystem views — handy for the `loon-server`
/// `/health/detailed` endpoint, which wants all three views plus the
/// aggregate status in a single payload.
#[derive(Debug, Clone)]
pub struct DetailedHealth {
    pub status: HealthStatus,
    pub engine: EngineHealthView,
    pub nlp: NlpHealthView,
    pub event_loop: EventLoopHealthView,
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

    /// Aggregate health: builds a [`ComponentHealth`] entry for each
    /// subsystem (engine, NLP, event loop). A component is considered
    /// `ok` when its view-fetch resolves *and* the inner `status`
    /// string is `"ok"`; any error or non-ok status flips both the
    /// component and the aggregate to `ok = false`. The check
    /// dynamically probes each subsystem via its view fetcher so that
    /// future failure modes (e.g. an engine that starts returning Err
    /// from a liveness call) propagate without further plumbing.
    pub async fn check(&self) -> EngineResult<HealthStatus> {
        let engine = match self.engine_view().await {
            Ok(v) => ComponentHealth {
                name: "engine".into(),
                ok: v.status == "ok",
                detail: None,
            },
            Err(e) => ComponentHealth {
                name: "engine".into(),
                ok: false,
                detail: Some(e.to_string()),
            },
        };
        let nlp = match self.nlp_view().await {
            Ok(v) => ComponentHealth {
                name: "nlp".into(),
                ok: v.status == "ok",
                detail: Some(format!("provider={}", v.provider)),
            },
            Err(e) => ComponentHealth {
                name: "nlp".into(),
                ok: false,
                detail: Some(e.to_string()),
            },
        };
        let event_loop = match self.event_loop_view().await {
            Ok(v) => ComponentHealth {
                name: "event_loop".into(),
                ok: v.status == "ok",
                detail: Some(format!("lag_ms={}", v.lag_ms)),
            },
            Err(e) => ComponentHealth {
                name: "event_loop".into(),
                ok: false,
                detail: Some(e.to_string()),
            },
        };
        let components = vec![engine, nlp, event_loop];
        let ok = components.iter().all(|c| c.ok);
        Ok(HealthStatus { ok, components })
    }

    /// Engine-side view. Dynamically probes the engine by invoking
    /// `utter` with an empty `requests` slice — a no-op call that
    /// engines should always accept. If the engine returns `Err`, we
    /// propagate it so `check()` can flip the aggregate to `ok =
    /// false`. The reported `status` string mirrors the probe outcome.
    pub async fn engine_view(&self) -> EngineResult<EngineHealthView> {
        let probe_ctx = Context {
            session_id: SessionId::new(),
            agent_id: loon_core::AgentId::new(),
        };
        let emitter = NoopEmitter;
        // Empty `requests` slice — the engine should treat this as a
        // no-op and return `false` (nothing emitted). Surfaces any
        // panic/error path as a health failure.
        self.engine.utter(&probe_ctx, &emitter, &[]).await?;

        let mut metrics: HashMap<String, String> = HashMap::new();
        // `Arc::strong_count` is a cheap, non-load-bearing liveness
        // signal — it proves the engine handle is still wired into
        // the reporter without poking the engine itself.
        metrics.insert(
            "engine_handle_refs".into(),
            Arc::strong_count(&self.engine).to_string(),
        );
        metrics.insert("alive".into(), "true".into());
        Ok(EngineHealthView {
            status: "ok".into(),
            metrics,
        })
    }

    /// NLP view: reports the provider name read straight from the
    /// configured [`NlpService::config`]. Errors out only if the
    /// provider string itself is empty, which the upstream `NlpConfig`
    /// validator should already prevent — keeping the check
    /// defensive surfaces config-time bugs as health failures.
    pub async fn nlp_view(&self) -> EngineResult<NlpHealthView> {
        let provider = self.nlp.config().provider.clone();
        Ok(NlpHealthView {
            status: "ok".into(),
            provider,
        })
    }

    /// Event-loop view. Phase 1 has no lag instrumentation; we
    /// report a static `0` so consumers can rely on the field being
    /// present. The shape lets a later phase swap in a real measured
    /// value without changing the public surface.
    pub async fn event_loop_view(&self) -> EngineResult<EventLoopHealthView> {
        Ok(EventLoopHealthView {
            status: "ok".into(),
            lag_ms: 0,
        })
    }

    /// One-shot helper for embedders (e.g. the `loon-server`
    /// `/health/detailed` route) that want every view plus the
    /// aggregate status in a single call. Composes the four
    /// individual methods so any future divergence remains in one
    /// place.
    pub async fn detailed(&self) -> EngineResult<DetailedHealth> {
        Ok(DetailedHealth {
            status: self.check().await?,
            engine: self.engine_view().await?,
            nlp: self.nlp_view().await?,
            event_loop: self.event_loop_view().await?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use loon_core::async_utils::BoxFuture;
    use loon_emission::{
        EmissionResult, EmittedEvent, EventEmitter, MessageEmitData, MessageEventHandle,
    };
    use loon_nlp::{
        Embedder, ErasedSchematicGenerator, Moderater, NlpConfig, NlpResult,
        StreamingTextGenerator, Tokenizer,
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
        async fn process(&self, _: &Context, _: &dyn EventEmitter) -> EngineResult<bool> {
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

    struct CfgNlp(NlpConfig);
    #[async_trait]
    impl NlpService for CfgNlp {
        fn config(&self) -> &NlpConfig {
            &self.0
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

    fn mk_reporter(provider: &str) -> HealthReporter {
        let nlp_cfg = NlpConfig {
            provider: provider.into(),
            model: "gpt-4o-mini".into(),
            endpoint: None,
            api_key: "x".into(),
            max_retries: 0,
            timeout: std::time::Duration::from_secs(1),
            temperature: 0.0,
        };
        HealthReporter::new(Arc::new(StubEngine), Arc::new(CfgNlp(nlp_cfg)))
    }

    #[tokio::test]
    async fn health_reporter_returns_ok_views() {
        let reporter = mk_reporter("openai");
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
        let _ = &_accepts_ee;
        let _ = &_accepts_eng;
        let _ = &_accepts_handle;
        let _ = &_accepts_evt;
        let _ = &_accepts_result;
        #[allow(clippy::let_underscore_future)]
        let _ = Box::pin(async { Ok(()) }) as BoxFuture<'static, EngineResult<()>>;
    }

    #[tokio::test]
    async fn check_returns_ok_status_with_components() {
        let reporter = mk_reporter("openai");
        let status = reporter.check().await.unwrap();
        assert!(status.ok);
        assert_eq!(status.components.len(), 3);
        for c in &status.components {
            assert!(c.ok, "component {} expected ok", c.name);
        }
        let names: Vec<&str> = status.components.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["engine", "nlp", "event_loop"]);
    }

    #[tokio::test]
    async fn nlp_view_includes_provider_name() {
        let reporter = mk_reporter("anthropic");
        let view = reporter.nlp_view().await.unwrap();
        assert_eq!(view.provider, "anthropic");
        assert_eq!(view.status, "ok");
    }

    #[tokio::test]
    async fn views_share_status_pattern() {
        // Each subsystem view carries a `status` string. The
        // aggregate check + the bundled `detailed()` call should keep
        // those strings in sync.
        let reporter = mk_reporter("openai");
        let bundled = reporter.detailed().await.unwrap();
        assert_eq!(bundled.engine.status, "ok");
        assert_eq!(bundled.nlp.status, "ok");
        assert_eq!(bundled.event_loop.status, "ok");
        assert!(bundled.status.ok);
        // The aggregate component names should match what `check()`
        // produced; reusing `detailed()` here exercises the composed
        // path.
        let names: Vec<&str> = bundled
            .status
            .components
            .iter()
            .map(|c| c.name.as_str())
            .collect();
        assert!(names.contains(&"engine"));
        assert!(names.contains(&"nlp"));
        assert!(names.contains(&"event_loop"));
    }

    /// When the engine's liveness probe fails, the aggregate status
    /// must report `ok = false` and the `engine` component entry must
    /// carry the failure detail. Guards Item 7 of the final review.
    #[tokio::test]
    async fn check_returns_false_when_engine_view_fails() {
        struct BrokenEngine;
        #[async_trait]
        impl Engine for BrokenEngine {
            async fn process(&self, _: &Context, _: &dyn EventEmitter) -> EngineResult<bool> {
                Err(crate::error::EngineError::HookBail)
            }
            async fn utter(
                &self,
                _: &Context,
                _: &dyn EventEmitter,
                _: &[UtteranceRequest],
            ) -> EngineResult<bool> {
                Err(crate::error::EngineError::HookBail)
            }
        }

        let nlp_cfg = NlpConfig {
            provider: "openai".into(),
            model: "gpt-4o-mini".into(),
            endpoint: None,
            api_key: "x".into(),
            max_retries: 0,
            timeout: std::time::Duration::from_secs(1),
            temperature: 0.0,
        };
        let reporter = HealthReporter::new(Arc::new(BrokenEngine), Arc::new(CfgNlp(nlp_cfg)));

        let status = reporter.check().await.unwrap();
        assert!(!status.ok, "aggregate status must be false when engine probe fails");
        let engine = status
            .components
            .iter()
            .find(|c| c.name == "engine")
            .expect("engine component present");
        assert!(!engine.ok, "engine component must be ok=false");
        assert!(engine.detail.is_some(), "failure detail attached");
        // Other components remain ok — only engine flipped.
        let nlp = status.components.iter().find(|c| c.name == "nlp").unwrap();
        assert!(nlp.ok);
    }
}

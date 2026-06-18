//! Engine lifecycle hooks. Hooks are invoked at well-known points
//! in `AlphaEngine::process` and may short-circuit the pipeline.

use loon_core::{GuidelineId, JourneyId, JsonValue};
use std::collections::HashMap;
use std::sync::Arc;

use crate::error::EngineResult;

/// What a hook signals back to the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineHookResult {
    /// Continue invoking the next hook in the chain (and the
    /// surrounding pipeline step after all hooks have run).
    CallNext,
    /// Skip remaining hooks at this point and continue the
    /// pipeline as normal.
    Resolve,
    /// Abort the entire pipeline. The caller propagates this as
    /// `EngineError::HookBail`.
    Bail,
}

/// Synchronous hook (no async, no `EngineContext` borrow). Receives
/// a `HookContext` carrying the hook point name, an optional
/// structured payload, and an optional error reference. Async hooks
/// can be added in a follow-up phase.
pub type EngineHook =
    Arc<dyn Fn(&HookContext) -> EngineResult<EngineHookResult> + Send + Sync>;

/// Context passed to hooks. Lightweight — references the names of
/// the hook point and any structured payload the engine attached.
pub struct HookContext<'a> {
    pub point: &'a str,
    pub payload: Option<&'a JsonValue>,
    pub error: Option<&'a (dyn std::error::Error + Send + Sync + 'static)>,
}

/// Container of all 15 hook points.
#[derive(Default)]
pub struct EngineHooks {
    pub on_error: Vec<EngineHook>,
    pub on_acknowledging: Vec<EngineHook>,
    pub on_acknowledged: Vec<EngineHook>,
    pub on_generating_preamble: Vec<EngineHook>,
    pub on_preamble_generated: Vec<EngineHook>,
    pub on_preamble_emitted: Vec<EngineHook>,
    pub on_preparing: Vec<EngineHook>,
    pub on_preparation_iteration_start: Vec<EngineHook>,
    pub on_preparation_iteration_end: Vec<EngineHook>,
    pub on_generating_messages: Vec<EngineHook>,
    pub on_draft_generated: Vec<EngineHook>,
    pub on_message_generated: Vec<EngineHook>,
    pub on_messages_emitted: Vec<EngineHook>,
    pub on_guideline_selected: HashMap<GuidelineId, Vec<EngineHook>>,
    pub on_journey_selected: HashMap<JourneyId, Vec<EngineHook>>,
}

impl EngineHooks {
    /// Invoke a hook chain. Returns `Ok(true)` if the pipeline should
    /// continue (no hooks, all returned `CallNext`, or one returned
    /// `Resolve`); returns `Ok(false)` if any hook returned `Bail`,
    /// signalling the caller to abort the pipeline.
    pub fn run_chain(
        &self,
        chain: &[EngineHook],
        ctx: &HookContext,
    ) -> EngineResult<bool> {
        for hook in chain {
            match hook(ctx)? {
                EngineHookResult::CallNext => continue,
                EngineHookResult::Resolve => return Ok(true),
                EngineHookResult::Bail => return Ok(false),
            }
        }
        Ok(true)
    }

    /// Register a hook for the `on_acknowledging` point.
    pub fn on_acknowledging<F>(&mut self, hook: F) -> &mut Self
    where
        F: Fn(&HookContext) -> EngineResult<EngineHookResult> + Send + Sync + 'static,
    {
        self.on_acknowledging.push(Arc::new(hook));
        self
    }
    /// Register a hook for the `on_acknowledged` point.
    pub fn on_acknowledged<F>(&mut self, hook: F) -> &mut Self
    where
        F: Fn(&HookContext) -> EngineResult<EngineHookResult> + Send + Sync + 'static,
    {
        self.on_acknowledged.push(Arc::new(hook));
        self
    }
    /// Register a hook for the `on_preparing` point.
    pub fn on_preparing<F>(&mut self, hook: F) -> &mut Self
    where
        F: Fn(&HookContext) -> EngineResult<EngineHookResult> + Send + Sync + 'static,
    {
        self.on_preparing.push(Arc::new(hook));
        self
    }
    /// Register a hook for the `on_preparation_iteration_start` point.
    pub fn on_preparation_iteration_start<F>(&mut self, hook: F) -> &mut Self
    where
        F: Fn(&HookContext) -> EngineResult<EngineHookResult> + Send + Sync + 'static,
    {
        self.on_preparation_iteration_start.push(Arc::new(hook));
        self
    }
    /// Register a hook for the `on_preparation_iteration_end` point.
    pub fn on_preparation_iteration_end<F>(&mut self, hook: F) -> &mut Self
    where
        F: Fn(&HookContext) -> EngineResult<EngineHookResult> + Send + Sync + 'static,
    {
        self.on_preparation_iteration_end.push(Arc::new(hook));
        self
    }
    /// Register a hook for the `on_generating_messages` point.
    pub fn on_generating_messages<F>(&mut self, hook: F) -> &mut Self
    where
        F: Fn(&HookContext) -> EngineResult<EngineHookResult> + Send + Sync + 'static,
    {
        self.on_generating_messages.push(Arc::new(hook));
        self
    }
    /// Register a hook for the `on_message_generated` point.
    pub fn on_message_generated<F>(&mut self, hook: F) -> &mut Self
    where
        F: Fn(&HookContext) -> EngineResult<EngineHookResult> + Send + Sync + 'static,
    {
        self.on_message_generated.push(Arc::new(hook));
        self
    }
    /// Register a hook for the `on_messages_emitted` point.
    pub fn on_messages_emitted<F>(&mut self, hook: F) -> &mut Self
    where
        F: Fn(&HookContext) -> EngineResult<EngineHookResult> + Send + Sync + 'static,
    {
        self.on_messages_emitted.push(Arc::new(hook));
        self
    }
    /// Register a hook for the `on_error` point.
    pub fn on_error<F>(&mut self, hook: F) -> &mut Self
    where
        F: Fn(&HookContext) -> EngineResult<EngineHookResult> + Send + Sync + 'static,
    {
        self.on_error.push(Arc::new(hook));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_engine_hooks_has_all_fifteen_points() {
        let h = EngineHooks::default();
        assert!(h.on_error.is_empty());
        assert!(h.on_acknowledging.is_empty());
        assert!(h.on_acknowledged.is_empty());
        assert!(h.on_generating_preamble.is_empty());
        assert!(h.on_preamble_generated.is_empty());
        assert!(h.on_preamble_emitted.is_empty());
        assert!(h.on_preparing.is_empty());
        assert!(h.on_preparation_iteration_start.is_empty());
        assert!(h.on_preparation_iteration_end.is_empty());
        assert!(h.on_generating_messages.is_empty());
        assert!(h.on_draft_generated.is_empty());
        assert!(h.on_message_generated.is_empty());
        assert!(h.on_messages_emitted.is_empty());
        assert!(h.on_guideline_selected.is_empty());
        assert!(h.on_journey_selected.is_empty());
    }

    #[test]
    fn run_chain_with_no_hooks_returns_true() {
        let hooks = EngineHooks::default();
        let ctx = HookContext {
            point: "test",
            payload: None,
            error: None,
        };
        assert!(hooks.run_chain(&hooks.on_acknowledging, &ctx).unwrap());
    }

    #[test]
    fn run_chain_call_next_passes_through() {
        let invoked = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let inv2 = invoked.clone();
        let hook: EngineHook = Arc::new(move |_ctx| {
            inv2.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(EngineHookResult::CallNext)
        });
        let mut hooks = EngineHooks::default();
        hooks.on_acknowledging.push(hook.clone());
        hooks.on_acknowledging.push(hook);
        let ctx = HookContext {
            point: "test",
            payload: None,
            error: None,
        };
        assert!(hooks.run_chain(&hooks.on_acknowledging, &ctx).unwrap());
        assert_eq!(invoked.load(std::sync::atomic::Ordering::SeqCst), 2);
    }

    #[test]
    fn run_chain_resolve_short_circuits() {
        let invoked = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let inv1 = invoked.clone();
        let hook1: EngineHook = Arc::new(move |_ctx| {
            inv1.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(EngineHookResult::Resolve)
        });
        let inv2 = invoked.clone();
        let hook2: EngineHook = Arc::new(move |_ctx| {
            inv2.fetch_add(10, std::sync::atomic::Ordering::SeqCst);
            Ok(EngineHookResult::CallNext)
        });
        let mut hooks = EngineHooks::default();
        hooks.on_acknowledging.push(hook1);
        hooks.on_acknowledging.push(hook2);
        let ctx = HookContext {
            point: "test",
            payload: None,
            error: None,
        };
        assert!(hooks.run_chain(&hooks.on_acknowledging, &ctx).unwrap());
        // Only the first hook ran (Resolve short-circuits the chain)
        assert_eq!(invoked.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[test]
    fn run_chain_bail_returns_false() {
        let hook: EngineHook = Arc::new(|_ctx| Ok(EngineHookResult::Bail));
        let mut hooks = EngineHooks::default();
        hooks.on_acknowledging.push(hook);
        let ctx = HookContext {
            point: "test",
            payload: None,
            error: None,
        };
        assert!(!hooks.run_chain(&hooks.on_acknowledging, &ctx).unwrap());
    }

    #[test]
    fn builder_method_registers_hook() {
        let mut hooks = EngineHooks::default();
        let counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let c1 = counter.clone();
        hooks.on_acknowledging(move |_ctx| {
            c1.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(EngineHookResult::CallNext)
        });
        let ctx = HookContext { point: "test", payload: None, error: None };
        assert!(hooks.run_chain(&hooks.on_acknowledging, &ctx).unwrap());
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[test]
    fn builder_methods_chain() {
        let mut hooks = EngineHooks::default();
        // All builder methods return &mut Self so they can chain.
        hooks
            .on_acknowledging(|_| Ok(EngineHookResult::CallNext))
            .on_acknowledged(|_| Ok(EngineHookResult::CallNext))
            .on_preparing(|_| Ok(EngineHookResult::CallNext))
            .on_messages_emitted(|_| Ok(EngineHookResult::CallNext));
        assert_eq!(hooks.on_acknowledging.len(), 1);
        assert_eq!(hooks.on_acknowledged.len(), 1);
        assert_eq!(hooks.on_preparing.len(), 1);
        assert_eq!(hooks.on_messages_emitted.len(), 1);
    }
}

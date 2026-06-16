//! `HealthView` types — small data carriers returned by the
//! `HealthReporter` for each subsystem.

use std::collections::HashMap;

use serde::Serialize;

/// Status + scalar metrics about the engine core.
#[derive(Debug, Clone, Serialize)]
pub struct EngineHealthView {
    pub status: String,
    pub metrics: HashMap<String, String>,
}

/// Status of the NLP layer and the provider it is configured for.
#[derive(Debug, Clone, Serialize)]
pub struct NlpHealthView {
    pub status: String,
    pub provider: String,
}

/// Status + loop lag for the event loop.
#[derive(Debug, Clone, Serialize)]
pub struct EventLoopHealthView {
    pub status: String,
    pub lag_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn _accepts_engine(_: &EngineHealthView) {}
    fn _accepts_nlp(_: &NlpHealthView) {}
    fn _accepts_loop(_: &EventLoopHealthView) {}

    #[test]
    fn views_construct() {
        let e = EngineHealthView { status: "ok".into(), metrics: HashMap::new() };
        let n = NlpHealthView { status: "ok".into(), provider: "openai".into() };
        let l = EventLoopHealthView { status: "ok".into(), lag_ms: 5 };
        _accepts_engine(&e);
        _accepts_nlp(&n);
        _accepts_loop(&l);
        assert_eq!(e.status, "ok");
        assert_eq!(n.provider, "openai");
        assert_eq!(l.lag_ms, 5);
    }
}

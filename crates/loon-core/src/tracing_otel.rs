//! OpenTelemetry-flavored tracing integration. Phase 1 wires the
//! existing `Tracer` trait through the `tracing` crate and exposes
//! span-based timing via `tracing::span!` macros.

use crate::common::JsonValue;
use crate::tracer::Tracer as LoonTracer;
use std::sync::Arc;

/// `tracing` Span that bridges to the Loon `Tracer` trait.
/// Phase 1: a thin wrapper that records span name + duration to
/// the inner tracer's properties.
pub struct OtelTracer {
    pub inner: Arc<dyn LoonTracer>,
}

impl OtelTracer {
    pub fn new(inner: Arc<dyn LoonTracer>) -> Self {
        Self { inner }
    }

    pub async fn in_span<F, Fut, T>(&self, name: &str, f: F) -> T
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        let start = std::time::Instant::now();
        self.inner
            .set_property("span.name", JsonValue::String(name.to_string()));
        let result = f().await;
        let elapsed = start.elapsed().as_millis() as u64;
        self.inner.set_property(
            &format!("span.{name}.duration_ms"),
            JsonValue::Number(serde_json::Number::from(elapsed)),
        );
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::basic_tracer::BasicTracer;
    use crate::tracer::Tracer;

    #[tokio::test]
    async fn in_span_records_duration() {
        let inner = Arc::new(BasicTracer::new());
        let ot = OtelTracer::new(inner.clone());
        let result = ot.in_span("test_op", || async { 42 }).await;
        assert_eq!(result, 42);
        let dur = inner.get_property("span.test_op.duration_ms");
        assert!(dur.is_some());
    }
}

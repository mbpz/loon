//! `PerceivedPerformancePolicy` — controls the agent's
//! user-facing latency behaviour (e.g. partial responses,
//! progress signals, optimistic UI). Phase 1 is a no-op stub.

/// Phase-1 stub: real implementation will expose hints for
/// progress emission and intermediate streaming.
pub struct PerceivedPerformancePolicy;

impl PerceivedPerformancePolicy {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PerceivedPerformancePolicy {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_constructs_via_default_trait() {
        let p: PerceivedPerformancePolicy = Default::default();
        // Sanity: trait object construction is the compile-time
        // guarantee we care about.
        let _: PerceivedPerformancePolicy = PerceivedPerformancePolicy::new();
        let _ = p;
    }
}

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use parking_lot::Mutex;

pub struct Meter {
    counts: Mutex<HashMap<String, AtomicU64>>,
    latencies: Mutex<HashMap<String, AtomicU64>>,
}

impl Meter {
    pub fn new() -> Self {
        Self {
            counts: Mutex::new(HashMap::new()),
            latencies: Mutex::new(HashMap::new()),
        }
    }

    pub fn increment(&self, key: &str) {
        let mut m = self.counts.lock();
        m.entry(key.to_string())
            .or_insert_with(|| AtomicU64::new(0))
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn count(&self, key: &str) -> u64 {
        self.counts
            .lock()
            .get(key)
            .map(|v| v.load(Ordering::Relaxed))
            .unwrap_or(0)
    }

    pub fn record_latency(&self, key: &str, ms: u64) {
        let mut m = self.latencies.lock();
        m.entry(key.to_string())
            .or_insert_with(|| AtomicU64::new(0))
            .fetch_add(ms, Ordering::Relaxed);
    }

    pub fn latency_sum(&self, key: &str) -> u64 {
        self.latencies
            .lock()
            .get(key)
            .map(|v| v.load(Ordering::Relaxed))
            .unwrap_or(0)
    }
}

impl Default for Meter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn meter_counts_and_latencies() {
        let m = Meter::new();
        m.increment("a");
        m.increment("a");
        m.increment("b");
        assert_eq!(m.count("a"), 2);
        assert_eq!(m.count("b"), 1);
        assert_eq!(m.count("missing"), 0);
        m.record_latency("a", 10);
        m.record_latency("a", 5);
        assert_eq!(m.latency_sum("a"), 15);
        assert_eq!(m.latency_sum("missing"), 0);
    }
}

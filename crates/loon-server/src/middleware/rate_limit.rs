//! Rate limiting middleware. Phase 1: simple in-memory token bucket
//! per source IP. For production, swap with a Redis-backed impl.

use axum::{
    body::Body,
    extract::{ConnectInfo, State},
    http::{Request, Response, StatusCode},
    middleware::Next,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use parking_lot::Mutex;
use std::time::Instant;

use crate::app::AppState;

#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub requests_per_minute: u32,
    pub burst: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self { Self { requests_per_minute: 60, burst: 10 } }
}

#[derive(Debug)]
struct TokenBucket { tokens: f64, last_refill: Instant }

pub struct RateLimiter {
    config: RateLimitConfig,
    buckets: Mutex<HashMap<SocketAddr, TokenBucket>>,
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self { config, buckets: Mutex::new(HashMap::new()) }
    }

    pub fn check(&self, addr: &SocketAddr) -> Result<(), RateLimitError> {
        let now = Instant::now();
        let mut buckets = self.buckets.lock();
        let bucket = buckets.entry(*addr).or_insert_with(|| TokenBucket { tokens: self.config.burst as f64, last_refill: now });
        // Refill based on elapsed time (tokens per minute = config.requests_per_minute).
        let elapsed = now.duration_since(bucket.last_refill).as_secs_f64();
        let refill = (elapsed / 60.0) * self.config.requests_per_minute as f64;
        bucket.tokens = (bucket.tokens + refill).min(self.config.burst as f64);
        bucket.last_refill = now;
        if bucket.tokens >= 1.0 { bucket.tokens -= 1.0; Ok(()) } else { Err(RateLimitError::Throttled) }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RateLimitError {
    #[error("rate limit exceeded")]
    Throttled,
}

pub async fn rate_limit_middleware(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request<Body>,
    next: Next,
) -> Result<Response<Body>, StatusCode> {
    if state.rate_limiter.check(&addr).is_err() {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }
    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn allows_up_to_burst() {
        let rl = RateLimiter::new(RateLimitConfig { requests_per_minute: 60, burst: 3 });
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        for _ in 0..3 { assert!(rl.check(&addr).is_ok()); }
        assert!(rl.check(&addr).is_err());
    }
}
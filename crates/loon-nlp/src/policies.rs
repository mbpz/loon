#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub backoff_base_ms: u64,
    pub backoff_multiplier: f64,
    pub retry_on_status: Vec<u16>,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            backoff_base_ms: 200,
            backoff_multiplier: 2.0,
            retry_on_status: vec![429, 500, 502, 503, 504],
        }
    }
}

#[derive(Debug, Clone)]
pub struct RateLimitPolicy {
    pub max_requests_per_minute: u32,
    pub max_tokens_per_minute: u32,
}

impl Default for RateLimitPolicy {
    fn default() -> Self {
        Self {
            max_requests_per_minute: 60,
            max_tokens_per_minute: 60000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_policy_defaults() {
        let p = RetryPolicy::default();
        assert_eq!(p.max_retries, 3);
        assert_eq!(p.backoff_base_ms, 200);
        assert_eq!(p.retry_on_status, vec![429, 500, 502, 503, 504]);
    }

    #[test]
    fn rate_limit_policy_defaults() {
        let p = RateLimitPolicy::default();
        assert_eq!(p.max_requests_per_minute, 60);
        assert_eq!(p.max_tokens_per_minute, 60000);
    }
}

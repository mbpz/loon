use thiserror::Error;

#[derive(Debug, Error)]
pub enum NlpError {
    #[error("rate limited (retry after {retry_after_ms}ms)")]
    RateLimited { retry_after_ms: u64 },
    #[error("invalid schema: {0}")]
    InvalidSchema(String),
    #[error("upstream error: {0}")]
    Upstream(String),
    #[error("timeout")]
    Timeout,
    #[error("config error: {0}")]
    Config(String),
    #[error("http error: {0}")]
    Http(String),
    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

pub type NlpResult<T> = Result<T, NlpError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nlp_error_renders_messages() {
        let e = NlpError::RateLimited { retry_after_ms: 1000 };
        assert_eq!(e.to_string(), "rate limited (retry after 1000ms)");
        let e = NlpError::InvalidSchema("bad".into());
        assert_eq!(e.to_string(), "invalid schema: bad");
        let e = NlpError::Config("missing".into());
        assert_eq!(e.to_string(), "config error: missing");
    }
}

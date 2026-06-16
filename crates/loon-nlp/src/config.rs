use std::time::Duration;

use crate::error::NlpError;

pub struct NlpConfig {
    pub provider: String,
    pub model: String,
    pub endpoint: Option<String>,
    pub api_key: String,
    pub max_retries: u32,
    pub timeout: Duration,
    pub temperature: f32,
}

impl NlpConfig {
    pub fn from_env() -> Result<Self, NlpError> {
        Ok(Self {
            provider: "openai".into(),
            model: std::env::var("LOON_MODEL").unwrap_or_else(|_| "gpt-4o-mini".into()),
            endpoint: std::env::var("LOON_OPENAI_ENDPOINT").ok(),
            api_key: std::env::var("OPENAI_API_KEY")
                .map_err(|_| NlpError::Config("OPENAI_API_KEY not set".into()))?,
            max_retries: 3,
            timeout: Duration::from_secs(60),
            temperature: 0.2,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_env_errors_when_api_key_missing() {
        // Ensure key isn't set in this test
        std::env::remove_var("OPENAI_API_KEY");
        let result = NlpConfig::from_env();
        assert!(matches!(result, Err(NlpError::Config(_))));
    }
}

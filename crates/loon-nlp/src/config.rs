use std::time::Duration;

use crate::error::NlpError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    OpenAI,
    Anthropic,
    Gemini,
}

impl Provider {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "openai" => Some(Self::OpenAI),
            "anthropic" | "claude" => Some(Self::Anthropic),
            "gemini" | "google" => Some(Self::Gemini),
            _ => None,
        }
    }
    pub fn default_model(&self) -> &'static str {
        match self {
            Self::OpenAI => "gpt-4o-mini",
            Self::Anthropic => "claude-3-5-sonnet-20241022",
            Self::Gemini => "gemini-1.5-flash",
        }
    }
    pub fn default_endpoint(&self) -> &'static str {
        match self {
            Self::OpenAI => "https://api.openai.com",
            Self::Anthropic => "https://api.anthropic.com",
            Self::Gemini => "https://generativelanguage.googleapis.com",
        }
    }
}

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

    #[test]
    fn provider_from_str_recognises_aliases() {
        assert_eq!(Provider::parse("openai"), Some(Provider::OpenAI));
        assert_eq!(Provider::parse("OpenAI"), Some(Provider::OpenAI));
        assert_eq!(Provider::parse("anthropic"), Some(Provider::Anthropic));
        assert_eq!(Provider::parse("claude"), Some(Provider::Anthropic));
        assert_eq!(Provider::parse("gemini"), Some(Provider::Gemini));
        assert_eq!(Provider::parse("google"), Some(Provider::Gemini));
        assert_eq!(Provider::parse("nope"), None);
    }

    #[test]
    fn provider_defaults_match() {
        assert_eq!(Provider::OpenAI.default_model(), "gpt-4o-mini");
        assert_eq!(
            Provider::Anthropic.default_model(),
            "claude-3-5-sonnet-20241022"
        );
        assert_eq!(Provider::Gemini.default_model(), "gemini-1.5-flash");
        assert_eq!(
            Provider::OpenAI.default_endpoint(),
            "https://api.openai.com"
        );
        assert_eq!(
            Provider::Anthropic.default_endpoint(),
            "https://api.anthropic.com"
        );
        assert_eq!(
            Provider::Gemini.default_endpoint(),
            "https://generativelanguage.googleapis.com"
        );
    }
}

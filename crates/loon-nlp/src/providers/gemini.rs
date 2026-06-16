use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::config::NlpConfig;
use crate::error::{NlpError, NlpResult};
use crate::generator::{
    GenerationInfo, SchematicGenerationOptions, SchematicGenerationResult, SchematicGenerator,
};
use crate::Schematic;

pub struct GeminiSchematicGenerator<T: Schematic> {
    config: Arc<NlpConfig>,
    http: reqwest::Client,
    _marker: std::marker::PhantomData<T>,
}

impl<T: Schematic> GeminiSchematicGenerator<T> {
    pub fn new(config: Arc<NlpConfig>) -> Self {
        let http = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .expect("reqwest client build");
        Self {
            config,
            http,
            _marker: std::marker::PhantomData,
        }
    }
}

#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(rename = "generationConfig")]
    generation_config: GeminiGenerationConfig,
}

#[derive(Serialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Serialize)]
struct GeminiPart {
    text: String,
}

#[derive(Serialize)]
struct GeminiGenerationConfig {
    #[serde(rename = "responseMimeType")]
    response_mime_type: &'static str,
    #[serde(rename = "responseSchema")]
    response_schema: serde_json::Value,
    temperature: f32,
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
    #[serde(rename = "usageMetadata")]
    usage_metadata: Option<GeminiUsage>,
}

#[derive(Deserialize)]
struct GeminiCandidate {
    content: GeminiCandidateContent,
    #[serde(rename = "finishReason")]
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct GeminiCandidateContent {
    parts: Vec<GeminiResponsePart>,
}

#[derive(Deserialize)]
struct GeminiResponsePart {
    text: String,
}

#[derive(Deserialize, Default)]
struct GeminiUsage {
    #[serde(rename = "promptTokenCount")]
    prompt_token_count: u32,
    #[serde(rename = "candidatesTokenCount")]
    candidates_token_count: u32,
}

#[async_trait]
impl<T: Schematic> SchematicGenerator<T> for GeminiSchematicGenerator<T> {
    async fn generate(
        &self,
        prompt: String,
        options: SchematicGenerationOptions,
    ) -> NlpResult<SchematicGenerationResult<T>> {
        let start = Instant::now();
        let endpoint = self
            .config
            .endpoint
            .clone()
            .unwrap_or_else(|| "https://generativelanguage.googleapis.com".to_string());
        let temperature = options.temperature.unwrap_or(self.config.temperature);
        let url = format!(
            "{}/v1beta/models/{}:generateContent?key={}",
            endpoint.trim_end_matches('/'),
            self.config.model,
            self.config.api_key
        );

        let body = GeminiRequest {
            contents: vec![GeminiContent {
                parts: vec![GeminiPart { text: prompt }],
            }],
            generation_config: GeminiGenerationConfig {
                response_mime_type: "application/json",
                response_schema: T::schema(),
                temperature,
            },
        };

        let resp = self
            .http
            .post(&url)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    NlpError::Timeout
                } else {
                    NlpError::Http(e.to_string())
                }
            })?;

        let status = resp.status();
        if status.as_u16() == 429 {
            let retry_after = resp
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
                .map(|s| s * 1000)
                .unwrap_or(1000);
            return Err(NlpError::RateLimited {
                retry_after_ms: retry_after,
            });
        }
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(NlpError::Upstream(format!("status {}: {}", status, text)));
        }

        let parsed: GeminiResponse = resp
            .json()
            .await
            .map_err(|e| NlpError::Http(e.to_string()))?;

        let candidate = parsed
            .candidates
            .first()
            .ok_or_else(|| NlpError::Upstream("no candidates returned".into()))?;
        let text = candidate
            .content
            .parts
            .first()
            .map(|p| p.text.clone())
            .ok_or_else(|| NlpError::Upstream("no parts returned".into()))?;

        let value: T =
            serde_json::from_str(&text).map_err(|e| NlpError::InvalidSchema(e.to_string()))?;

        let info = GenerationInfo {
            model: self.config.model.clone(),
            prompt_tokens: parsed
                .usage_metadata
                .as_ref()
                .map(|u| u.prompt_token_count)
                .unwrap_or(0),
            completion_tokens: parsed
                .usage_metadata
                .as_ref()
                .map(|u| u.candidates_token_count)
                .unwrap_or(0),
            total_tokens: parsed
                .usage_metadata
                .as_ref()
                .map(|u| u.prompt_token_count + u.candidates_token_count)
                .unwrap_or(0),
            finish_reason: candidate.finish_reason.clone().unwrap_or_default(),
            latency_ms: start.elapsed().as_millis() as u64,
        };
        Ok(SchematicGenerationResult { value, info })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::define_schematic;
    use std::time::Duration;
    use wiremock::matchers::{method, path_regex};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    define_schematic! {
        pub struct TestS { pub a: String, pub b: i32 }
    }

    fn test_config(endpoint: String) -> NlpConfig {
        NlpConfig {
            provider: "gemini".into(),
            model: "gemini-1.5-flash".into(),
            endpoint: Some(endpoint),
            api_key: "test-key".into(),
            max_retries: 0,
            timeout: Duration::from_secs(5),
            temperature: 0.2,
        }
    }

    #[tokio::test]
    async fn gemini_schematic_generator_parses_response() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path_regex("/v1beta/models/.*:generateContent"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                r#"{"candidates":[{"content":{"parts":[{"text":"{\"a\":\"hi\",\"b\":42}"}]},"finishReason":"STOP"}],"usageMetadata":{"promptTokenCount":10,"candidatesTokenCount":5}}"#,
            ))
            .mount(&server)
            .await;
        let gen = GeminiSchematicGenerator::<TestS>::new(Arc::new(test_config(server.uri())));
        let result = gen
            .generate("hello".into(), Default::default())
            .await
            .unwrap();
        assert_eq!(result.value.a, "hi");
        assert_eq!(result.value.b, 42);
        assert_eq!(result.info.prompt_tokens, 10);
        assert_eq!(result.info.completion_tokens, 5);
    }
}
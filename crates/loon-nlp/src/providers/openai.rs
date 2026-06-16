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

pub struct OpenAiSchematicGenerator<T: Schematic> {
    config: Arc<NlpConfig>,
    http: reqwest::Client,
    _marker: std::marker::PhantomData<T>,
}

impl<T: Schematic> OpenAiSchematicGenerator<T> {
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
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    response_format: ResponseFormat,
    temperature: f32,
}

#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    kind: &'static str,
    json_schema: serde_json::Value,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
    usage: Option<Usage>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatRespMessage,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct ChatRespMessage {
    content: String,
}

#[derive(Deserialize, Default)]
struct Usage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[async_trait]
impl<T: Schematic> SchematicGenerator<T> for OpenAiSchematicGenerator<T> {
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
            .unwrap_or_else(|| "https://api.openai.com".to_string());
        let url = format!("{}/v1/chat/completions", endpoint.trim_end_matches('/'));
        let temperature = options.temperature.unwrap_or(self.config.temperature);

        let body = ChatRequest {
            model: &self.config.model,
            messages: vec![ChatMessage {
                role: "user",
                content: &prompt,
            }],
            response_format: ResponseFormat {
                kind: "json_schema",
                json_schema: serde_json::json!({
                    "name": "schematic",
                    "schema": T::schema(),
                }),
            },
            temperature,
        };

        let resp = self
            .http
            .post(&url)
            .bearer_auth(&self.config.api_key)
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

        let parsed: ChatResponse = resp
            .json()
            .await
            .map_err(|e| NlpError::Http(e.to_string()))?;
        let content = parsed
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| NlpError::Upstream("no choices returned".into()))?;

        let value: T = serde_json::from_str(&content)
            .map_err(|e| NlpError::InvalidSchema(e.to_string()))?;

        let info = GenerationInfo {
            model: self.config.model.clone(),
            prompt_tokens: parsed.usage.as_ref().map(|u| u.prompt_tokens).unwrap_or(0),
            completion_tokens: parsed
                .usage
                .as_ref()
                .map(|u| u.completion_tokens)
                .unwrap_or(0),
            total_tokens: parsed.usage.as_ref().map(|u| u.total_tokens).unwrap_or(0),
            finish_reason: parsed
                .choices
                .first()
                .and_then(|c| c.finish_reason.clone())
                .unwrap_or_default(),
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
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    define_schematic! {
        pub struct TestS { pub a: String, pub b: i32 }
    }

    fn test_config(endpoint: String) -> NlpConfig {
        NlpConfig {
            provider: "openai".into(),
            model: "gpt-4o-mini".into(),
            endpoint: Some(endpoint),
            api_key: "test".into(),
            max_retries: 0,
            timeout: Duration::from_secs(5),
            temperature: 0.2,
        }
    }

    #[tokio::test]
    async fn openai_schematic_generator_parses_response() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                r#"{"choices":[{"message":{"content":"{\"a\":\"hi\",\"b\":42}"},"finish_reason":"stop"}],"usage":{"prompt_tokens":3,"completion_tokens":2,"total_tokens":5}}"#,
            ))
            .mount(&server)
            .await;
        let gen = OpenAiSchematicGenerator::<TestS>::new(Arc::new(test_config(server.uri())));
        let result = gen
            .generate("hello".into(), Default::default())
            .await
            .unwrap();
        assert_eq!(result.value.a, "hi");
        assert_eq!(result.value.b, 42);
        assert_eq!(result.info.model, "gpt-4o-mini");
        assert_eq!(result.info.total_tokens, 5);
    }

    #[tokio::test]
    async fn openai_schematic_generator_returns_rate_limited() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(429))
            .mount(&server)
            .await;
        let gen = OpenAiSchematicGenerator::<TestS>::new(Arc::new(test_config(server.uri())));
        let result = gen.generate("hello".into(), Default::default()).await;
        assert!(matches!(result, Err(NlpError::RateLimited { .. })));
    }
}

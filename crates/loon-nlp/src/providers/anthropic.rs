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

pub struct AnthropicSchematicGenerator<T: Schematic> {
    config: Arc<NlpConfig>,
    http: reqwest::Client,
    _marker: std::marker::PhantomData<T>,
}

impl<T: Schematic> AnthropicSchematicGenerator<T> {
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
struct MessagesRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    messages: Vec<AnthropicMessage<'a>>,
    tools: Vec<Tool<'a>>,
    tool_choice: ToolChoice,
}

#[derive(Serialize)]
struct AnthropicMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Serialize)]
struct Tool<'a> {
    name: &'a str,
    description: &'a str,
    input_schema: serde_json::Value,
}

#[derive(Serialize)]
struct ToolChoice {
    #[serde(rename = "type")]
    kind: &'static str,
    name: &'static str,
}

#[derive(Deserialize)]
struct MessagesResponse {
    content: Vec<ContentBlock>,
    #[serde(rename = "stop_reason")]
    stop_reason: Option<String>,
    usage: Option<AnthropicUsage>,
}

#[derive(Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    kind: String,
    name: Option<String>,
    input: Option<serde_json::Value>,
}

#[derive(Deserialize, Default)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

#[async_trait]
impl<T: Schematic> SchematicGenerator<T> for AnthropicSchematicGenerator<T> {
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
            .unwrap_or_else(|| "https://api.anthropic.com".to_string());
        let url = format!("{}/v1/messages", endpoint.trim_end_matches('/'));

        let body = MessagesRequest {
            model: &self.config.model,
            max_tokens: 1024,
            messages: vec![AnthropicMessage {
                role: "user",
                content: &prompt,
            }],
            tools: vec![Tool {
                name: "json_output",
                description: "Emit structured JSON",
                input_schema: T::schema(),
            }],
            tool_choice: ToolChoice {
                kind: "tool",
                name: "json_output",
            },
        };

        let resp = self
            .http
            .post(&url)
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", "2023-06-01")
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

        let parsed: MessagesResponse = resp
            .json()
            .await
            .map_err(|e| NlpError::Http(e.to_string()))?;

        let tool_block = parsed
            .content
            .iter()
            .find(|b| b.kind == "tool_use" && b.name.as_deref() == Some("json_output"))
            .ok_or_else(|| NlpError::Upstream("no tool_use block returned".into()))?;
        let input = tool_block
            .input
            .clone()
            .ok_or_else(|| NlpError::Upstream("tool_use block missing input".into()))?;

        let value: T = serde_json::from_value(input)
            .map_err(|e| NlpError::InvalidSchema(e.to_string()))?;

        let info = GenerationInfo {
            model: self.config.model.clone(),
            prompt_tokens: parsed.usage.as_ref().map(|u| u.input_tokens).unwrap_or(0),
            completion_tokens: parsed.usage.as_ref().map(|u| u.output_tokens).unwrap_or(0),
            total_tokens: parsed
                .usage
                .as_ref()
                .map(|u| u.input_tokens + u.output_tokens)
                .unwrap_or(0),
            finish_reason: parsed.stop_reason.unwrap_or_default(),
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
            provider: "anthropic".into(),
            model: "claude-3-5-sonnet-20241022".into(),
            endpoint: Some(endpoint),
            api_key: "test".into(),
            max_retries: 0,
            timeout: Duration::from_secs(5),
            temperature: 0.2,
        }
    }

    #[tokio::test]
    async fn anthropic_schematic_generator_parses_response() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                r#"{"content":[{"type":"tool_use","name":"json_output","input":{"a":"hi","b":42}}],"stop_reason":"tool_use","usage":{"input_tokens":10,"output_tokens":5}}"#,
            ))
            .mount(&server)
            .await;
        let gen = AnthropicSchematicGenerator::<TestS>::new(Arc::new(test_config(server.uri())));
        let result = gen
            .generate("hello".into(), Default::default())
            .await
            .unwrap();
        assert_eq!(result.value.a, "hi");
        assert_eq!(result.value.b, 42);
        assert_eq!(result.info.prompt_tokens, 10);
        assert_eq!(result.info.completion_tokens, 5);
    }

    #[tokio::test]
    async fn anthropic_schematic_generator_returns_rate_limited() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(429))
            .mount(&server)
            .await;
        let gen = AnthropicSchematicGenerator::<TestS>::new(Arc::new(test_config(server.uri())));
        let result = gen.generate("hello".into(), Default::default()).await;
        assert!(matches!(result, Err(NlpError::RateLimited { .. })));
    }
}
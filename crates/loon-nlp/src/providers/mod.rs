pub mod anthropic;
pub mod gemini;
pub mod openai;
pub mod openai_provider;

pub use anthropic::*;
pub use gemini::*;
pub use openai::*;
pub use openai_provider::*;

use std::sync::Arc;

use async_trait::async_trait;

use crate::config::{NlpConfig, Provider};
use crate::embedding::Embedder;
use crate::error::{NlpError, NlpResult};
use crate::generator::{ErasedSchematicGenerator, GenerationInfo};
use crate::moderation::Moderater;
use crate::service::NlpService;
use crate::tokenization::Tokenizer;
use crate::StreamingTextGenerator;

/// A `NlpService` that dispatches to the provider configured in `NlpConfig`.
/// Only the OpenAI side has full streaming/embedding/moderation coverage;
/// Anthropic and Gemini are routed for schematic generation (the primary
/// engine use-case in Phase 3), and fall through to OpenAI's stubs for the
/// other capabilities.
pub struct MultiProvider {
    config: Arc<NlpConfig>,
}

impl MultiProvider {
    pub fn new(config: Arc<NlpConfig>) -> Self {
        Self { config }
    }

    pub fn provider_kind(&self) -> Provider {
        Provider::from_str(&self.config.provider).unwrap_or(Provider::OpenAI)
    }

    fn build_http(&self) -> NlpResult<reqwest::Client> {
        reqwest::Client::builder()
            .timeout(self.config.timeout)
            .build()
            .map_err(|e| NlpError::Http(e.to_string()))
    }
}

#[async_trait]
impl NlpService for MultiProvider {
    fn config(&self) -> &NlpConfig {
        &self.config
    }

    async fn text_generator(&self) -> NlpResult<Box<dyn StreamingTextGenerator>> {
        // Phase 3: only the OpenAI text generator is wired in; the others
        // fall through to the existing Phase 1 stub.
        let http = self.build_http()?;
        Ok(Box::new(OpenAiTextGenerator {
            config: self.config.clone(),
            http,
        }))
    }

    async fn schematic_generator(
        &self,
        _schema: serde_json::Value,
    ) -> NlpResult<Box<dyn ErasedSchematicGenerator>> {
        // Phase 3: the typed `SchematicGenerator<T>` is the primary
        // Phase 3 deliverable; the type-erased adapter path continues
        // to use the Phase 1 stub. Wiring `TypedErasedSchematicGenerator`
        // through every provider is deferred to the integration stage
        // where a `JsonValue: Schematic` boundary is finalised.
        Ok(Box::new(StubErasedSchematicGenerator))
    }

    async fn embedder(&self) -> NlpResult<Box<dyn Embedder>> {
        Ok(Box::new(OpenAiEmbedder {
            config: self.config.clone(),
        }))
    }

    async fn tokenizer(&self) -> NlpResult<Box<dyn Tokenizer>> {
        Ok(Box::new(OpenAiTokenizer))
    }

    async fn moderater(&self) -> NlpResult<Box<dyn Moderater>> {
        Ok(Box::new(OpenAiModerater))
    }
}

/// A local `JsonValue`-shaped newtype that implements `Schematic` so
/// `TypedErasedSchematicGenerator<JsonValue>` can serve as the erased
/// adapter for every provider.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct JsonValue(pub serde_json::Value);

impl crate::Schematic for JsonValue {
    fn schema() -> serde_json::Value {
        serde_json::json!({})
    }
}

/// Fall-through generation info helper for stubs.
pub fn empty_generation_info() -> GenerationInfo {
    GenerationInfo::default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn cfg_for(provider: &str) -> NlpConfig {
        NlpConfig {
            provider: provider.into(),
            model: "any".into(),
            endpoint: None,
            api_key: "test".into(),
            max_retries: 0,
            timeout: Duration::from_secs(5),
            temperature: 0.0,
        }
    }

    #[test]
    fn multi_provider_dispatches_provider_kind() {
        let m = MultiProvider::new(Arc::new(cfg_for("openai")));
        assert_eq!(m.provider_kind(), Provider::OpenAI);
        let m = MultiProvider::new(Arc::new(cfg_for("anthropic")));
        assert_eq!(m.provider_kind(), Provider::Anthropic);
        let m = MultiProvider::new(Arc::new(cfg_for("claude")));
        assert_eq!(m.provider_kind(), Provider::Anthropic);
        let m = MultiProvider::new(Arc::new(cfg_for("gemini")));
        assert_eq!(m.provider_kind(), Provider::Gemini);
        let m = MultiProvider::new(Arc::new(cfg_for("bogus")));
        assert_eq!(m.provider_kind(), Provider::OpenAI);
    }

    #[tokio::test]
    async fn multi_provider_returns_erased_schematic_for_each_provider() {
        let m = MultiProvider::new(Arc::new(cfg_for("openai")));
        let _gen: Box<dyn ErasedSchematicGenerator> =
            m.schematic_generator(serde_json::json!({})).await.unwrap();
        let m = MultiProvider::new(Arc::new(cfg_for("anthropic")));
        let _gen: Box<dyn ErasedSchematicGenerator> =
            m.schematic_generator(serde_json::json!({})).await.unwrap();
        let m = MultiProvider::new(Arc::new(cfg_for("gemini")));
        let _gen: Box<dyn ErasedSchematicGenerator> =
            m.schematic_generator(serde_json::json!({})).await.unwrap();
    }

    #[tokio::test]
    async fn multi_provider_other_capabilities_default_to_openai_stubs() {
        let m = MultiProvider::new(Arc::new(cfg_for("anthropic")));
        let _t = m.text_generator().await.unwrap();
        let _e = m.embedder().await.unwrap();
        let _tok = m.tokenizer().await.unwrap();
        let _mod = m.moderater().await.unwrap();
    }
}
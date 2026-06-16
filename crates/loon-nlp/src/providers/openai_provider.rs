use std::sync::Arc;

use async_trait::async_trait;

use crate::config::NlpConfig;
use crate::embedding::Embedder;
use crate::generator::{
    GenerationInfo, StreamingTextGenerationResult, StreamingTextGenerator, TextGenerationOptions,
};
use crate::moderation::{Moderater, ModerationResult};
use crate::service::NlpService;
use crate::tokenization::Tokenizer;
use crate::{NlpResult, Schematic, SchematicGenerator};
use crate::error::NlpError;

use super::openai::OpenAiSchematicGenerator;

pub struct OpenAiProvider {
    config: Arc<NlpConfig>,
}

impl OpenAiProvider {
    pub fn new(config: Arc<NlpConfig>) -> Self {
        Self { config }
    }
}

pub struct OpenAiTextGenerator {
    pub config: Arc<NlpConfig>,
    pub http: reqwest::Client,
}

#[async_trait]
impl StreamingTextGenerator for OpenAiTextGenerator {
    async fn generate(
        &self,
        _prompt: String,
        _options: TextGenerationOptions,
    ) -> NlpResult<StreamingTextGenerationResult> {
        // Phase 1 stub; real impl deferred (Stage 6 AlphaEngine uses SchematicGenerator primarily)
        Ok(StreamingTextGenerationResult {
            text: String::new(),
            info: GenerationInfo::default(),
        })
    }
}

pub struct OpenAiEmbedder {
    pub config: Arc<NlpConfig>,
}

#[async_trait]
impl Embedder for OpenAiEmbedder {
    async fn embed(&self, _text: &str) -> NlpResult<Vec<f32>> {
        Ok(vec![])
    }
}

pub struct OpenAiTokenizer;

#[async_trait]
impl Tokenizer for OpenAiTokenizer {
    async fn count_tokens(&self, text: &str) -> NlpResult<u32> {
        Ok(text.split_whitespace().count() as u32)
    }
}

pub struct OpenAiModerater;

#[async_trait]
impl Moderater for OpenAiModerater {
    async fn moderate(&self, _text: &str) -> NlpResult<ModerationResult> {
        Ok(ModerationResult {
            flagged: false,
            categories: Default::default(),
            scores: Default::default(),
        })
    }
}

pub struct StubErasedSchematicGenerator;

#[async_trait]
impl crate::ErasedSchematicGenerator for StubErasedSchematicGenerator {
    async fn generate(
        &self,
        _prompt: String,
        _options: crate::SchematicGenerationOptions,
    ) -> NlpResult<crate::ErasedSchematicGenerationResult> {
        Ok(crate::ErasedSchematicGenerationResult {
            value: serde_json::Value::Null,
            info: GenerationInfo::default(),
        })
    }
}

#[async_trait]
impl NlpService for OpenAiProvider {
    fn config(&self) -> &NlpConfig {
        &self.config
    }
    async fn text_generator(&self) -> NlpResult<Box<dyn StreamingTextGenerator>> {
        let http = reqwest::Client::builder()
            .timeout(self.config.timeout)
            .build()
            .map_err(|e| NlpError::Http(e.to_string()))?;
        Ok(Box::new(OpenAiTextGenerator {
            config: self.config.clone(),
            http,
        }))
    }
    async fn schematic_generator(
        &self,
        _schema: serde_json::Value,
    ) -> NlpResult<Box<dyn crate::ErasedSchematicGenerator>> {
        // Phase 1: return a stub `ErasedSchematicGenerator` so callers
        // can still exercise the type-erased path. The full impl will
        // wrap `OpenAiSchematicGenerator<JsonValue>` once `JsonValue`
        // implements `Schematic`.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::define_schematic;
    use std::time::Duration;

    define_schematic! {
        pub struct TestS { pub a: String }
    }

    fn test_config(endpoint: &str) -> NlpConfig {
        NlpConfig {
            provider: "openai".into(),
            model: "gpt-4o-mini".into(),
            endpoint: Some(endpoint.into()),
            api_key: "test".into(),
            max_retries: 0,
            timeout: Duration::from_secs(5),
            temperature: 0.2,
        }
    }

    #[tokio::test]
    async fn openai_provider_returns_schematic_generator() {
        let p = OpenAiProvider::new(Arc::new(test_config("http://x")));
        let _gen: Box<dyn crate::ErasedSchematicGenerator> =
            p.schematic_generator(TestS::schema()).await.unwrap();
    }
}

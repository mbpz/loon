//! Test helpers for the `loon-nlp` crate.
//!
//! `FakeNlpService` and `EchoSchematicGenerator` provide just enough
//! behaviour for engine and matcher tests without contacting any real
//! upstream.

use std::time::Duration;

use async_trait::async_trait;

use crate::{
    Embedder, GenerationInfo, Moderater, ModerationResult, NlpConfig, NlpResult, NlpService,
    Schematic, SchematicGenerationOptions, SchematicGenerationResult, SchematicGenerator,
    StreamingTextGenerationResult, StreamingTextGenerator, TextGenerationOptions, Tokenizer,
};

/// A `SchematicGenerator` that always returns `Err(NlpError::Upstream)`.
pub struct AlwaysFailingSchematicGen;

#[async_trait]
impl<T> SchematicGenerator<T> for AlwaysFailingSchematicGen
where
    T: Schematic + serde::de::DeserializeOwned + Send + 'static,
{
    async fn generate(
        &self,
        _prompt: String,
        _options: SchematicGenerationOptions,
    ) -> NlpResult<SchematicGenerationResult<T>> {
        Err(crate::error::NlpError::Upstream("primary failed".into()))
    }
}

/// A `SchematicGenerator` that always returns `Ok(T::default())`.
pub struct SuccessSchematicGen;

#[async_trait]
impl<T> SchematicGenerator<T> for SuccessSchematicGen
where
    T: Schematic + serde::de::DeserializeOwned + Default + Send + 'static,
{
    async fn generate(
        &self,
        _prompt: String,
        _options: SchematicGenerationOptions,
    ) -> NlpResult<SchematicGenerationResult<T>> {
        Ok(SchematicGenerationResult {
            value: T::default(),
            info: GenerationInfo::default(),
        })
    }
}

/// A `SchematicGenerator` that always returns `T::default()`.
pub struct EchoSchematicGenerator;

#[async_trait]
impl<T> SchematicGenerator<T> for EchoSchematicGenerator
where
    T: Schematic + Default + Send + 'static,
{
    async fn generate(
        &self,
        _prompt: String,
        _options: SchematicGenerationOptions,
    ) -> NlpResult<SchematicGenerationResult<T>> {
        Ok(SchematicGenerationResult {
            value: T::default(),
            info: GenerationInfo::default(),
        })
    }
}

/// A trivial `NlpService` for tests. Only `schematic_generator` is
/// implemented meaningfully; the other producers `unimplemented!()`.
pub struct FakeNlpService {
    pub config: NlpConfig,
}

impl FakeNlpService {
    pub fn new() -> Self {
        Self {
            config: NlpConfig {
                provider: "fake".into(),
                model: "fake".into(),
                endpoint: None,
                api_key: String::new(),
                max_retries: 0,
                timeout: Duration::from_secs(1),
                temperature: 0.0,
            },
        }
    }
}

impl Default for FakeNlpService {
    fn default() -> Self {
        Self::new()
    }
}

pub struct FakeTextGen;
#[async_trait]
impl StreamingTextGenerator for FakeTextGen {
    async fn generate(
        &self,
        _prompt: String,
        _options: TextGenerationOptions,
    ) -> NlpResult<StreamingTextGenerationResult> {
        Ok(StreamingTextGenerationResult {
            text: String::new(),
            info: GenerationInfo::default(),
        })
    }
}

pub struct FakeEmbedder;
#[async_trait]
impl Embedder for FakeEmbedder {
    async fn embed(&self, _text: &str) -> NlpResult<Vec<f32>> {
        Ok(vec![0.0])
    }
}

pub struct FakeTokenizer;
#[async_trait]
impl Tokenizer for FakeTokenizer {
    async fn count_tokens(&self, text: &str) -> NlpResult<u32> {
        Ok(text.split_whitespace().count() as u32)
    }
}

pub struct FakeModerater;
#[async_trait]
impl Moderater for FakeModerater {
    async fn moderate(&self, _text: &str) -> NlpResult<ModerationResult> {
        Ok(ModerationResult::default())
    }
}

#[async_trait]
impl NlpService for FakeNlpService {
    fn config(&self) -> &NlpConfig {
        &self.config
    }
    async fn text_generator(&self) -> NlpResult<Box<dyn StreamingTextGenerator>> {
        Ok(Box::new(FakeTextGen))
    }
    async fn schematic_generator(
        &self,
        _schema: serde_json::Value,
    ) -> NlpResult<Box<dyn crate::ErasedSchematicGenerator>> {
        Ok(Box::new(EchoErasedSchematicGenerator))
    }
    async fn embedder(&self) -> NlpResult<Box<dyn Embedder>> {
        Ok(Box::new(FakeEmbedder))
    }
    async fn tokenizer(&self) -> NlpResult<Box<dyn Tokenizer>> {
        Ok(Box::new(FakeTokenizer))
    }
    async fn moderater(&self) -> NlpResult<Box<dyn Moderater>> {
        Ok(Box::new(FakeModerater))
    }
}

/// Always returns `serde_json::Value::Null`.
pub struct EchoErasedSchematicGenerator;
#[async_trait]
impl crate::ErasedSchematicGenerator for EchoErasedSchematicGenerator {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::define_schematic;

    define_schematic! {
        pub struct Tiny { pub a: String }
    }

    #[tokio::test]
    async fn echo_schematic_returns_default() {
        let g = EchoSchematicGenerator;
        let r: SchematicGenerationResult<Tiny> = g
            .generate("p".into(), SchematicGenerationOptions::default())
            .await
            .unwrap();
        assert_eq!(r.value.a, "");
    }

    #[tokio::test]
    async fn fake_nlp_provides_schematic_generator() {
        let s = FakeNlpService::new();
        let gen: Box<dyn crate::ErasedSchematicGenerator> =
            s.schematic_generator(Tiny::schema()).await.unwrap();
        let r = gen
            .generate("p".into(), SchematicGenerationOptions::default())
            .await
            .unwrap();
        assert_eq!(r.value, serde_json::Value::Null);
    }
}

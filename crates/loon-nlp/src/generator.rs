use async_trait::async_trait;

use crate::{NlpResult, Schematic};

#[derive(Debug, Clone, Default)]
pub struct TextGenerationOptions {
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, Default)]
pub struct SchematicGenerationOptions {
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, Default)]
pub struct GenerationInfo {
    pub model: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub finish_reason: String,
    pub latency_ms: u64,
}

#[derive(Debug, Clone)]
pub struct StreamingTextGenerationResult {
    pub text: String,
    pub info: GenerationInfo,
}

#[derive(Debug, Clone)]
pub struct SchematicGenerationResult<T> {
    pub value: T,
    pub info: GenerationInfo,
}

#[async_trait]
pub trait StreamingTextGenerator: Send + Sync {
    async fn generate(
        &self,
        prompt: String,
        options: TextGenerationOptions,
    ) -> NlpResult<StreamingTextGenerationResult>;
}

#[async_trait]
pub trait SchematicGenerator<T: Schematic>: Send + Sync {
    async fn generate(
        &self,
        prompt: String,
        options: SchematicGenerationOptions,
    ) -> NlpResult<SchematicGenerationResult<T>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::define_schematic;
    use std::sync::Arc;

    define_schematic! {
        pub struct TestS { pub a: String }
    }

    struct DummyGen;
    #[async_trait]
    impl SchematicGenerator<TestS> for DummyGen {
        async fn generate(
            &self,
            _prompt: String,
            _options: SchematicGenerationOptions,
        ) -> NlpResult<SchematicGenerationResult<TestS>> {
            Ok(SchematicGenerationResult {
                value: TestS { a: "ok".into() },
                info: GenerationInfo::default(),
            })
        }
    }

    #[tokio::test]
    async fn compile_and_dispatch() {
        let g: Arc<dyn SchematicGenerator<TestS>> = Arc::new(DummyGen);
        let r = g
            .generate("p".into(), SchematicGenerationOptions::default())
            .await
            .unwrap();
        assert_eq!(r.value.a, "ok");
    }
}

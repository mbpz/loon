use async_trait::async_trait;

use crate::{
    Embedder, ErasedSchematicGenerator, Moderater, NlpConfig, NlpResult, StreamingTextGenerator,
    Tokenizer,
};

#[async_trait]
pub trait NlpService: Send + Sync {
    fn config(&self) -> &NlpConfig;
    async fn text_generator(&self) -> NlpResult<Box<dyn StreamingTextGenerator>>;
    /// Type-erased schematic generator. Callers convert the
    /// returned `JsonValue` to their target type via
    /// `serde_json::from_value`.
    async fn schematic_generator(
        &self,
        schema: serde_json::Value,
    ) -> NlpResult<Box<dyn ErasedSchematicGenerator>>;
    async fn embedder(&self) -> NlpResult<Box<dyn Embedder>>;
    async fn tokenizer(&self) -> NlpResult<Box<dyn Tokenizer>>;
    async fn moderater(&self) -> NlpResult<Box<dyn Moderater>>;
}

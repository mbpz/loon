use async_trait::async_trait;

use crate::{
    Embedder, Moderater, NlpConfig, NlpResult, Schematic, SchematicGenerator, StreamingTextGenerator,
    Tokenizer,
};

#[async_trait]
pub trait NlpService: Send + Sync {
    fn config(&self) -> &NlpConfig;
    async fn text_generator(&self) -> NlpResult<Box<dyn StreamingTextGenerator>>;
    async fn schematic_generator<T: Schematic + 'static>(
        &self,
    ) -> NlpResult<Box<dyn SchematicGenerator<T>>>;
    async fn embedder(&self) -> NlpResult<Box<dyn Embedder>>;
    async fn tokenizer(&self) -> NlpResult<Box<dyn Tokenizer>>;
    async fn moderater(&self) -> NlpResult<Box<dyn Moderater>>;
}

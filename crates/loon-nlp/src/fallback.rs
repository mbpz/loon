use async_trait::async_trait;

use crate::error::NlpResult;
use crate::generator::{SchematicGenerationOptions, SchematicGenerationResult, SchematicGenerator};
use crate::Schematic;

pub struct FallbackSchematicGenerator<T: Schematic> {
    pub primary: Box<dyn SchematicGenerator<T>>,
    pub fallbacks: Vec<Box<dyn SchematicGenerator<T>>>,
}

impl<T: Schematic> FallbackSchematicGenerator<T> {
    pub fn new(
        primary: Box<dyn SchematicGenerator<T>>,
        fallbacks: Vec<Box<dyn SchematicGenerator<T>>>,
    ) -> Self {
        Self { primary, fallbacks }
    }
}

#[async_trait]
impl<T: Schematic> SchematicGenerator<T> for FallbackSchematicGenerator<T> {
    async fn generate(
        &self,
        prompt: String,
        options: SchematicGenerationOptions,
    ) -> NlpResult<SchematicGenerationResult<T>> {
        // Phase 1: just call primary
        self.primary.generate(prompt, options).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::define_schematic;
    use crate::generator::GenerationInfo;

    define_schematic! {
        pub struct TestS { pub a: String }
    }

    struct FixedGen(String);
    #[async_trait]
    impl SchematicGenerator<TestS> for FixedGen {
        async fn generate(
            &self,
            _prompt: String,
            _options: SchematicGenerationOptions,
        ) -> NlpResult<SchematicGenerationResult<TestS>> {
            Ok(SchematicGenerationResult {
                value: TestS { a: self.0.clone() },
                info: GenerationInfo::default(),
            })
        }
    }

    #[tokio::test]
    async fn fallback_calls_primary() {
        let primary = Box::new(FixedGen("p".into()));
        let gen = FallbackSchematicGenerator::<TestS>::new(primary, vec![]);
        let r = gen
            .generate("hello".into(), SchematicGenerationOptions::default())
            .await
            .unwrap();
        assert_eq!(r.value.a, "p");
    }
}

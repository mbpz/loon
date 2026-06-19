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
        match self.primary.generate(prompt.clone(), options.clone()).await {
            Ok(r) => Ok(r),
            Err(primary_err) => {
                let last_idx = self.fallbacks.len().saturating_sub(1);
                for (idx, fallback) in self.fallbacks.iter().enumerate() {
                    match fallback.generate(prompt.clone(), options.clone()).await {
                        Ok(r) => return Ok(r),
                        Err(_e) => {
                            if idx == last_idx {
                                // All fallbacks exhausted; bubble up the
                                // *last* fallback's error so callers see
                                // the most recent failure mode.
                                return Err(_e);
                            }
                        }
                    }
                }
                // No fallbacks configured at all.
                Err(primary_err)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::define_schematic;
    use crate::generator::GenerationInfo;
    use crate::test_utils::{AlwaysFailingSchematicGen, SuccessSchematicGen};

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

    #[tokio::test]
    async fn fallback_picks_secondary_when_primary_fails() {
        let primary: Box<dyn SchematicGenerator<TestS>> = Box::new(AlwaysFailingSchematicGen);
        let secondary: Box<dyn SchematicGenerator<TestS>> = Box::new(SuccessSchematicGen);
        let gen = FallbackSchematicGenerator::<TestS>::new(primary, vec![secondary]);
        let r = gen
            .generate("hello".into(), SchematicGenerationOptions::default())
            .await
            .unwrap();
        // SuccessSchematicGen returns TestS::default() -> a == ""
        assert_eq!(r.value.a, "");
    }

    #[tokio::test]
    async fn fallback_returns_error_when_all_exhausted() {
        let primary: Box<dyn SchematicGenerator<TestS>> = Box::new(AlwaysFailingSchematicGen);
        let secondary: Box<dyn SchematicGenerator<TestS>> = Box::new(AlwaysFailingSchematicGen);
        let gen = FallbackSchematicGenerator::<TestS>::new(primary, vec![secondary]);
        let r = gen
            .generate("hello".into(), SchematicGenerationOptions::default())
            .await;
        assert!(r.is_err());
    }

    #[tokio::test]
    async fn fallback_returns_primary_error_when_no_fallbacks_configured() {
        let primary: Box<dyn SchematicGenerator<TestS>> = Box::new(AlwaysFailingSchematicGen);
        let gen = FallbackSchematicGenerator::<TestS>::new(primary, vec![]);
        let r = gen
            .generate("hello".into(), SchematicGenerationOptions::default())
            .await;
        assert!(r.is_err());
    }

    #[tokio::test]
    async fn fallback_skips_failing_first_fallback() {
        let primary: Box<dyn SchematicGenerator<TestS>> = Box::new(AlwaysFailingSchematicGen);
        let first_fallback: Box<dyn SchematicGenerator<TestS>> =
            Box::new(AlwaysFailingSchematicGen);
        let second_fallback: Box<dyn SchematicGenerator<TestS>> = Box::new(FixedGen("ok".into()));
        let gen = FallbackSchematicGenerator::<TestS>::new(
            primary,
            vec![first_fallback, second_fallback],
        );
        let r = gen
            .generate("hello".into(), SchematicGenerationOptions::default())
            .await
            .unwrap();
        assert_eq!(r.value.a, "ok");
    }
}

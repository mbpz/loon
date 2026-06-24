//! LLM-backed `GuidelineMatcher`.

use std::sync::Arc;

use async_trait::async_trait;

use loon_core::Guideline;
use loon_nlp::{define_schematic, NlpService, Schematic};

use crate::engine_context::GuidelineMatch;
use crate::error::{EngineError, EngineResult};
use crate::guideline_matching::context::GuidelineMatchingContext;
use crate::guideline_matching::matcher::GuidelineMatcher;

define_schematic! {
    pub struct LlmMatchOutput {
        pub guideline_id: String,
        pub confidence: f32,
        pub rationale: String,
    }
}

pub struct LlmGuidelineMatcher {
    pub nlp: Arc<dyn NlpService>,
}

impl LlmGuidelineMatcher {
    pub fn new(nlp: Arc<dyn NlpService>) -> Self {
        Self { nlp }
    }
}

#[async_trait]
impl GuidelineMatcher for LlmGuidelineMatcher {
    async fn match_guidelines(
        &self,
        ctx: &GuidelineMatchingContext,
    ) -> EngineResult<Vec<GuidelineMatch>> {
        // Phase 1a: split off "always"-condition guidelines and
        // short-circuit them with confidence 1.0. Always-match is the
        // common case for simple agents and routinely the *only* kind
        // of guideline configured — when every guideline is always-on
        // we skip the LLM call entirely.
        let mut matches: Vec<GuidelineMatch> = Vec::new();
        let mut llm_candidates: Vec<&Guideline> = Vec::new();
        for g in &ctx.guidelines {
            if g.enabled && g.content.condition.trim().eq_ignore_ascii_case("always") {
                matches.push(GuidelineMatch {
                    guideline: g.clone(),
                    confidence: 1.0,
                    rationale: "always-match guideline".into(),
                });
            } else {
                llm_candidates.push(g);
            }
        }
        if llm_candidates.is_empty() {
            return Ok(matches);
        }

        // Phase 1b: ask the LLM about the remaining candidates only.
        let prompt = format!(
            "Match guidelines for message: '{}'\nGuidelines:\n{}",
            ctx.interaction
                .last_customer_message()
                .map(|m| m.content)
                .unwrap_or_default(),
            llm_candidates
                .iter()
                .map(|g| format!("- {}: {}", g.id, g.content.condition))
                .collect::<Vec<_>>()
                .join("\n")
        );

        let generator = self
            .nlp
            .schematic_generator(LlmMatchOutput::schema())
            .await
            .map_err(|e| EngineError::GuidelineMatchingFailed(e.to_string()))?;
        let result = generator
            .generate(prompt, Default::default())
            .await
            .map_err(|e| EngineError::GuidelineMatchingFailed(e.to_string()))?;

        // Convert the erased JSON value to our typed struct.
        let parsed: LlmMatchOutput = serde_json::from_value(result.value).unwrap_or_default();

        for g in llm_candidates {
            if parsed.guideline_id == g.id.0 {
                matches.push(GuidelineMatch {
                    guideline: g.clone(),
                    confidence: parsed.confidence,
                    rationale: parsed.rationale.clone(),
                });
            }
        }

        Ok(matches)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use loon_core::{
        Agent, AgentId, Criticality, CustomerId, Event, EventId, EventKind, EventSource, Guideline,
        GuidelineContent, Session, SessionId, SessionMode, TagId, UniqueId,
    };
    use loon_nlp::test_utils::FakeNlpService;

    use crate::engine_context::Interaction;

    #[tokio::test]
    async fn llm_matcher_runs_without_panic() {
        let nlp: Arc<dyn NlpService> = Arc::new(FakeNlpService::new());
        let matcher = LlmGuidelineMatcher::new(nlp.clone());

        let agent = Agent::new("a", "b");
        let session = Session {
            id: SessionId::new(),
            agent_id: agent.id.clone(),
            customer_id: None,
            title: None,
            mode: SessionMode::Auto,
            labels: Default::default(),
            creation_utc: chrono::Utc::now(),
        };
        let g = Guideline {
            id: loon_core::GuidelineId::new(),
            agent_id: agent.id.clone(),
            content: GuidelineContent {
                condition: "x".into(),
                action: "y".into(),
                description: None,
            },
            criticality: Criticality::Low,
            enabled: true,
            tags: vec![],
            creation_utc: chrono::Utc::now(),
            metadata: loon_core::JsonValue::Null,
        };

        let event = Event {
            id: EventId::new(),
            source: EventSource::Customer,
            kind: EventKind::Message,
            trace_id: "t".into(),
            data: serde_json::json!({"message": "hi"}),
            metadata: None,
            creation_utc: chrono::Utc::now(),
        };
        let interaction = Interaction::new(vec![event]);

        let ctx = GuidelineMatchingContext {
            agent,
            session,
            interaction,
            guidelines: vec![g],
            glossary_terms: vec![],
            nlp: Arc::new(FakeNlpService::new()),
        };
        let result = matcher.match_guidelines(&ctx).await.unwrap();
        // Echo schematic returns Default which has empty guideline_id, so
        // there will be no match (the test guideline's id is non-empty).
        assert!(result.is_empty());

        // Silence unused warnings on cross-crate types.
        let _ = (
            UniqueId::default(),
            CustomerId::new(),
            TagId::new(),
            AgentId::new(),
        );
    }

    #[tokio::test]
    async fn always_match_guidelines_skips_llm() {
        // A schematic generator that, if invoked, panics — proving the
        // matcher never hits the LLM path when every guideline is
        // unconditional ("always").
        struct ExplodingNlp(FakeNlpService);
        #[async_trait]
        impl NlpService for ExplodingNlp {
            fn config(&self) -> &loon_nlp::NlpConfig {
                self.0.config()
            }
            async fn text_generator(
                &self,
            ) -> loon_nlp::NlpResult<Box<dyn loon_nlp::StreamingTextGenerator>> {
                self.0.text_generator().await
            }
            async fn schematic_generator(
                &self,
                _: serde_json::Value,
            ) -> loon_nlp::NlpResult<Box<dyn loon_nlp::ErasedSchematicGenerator>> {
                panic!("LLM schematic_generator must not be called for always-match guidelines");
            }
            async fn embedder(&self) -> loon_nlp::NlpResult<Box<dyn loon_nlp::Embedder>> {
                self.0.embedder().await
            }
            async fn tokenizer(&self) -> loon_nlp::NlpResult<Box<dyn loon_nlp::Tokenizer>> {
                self.0.tokenizer().await
            }
            async fn moderater(&self) -> loon_nlp::NlpResult<Box<dyn loon_nlp::Moderater>> {
                self.0.moderater().await
            }
        }

        let nlp: Arc<dyn NlpService> = Arc::new(ExplodingNlp(FakeNlpService::new()));
        let matcher = LlmGuidelineMatcher::new(nlp);

        let agent = Agent::new("a", "b");
        let session = Session {
            id: SessionId::new(),
            agent_id: agent.id.clone(),
            customer_id: None,
            title: None,
            mode: SessionMode::Auto,
            labels: Default::default(),
            creation_utc: chrono::Utc::now(),
        };
        let g = Guideline {
            id: loon_core::GuidelineId::new(),
            agent_id: agent.id.clone(),
            content: GuidelineContent {
                condition: "always".into(),
                action: "greet user".into(),
                description: None,
            },
            criticality: Criticality::Low,
            enabled: true,
            tags: vec![],
            creation_utc: chrono::Utc::now(),
            metadata: loon_core::JsonValue::Null,
        };

        let ctx = GuidelineMatchingContext {
            agent,
            session,
            interaction: Interaction::new(vec![]),
            guidelines: vec![g],
            glossary_terms: vec![],
            nlp: Arc::new(FakeNlpService::new()),
        };
        let matches = matcher.match_guidelines(&ctx).await.unwrap();
        assert_eq!(matches.len(), 1);
        assert!((matches[0].confidence - 1.0).abs() < 0.01);
        assert_eq!(matches[0].guideline.content.action, "greet user");
        assert_eq!(matches[0].rationale, "always-match guideline");
    }
}

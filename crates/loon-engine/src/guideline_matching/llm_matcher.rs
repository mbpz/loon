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
        // Phase 1: simple impl — call LLM with prompt listing all
        // guidelines and the last message; pick the matching guideline
        // (if any) whose id matches the LLM's answer.
        let prompt = format!(
            "Match guidelines for message: '{}'\nGuidelines:\n{}",
            ctx.interaction
                .last_customer_message()
                .map(|m| m.content)
                .unwrap_or_default(),
            ctx.guidelines
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

        let matches: Vec<GuidelineMatch> = ctx
            .guidelines
            .iter()
            .filter_map(|g: &Guideline| {
                if parsed.guideline_id == g.id.0 {
                    Some(GuidelineMatch {
                        guideline: g.clone(),
                        confidence: parsed.confidence,
                        rationale: parsed.rationale.clone(),
                    })
                } else {
                    None
                }
            })
            .collect();

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
}

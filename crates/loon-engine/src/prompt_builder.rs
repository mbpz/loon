//! `PromptBuilder` — assembles LLM prompts from a prepared
//! `EngineContext` (matched guidelines, glossary terms, context
//! variables, tool results, journey state, capabilities, canned
//! responses) under a token budget.

use std::sync::Arc;

use loon_core::{
    Agent, CannedResponse, Capability, ContextVariable, ContextVariableValue, JourneyNode, Term,
};
use loon_nlp::Tokenizer;

use crate::engine_context::{GuidelineMatch, Interaction};
use crate::error::EngineResult;

/// Builds prompts for the LLM under a fixed token budget.
pub struct PromptBuilder {
    pub tokenizer: Arc<dyn Tokenizer>,
    pub max_tokens: usize,
}

impl PromptBuilder {
    pub fn new(tokenizer: Arc<dyn Tokenizer>, max_tokens: usize) -> Self {
        Self {
            tokenizer,
            max_tokens,
        }
    }
}

impl PromptBuilder {
    /// Build the final generation prompt from a fully-prepared
    /// `EngineContext`. Phase 1 produces a flat string with the
    /// matched guidelines; real implementation will budget
    /// sections, truncate tool results, and inject glossary/
    /// variable context.
    #[allow(clippy::too_many_arguments)]
    pub async fn build_prompt(
        &self,
        _agent: &Agent,
        _interaction: &Interaction,
        matched_guidelines: &[GuidelineMatch],
        _glossary_terms: &[Term],
        _context_variables: &[(ContextVariable, ContextVariableValue)],
        _tool_results: &[crate::tool_calling::caller::ToolExecutionResult],
        _journey_state: Option<&JourneyNode>,
        _capabilities: &[Capability],
        _canned_responses: &[CannedResponse],
    ) -> EngineResult<String> {
        let mut prompt = String::new();
        prompt.push_str("You are an AI agent. Follow these guidelines:\n");
        for m in matched_guidelines {
            prompt.push_str(&format!(
                "- {} (confidence {:.2}): {}\n",
                m.guideline.content.condition, m.confidence, m.guideline.content.action
            ));
        }
        Ok(prompt)
    }

    /// Build a prompt that asks an LLM to score which guidelines
    /// apply to the current interaction.
    pub async fn build_guideline_matching_prompt(
        &self,
        guidelines: &[loon_core::Guideline],
        interaction: &Interaction,
        _glossary: &[Term],
    ) -> EngineResult<String> {
        Ok(format!(
            "Match these guidelines for: {:?}\nGuidelines:\n{}",
            interaction.last_customer_message().map(|m| m.content),
            guidelines
                .iter()
                .map(|g| format!("- {}: {}", g.id, g.content.condition))
                .collect::<Vec<_>>()
                .join("\n")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use loon_core::{AgentId, Guideline, GuidelineContent};

    struct WsTok;
    #[async_trait]
    impl Tokenizer for WsTok {
        async fn count_tokens(&self, text: &str) -> NlpResult<u32> {
            Ok(text.split_whitespace().count() as u32)
        }
    }

    use loon_nlp::NlpResult;

    fn mk_guideline(action: &str) -> GuidelineMatch {
        let g = Guideline::new(
            GuidelineContent {
                condition: "user greets".into(),
                action: action.into(),
                description: None,
            },
            &AgentId::new(),
            true,
            0,
        );
        GuidelineMatch {
            guideline: g,
            confidence: 0.95,
            rationale: "r".into(),
        }
    }

    #[tokio::test]
    async fn build_prompt_includes_action_text() {
        let tok: Arc<dyn Tokenizer> = Arc::new(WsTok);
        let pb = PromptBuilder::new(tok, 1000);
        let agent = Agent::new("a", "b");
        let interaction = Interaction::new(vec![]);
        let matches = vec![mk_guideline("say hi back")];

        let prompt = pb
            .build_prompt(
                &agent,
                &interaction,
                &matches,
                &[],
                &[],
                &[],
                None,
                &[],
                &[],
            )
            .await
            .unwrap();

        assert!(
            prompt.contains("say hi back"),
            "prompt missing action: {prompt}"
        );
        assert!(prompt.contains("user greets"));
    }
}

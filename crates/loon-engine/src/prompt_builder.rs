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
use crate::tool_calling::caller::ToolExecutionResult;

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

/// Result of [`PromptBuilder::build_prompt_with_stats`].
#[derive(Debug, Clone)]
pub struct PromptBuildResult {
    /// The assembled (and possibly truncated) prompt text.
    pub prompt: String,
    /// Number of conversation messages dropped to fit the token budget.
    /// `0` if no truncation was needed or if the conversation was empty.
    pub dropped_messages: usize,
    /// Token count of the final prompt (approximate; uses the configured
    /// tokenizer). `0` if the tokenizer failed to count.
    pub tokens_used: u32,
    /// `true` if the history section was dropped entirely (budget was
    /// unsatisfiable even with no history).
    pub history_dropped_entirely: bool,
}

impl PromptBuilder {
    /// Build the final agent-response prompt. Sections (in order):
    ///   1. Agent identity and description
    ///   2. Glossary terms (if any)
    ///   3. Context variables (if any)
    ///   4. Capabilities (if any)
    ///   5. Active journey state (if any)
    ///   6. Tool results (if any)
    ///   7. Matched guidelines
    ///   8. Canned responses (strict mode hint)
    ///   9. Recent conversation history
    ///   10. Instruction to respond
    ///
    /// Thin wrapper around [`build_prompt_with_stats`] retained for
    /// back-compat. Returns only the assembled prompt string.
    #[allow(clippy::too_many_arguments)]
    pub async fn build_prompt(
        &self,
        agent: &Agent,
        interaction: &Interaction,
        matched_guidelines: &[GuidelineMatch],
        glossary_terms: &[Term],
        context_variables: &[(ContextVariable, ContextVariableValue)],
        tool_results: &[ToolExecutionResult],
        journey_state: Option<&JourneyNode>,
        capabilities: &[Capability],
        canned_responses: &[CannedResponse],
    ) -> EngineResult<String> {
        Ok(self
            .build_prompt_with_stats(
                agent,
                interaction,
                matched_guidelines,
                glossary_terms,
                context_variables,
                tool_results,
                journey_state,
                capabilities,
                canned_responses,
            )
            .await?
            .prompt)
    }

    /// Like [`build_prompt`] but returns a [`PromptBuildResult`] with
    /// truncation stats. When the assembled prompt exceeds
    /// `max_tokens`, the oldest conversation messages are dropped one at
    /// a time until the budget is satisfied. If even an empty history
    /// is over budget, the history section is removed entirely.
    #[allow(clippy::too_many_arguments)]
    pub async fn build_prompt_with_stats(
        &self,
        agent: &Agent,
        interaction: &Interaction,
        matched_guidelines: &[GuidelineMatch],
        glossary_terms: &[Term],
        context_variables: &[(ContextVariable, ContextVariableValue)],
        tool_results: &[ToolExecutionResult],
        journey_state: Option<&JourneyNode>,
        capabilities: &[Capability],
        canned_responses: &[CannedResponse],
    ) -> EngineResult<PromptBuildResult> {
        let mut sections: Vec<String> = Vec::new();

        // 1. Identity
        sections.push(format!("You are {}: {}", agent.name, agent.description));

        // 2. Glossary
        if !glossary_terms.is_empty() {
            let mut s = String::from("Domain glossary:\n");
            for t in glossary_terms {
                s.push_str(&format!("  - {}: {}\n", t.name, t.description));
            }
            sections.push(s);
        }

        // 3. Context variables
        if !context_variables.is_empty() {
            let mut s = String::from("Known context:\n");
            for (var, val) in context_variables {
                s.push_str(&format!("  - {} = {}\n", var.key, val.data));
            }
            sections.push(s);
        }

        // 4. Capabilities
        if !capabilities.is_empty() {
            let mut s = String::from("You have these capabilities:\n");
            for c in capabilities {
                s.push_str(&format!("  - {}: {}\n", c.name, c.description));
            }
            sections.push(s);
        }

        // 5. Journey state
        if let Some(node) = journey_state {
            sections.push(format!(
                "Current journey step: {} — {}",
                node.action,
                node.description.as_deref().unwrap_or("")
            ));
        }

        // 6. Tool results
        if !tool_results.is_empty() {
            let mut s = String::from("Recent tool results:\n");
            for r in tool_results {
                s.push_str(&format!("  - {}: {}\n", r.tool_id.0, r.result.data));
            }
            sections.push(s);
        }

        // 7. Matched guidelines
        if !matched_guidelines.is_empty() {
            let mut s = String::from("Active guidelines (follow these):\n");
            for m in matched_guidelines {
                s.push_str(&format!(
                    "  - {} (confidence {:.2}): {}\n",
                    m.guideline.content.condition, m.confidence, m.guideline.content.action
                ));
            }
            sections.push(s);
        }

        // 8. Canned responses
        if !canned_responses.is_empty() {
            let mut s =
                String::from("Available canned responses (prefer these when applicable):\n");
            for cr in canned_responses {
                s.push_str(&format!("  - {}\n", cr.value));
            }
            sections.push(s);
        }

        // 9. Recent conversation (last up to 10 messages)
        let messages = interaction.messages();
        let history_section = |take: usize| -> Option<String> {
            if messages.is_empty() || take == 0 {
                return None;
            }
            let mut s = String::from("Recent conversation:\n");
            for m in messages.iter().rev().take(take).rev() {
                let speaker = match m.source {
                    loon_core::EventSource::Customer => "Customer",
                    loon_core::EventSource::AiAgent => "Agent",
                    loon_core::EventSource::System => "System",
                };
                s.push_str(&format!("  {}: {}\n", speaker, m.content));
            }
            Some(s)
        };
        let history_index = sections.len();
        let initial_take = messages.len().min(10);
        if let Some(s) = history_section(initial_take) {
            sections.push(s);
        }

        // 10. Instruction
        sections.push(
            "Now respond to the most recent customer message, following the guidelines above."
                .into(),
        );

        let mut prompt = sections.join("\n\n");

        // Token budget enforcement: if over max_tokens, progressively
        // drop the oldest messages from the conversation history
        // section. If even an empty history is over budget, remove
        // the history section entirely. We still emit the prompt —
        // a truncated prompt is preferable to refusing to respond.
        let mut tokens = self.tokenizer.count_tokens(&prompt).await.unwrap_or(0);
        let mut dropped_messages: usize = 0;
        let mut history_dropped_entirely = false;
        if (tokens as usize) > self.max_tokens && !messages.is_empty() {
            tracing::warn!(
                "prompt over token budget: {} > {}, truncating history",
                tokens,
                self.max_tokens
            );
            let mut take = initial_take;
            while (tokens as usize) > self.max_tokens && take > 0 {
                take -= 1;
                match history_section(take) {
                    Some(s) => sections[history_index] = s,
                    None => {
                        sections.remove(history_index);
                        history_dropped_entirely = true;
                        break;
                    }
                }
                prompt = sections.join("\n\n");
                tokens = self.tokenizer.count_tokens(&prompt).await.unwrap_or(0);
            }
            dropped_messages = initial_take.saturating_sub(take);
            if (tokens as usize) > self.max_tokens {
                tracing::warn!(
                    "prompt still over token budget after truncation: {} > {}",
                    tokens,
                    self.max_tokens
                );
            }
        }

        Ok(PromptBuildResult {
            prompt,
            dropped_messages,
            tokens_used: tokens,
            history_dropped_entirely,
        })
    }

    /// Build a prompt that asks an LLM to score which guidelines
    /// apply to the current interaction.
    pub async fn build_guideline_matching_prompt(
        &self,
        guidelines: &[loon_core::Guideline],
        interaction: &Interaction,
        glossary: &[Term],
    ) -> EngineResult<String> {
        let last_msg = interaction
            .last_customer_message()
            .map(|m| m.content)
            .unwrap_or_default();
        let mut s =
            String::from("Match relevant guidelines for the customer's most recent message.\n\n");
        if !glossary.is_empty() {
            s.push_str("Domain glossary:\n");
            for t in glossary {
                s.push_str(&format!("  - {}: {}\n", t.name, t.description));
            }
            s.push('\n');
        }
        s.push_str(&format!("Customer message: {}\n\n", last_msg));
        s.push_str("Available guidelines:\n");
        for g in guidelines {
            s.push_str(&format!(
                "  - id={}, condition='{}', action='{}'\n",
                g.id.0, g.content.condition, g.content.action
            ));
        }
        s.push_str(
            "\nReturn the matching guideline id with confidence (0.0-1.0) and a one-sentence rationale.",
        );
        Ok(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use loon_core::{
        Agent, AgentId, Event, EventId, EventKind, EventSource, Guideline, GuidelineContent, Term,
    };
    use loon_nlp::NlpResult;

    struct WsTok;
    #[async_trait]
    impl Tokenizer for WsTok {
        async fn count_tokens(&self, text: &str) -> NlpResult<u32> {
            Ok(text.split_whitespace().count() as u32)
        }
    }

    fn pb() -> PromptBuilder {
        PromptBuilder::new(Arc::new(WsTok), 8000)
    }

    fn agent() -> Agent {
        Agent::new("Helpdesk", "Helps users with billing")
    }

    fn mk_match(action: &str) -> GuidelineMatch {
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

    fn mk_message_event(source: EventSource, content: &str) -> Event {
        Event {
            id: EventId::new(),
            source,
            kind: EventKind::Message,
            trace_id: "trace".into(),
            data: serde_json::json!({ "message": content }),
            metadata: None,
            creation_utc: chrono::Utc::now(),
        }
    }

    #[tokio::test]
    async fn build_prompt_includes_action_text() {
        let p = pb()
            .build_prompt(
                &agent(),
                &Interaction::new(vec![]),
                &[mk_match("say hi back")],
                &[],
                &[],
                &[],
                None,
                &[],
                &[],
            )
            .await
            .unwrap();
        assert!(p.contains("say hi back"), "prompt missing action: {p}");
        assert!(p.contains("user greets"));
    }

    #[tokio::test]
    async fn prompt_includes_agent_identity() {
        let p = pb()
            .build_prompt(
                &agent(),
                &Interaction::new(vec![]),
                &[],
                &[],
                &[],
                &[],
                None,
                &[],
                &[],
            )
            .await
            .unwrap();
        assert!(p.contains("Helpdesk"), "prompt missing agent name: {p}");
        assert!(
            p.contains("Helps users with billing"),
            "prompt missing agent description: {p}"
        );
    }

    #[tokio::test]
    async fn prompt_includes_matched_guidelines() {
        let matches = vec![mk_match("respond politely"), mk_match("offer help")];
        let p = pb()
            .build_prompt(
                &agent(),
                &Interaction::new(vec![]),
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
        assert!(p.contains("Active guidelines"));
        assert!(p.contains("respond politely"));
        assert!(p.contains("offer help"));
        assert!(p.contains("confidence 0.95"));
    }

    #[tokio::test]
    async fn prompt_includes_glossary_when_present() {
        let term = Term::new("MRR", "monthly recurring revenue");
        let p = pb()
            .build_prompt(
                &agent(),
                &Interaction::new(vec![]),
                &[],
                &[term],
                &[],
                &[],
                None,
                &[],
                &[],
            )
            .await
            .unwrap();
        assert!(
            p.contains("Domain glossary"),
            "missing glossary header: {p}"
        );
        assert!(p.contains("MRR"), "missing term name: {p}");
        assert!(
            p.contains("monthly recurring revenue"),
            "missing term description: {p}"
        );
    }

    #[tokio::test]
    async fn prompt_truncates_when_over_budget() {
        // Small budget: WsTok counts whitespace-separated tokens, so
        // a 100-token budget is easily exceeded by ~20 messages.
        let pb = PromptBuilder::new(Arc::new(WsTok), 100);
        let mut events = Vec::new();
        for i in 0..20 {
            let src = if i % 2 == 0 {
                EventSource::Customer
            } else {
                EventSource::AiAgent
            };
            events.push(mk_message_event(
                src,
                "this is a fairly long message that contributes many tokens to the prompt",
            ));
        }
        let interaction = Interaction::new(events);
        let prompt = pb
            .build_prompt(&agent(), &interaction, &[], &[], &[], &[], None, &[], &[])
            .await
            .unwrap();
        // Either we dropped enough messages to fit the budget, or
        // the budget is unsatisfiable even with no history; in the
        // latter case the history section must be empty/absent.
        let tokens = prompt.split_whitespace().count();
        let has_history_header = prompt.contains("Recent conversation:");
        assert!(
            tokens <= 100 || !has_history_header,
            "expected truncation to keep tokens<=100 or drop history; got tokens={tokens}, has_history={has_history_header}, prompt=\n{prompt}"
        );
    }

    #[tokio::test]
    async fn prompt_stats_reports_dropped_count() {
        // Small budget: WsTok counts whitespace-separated tokens, so
        // a 50-token budget is easily exceeded by 10 long messages.
        let pb = PromptBuilder::new(Arc::new(WsTok), 50);
        let mut events = Vec::new();
        for i in 0..10 {
            events.push(mk_message_event(
                EventSource::Customer,
                &format!("long message number {} with many tokens", i),
            ));
        }
        let interaction = Interaction::new(events);
        let result = pb
            .build_prompt_with_stats(
                &agent(),
                &interaction,
                &[],
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
            result.dropped_messages > 0,
            "expected at least one message dropped, got dropped={}, tokens_used={}, prompt=\n{}",
            result.dropped_messages,
            result.tokens_used,
            result.prompt
        );
        // tokens_used should be reported and should be at or under the budget
        // (or history was dropped entirely, in which case the budget is
        // unsatisfiable with the current setup but truncation was attempted).
        assert!(
            (result.tokens_used as usize) <= 50 || result.history_dropped_entirely,
            "expected tokens_used<=50 or history_dropped_entirely=true, got tokens_used={}, history_dropped_entirely={}, prompt=\n{}",
            result.tokens_used,
            result.history_dropped_entirely,
            result.prompt
        );
    }

    #[tokio::test]
    async fn prompt_stats_reports_zero_dropped_when_under_budget() {
        // Generous budget: nothing should be dropped.
        let pb = PromptBuilder::new(Arc::new(WsTok), 8000);
        let interaction = Interaction::new(vec![mk_message_event(
            EventSource::Customer,
            "short",
        )]);
        let result = pb
            .build_prompt_with_stats(
                &agent(),
                &interaction,
                &[],
                &[],
                &[],
                &[],
                None,
                &[],
                &[],
            )
            .await
            .unwrap();
        assert_eq!(result.dropped_messages, 0);
        assert!(!result.history_dropped_entirely);
        assert!(result.tokens_used > 0);
    }

    #[tokio::test]
    async fn prompt_includes_recent_messages() {
        let interaction = Interaction::new(vec![
            mk_message_event(EventSource::Customer, "I need help with billing"),
            mk_message_event(EventSource::AiAgent, "Sure, I can help"),
        ]);
        let p = pb()
            .build_prompt(&agent(), &interaction, &[], &[], &[], &[], None, &[], &[])
            .await
            .unwrap();
        assert!(
            p.contains("Recent conversation"),
            "missing conversation header: {p}"
        );
        assert!(
            p.contains("Customer: I need help with billing"),
            "missing customer message: {p}"
        );
        assert!(
            p.contains("Agent: Sure, I can help"),
            "missing agent message: {p}"
        );
    }
}

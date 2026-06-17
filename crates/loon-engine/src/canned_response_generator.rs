//! `CannedResponseGenerator` — picks a canned response for a draft
//! message and fills its template fields.

use std::collections::HashMap;
use std::sync::Arc;

use loon_core::{Agent, CannedResponse};
use loon_nlp::{define_schematic, NlpService, Schematic};

use crate::engine_context::Interaction;
use crate::error::{EngineError, EngineResult};

define_schematic! {
    pub struct CannedResponseSelectionResult {
        pub canned_response_index: i32,
        pub score: f32,
        pub fields_json: String,
    }
}

/// Type-erased selection result (the matched canned response, its
/// confidence score, and the field-fill values used during template
/// expansion).
pub struct CannedResponseSelectionOut {
    pub canned_response: CannedResponse,
    pub score: f32,
    pub filled_fields: HashMap<String, String>,
}

/// Selects the best matching canned response for a given draft and
/// interaction.
pub struct CannedResponseGenerator {
    pub nlp: Arc<dyn NlpService>,
}

impl CannedResponseGenerator {
    pub fn new(nlp: Arc<dyn NlpService>) -> Self {
        Self { nlp }
    }
}

impl CannedResponseGenerator {
    /// Select the best canned response for the current interaction.
    /// Uses an LLM via `NlpService::schematic_generator` to score
    /// candidates and extract template field values.
    ///
    /// Returns `None` if `canned_responses` is empty, if the LLM
    /// returns an out-of-range index, or if the LLM signals no fit
    /// with `canned_response_index = -1`.
    pub async fn select_best(
        &self,
        canned_responses: &[CannedResponse],
        draft_message: &str,
        agent: &Agent,
        interaction: &Interaction,
    ) -> EngineResult<Option<CannedResponseSelectionOut>> {
        if canned_responses.is_empty() {
            return Ok(None);
        }

        // Build the LLM prompt
        let last_msg = interaction
            .last_customer_message()
            .map(|m| m.content)
            .unwrap_or_default();
        let mut prompt = format!(
            "You are {agent_name}. The customer said: \"{last_msg}\".\n\
            Your draft response is: \"{draft}\".\n\n\
            Select the best canned response from the list below by index (0-based).\n\
            Return:\n\
              - canned_response_index: the index of the best match (or -1 if none fits)\n\
              - score: confidence in the match (0.0-1.0)\n\
              - fields_json: JSON string of {{field_name: value}} pairs to fill any \
                             {{field}} placeholders in the chosen response (or \"{{}}\" if none).\n\n\
            Candidates:\n",
            agent_name = agent.name,
            last_msg = last_msg,
            draft = draft_message,
        );
        for (i, cr) in canned_responses.iter().enumerate() {
            prompt.push_str(&format!("  [{}]: {}\n", i, cr.value));
        }

        let gen = self
            .nlp
            .schematic_generator(CannedResponseSelectionResult::schema())
            .await
            .map_err(|e| EngineError::MessageGenerationFailed(e.to_string()))?;
        let result = gen
            .generate(prompt, Default::default())
            .await
            .map_err(|e| EngineError::MessageGenerationFailed(e.to_string()))?;

        let parsed: CannedResponseSelectionResult = serde_json::from_value(result.value)
            .unwrap_or_default();

        let idx = parsed.canned_response_index;
        if idx < 0 || (idx as usize) >= canned_responses.len() {
            return Ok(None);
        }

        let cr = canned_responses[idx as usize].clone();
        let filled_fields: HashMap<String, String> =
            serde_json::from_str(&parsed.fields_json).unwrap_or_default();

        Ok(Some(CannedResponseSelectionOut {
            canned_response: cr,
            score: parsed.score,
            filled_fields,
        }))
    }

    /// Replace every `{field_name}` placeholder in `template` with
    /// the corresponding value in `fields`. Unknown placeholders
    /// are left intact.
    pub fn fill_template(template: &str, fields: &HashMap<String, String>) -> String {
        let mut result = template.to_string();
        for (k, v) in fields {
            result = result.replace(&format!("{{{}}}", k), v);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use loon_core::Agent;
    use loon_nlp::test_utils::FakeNlpService;

    #[tokio::test]
    async fn select_best_returns_none_when_empty() {
        let gen = CannedResponseGenerator::new(Arc::new(FakeNlpService::new()));
        let agent = Agent::new("test", "x");
        let result = gen
            .select_best(&[], "draft", &agent, &Interaction::new(vec![]))
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn fill_template_substitutes_placeholders() {
        let mut fields = HashMap::new();
        fields.insert("city".into(), "NYC".into());
        let out = CannedResponseGenerator::fill_template("Hello from {city}!", &fields);
        assert_eq!(out, "Hello from NYC!");
    }

    #[test]
    fn fill_template_leaves_unknown_placeholders() {
        let fields = HashMap::new();
        let out = CannedResponseGenerator::fill_template("Hello {unknown}!", &fields);
        assert_eq!(out, "Hello {unknown}!");
    }

    #[test]
    fn fill_template_substitutes_known_fields() {
        let mut fields = HashMap::new();
        fields.insert("city".to_string(), "NYC".to_string());
        let out = CannedResponseGenerator::fill_template("Hello {city}!", &fields);
        assert_eq!(out, "Hello NYC!");
    }
}

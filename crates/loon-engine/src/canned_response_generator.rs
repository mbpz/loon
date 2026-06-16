//! `CannedResponseGenerator` — picks a canned response for a draft
//! message and fills its template fields.

use std::collections::HashMap;
use std::sync::Arc;

use loon_core::{Agent, CannedResponse};
use loon_nlp::NlpService;

use crate::engine_context::Interaction;
use crate::error::EngineResult;

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
    /// Pick the best canned response. Phase 1 just takes the first
    /// available entry; a future version will use
    /// `NlpService::schematic_generator` to score each one against
    /// the draft message and interaction.
    pub async fn select_best(
        &self,
        canned_responses: &[CannedResponse],
        _draft_message: &str,
        _agent: &Agent,
        _interaction: &Interaction,
    ) -> EngineResult<Option<CannedResponseSelectionOut>> {
        if canned_responses.is_empty() {
            return Ok(None);
        }
        let cr = canned_responses[0].clone();
        Ok(Some(CannedResponseSelectionOut {
            canned_response: cr,
            score: 1.0,
            filled_fields: HashMap::new(),
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

    #[test]
    fn fill_template_substitutes_known_fields() {
        let mut fields = HashMap::new();
        fields.insert("city".to_string(), "NYC".to_string());
        let out = CannedResponseGenerator::fill_template("Hello {city}!", &fields);
        assert_eq!(out, "Hello NYC!");
    }

    #[test]
    fn fill_template_leaves_unknown_placeholders() {
        let fields: HashMap<String, String> = HashMap::new();
        let out = CannedResponseGenerator::fill_template("Hi {name}", &fields);
        assert_eq!(out, "Hi {name}");
    }
}

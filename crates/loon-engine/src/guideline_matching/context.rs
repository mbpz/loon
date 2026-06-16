//! Input context for guideline matching.

use std::sync::Arc;

use loon_core::{Agent, Guideline, Session, Term};
use loon_nlp::NlpService;

use crate::engine_context::Interaction;

pub struct GuidelineMatchingContext {
    pub agent: Agent,
    pub session: Session,
    pub interaction: Interaction,
    pub guidelines: Vec<Guideline>,
    pub glossary_terms: Vec<Term>,
    pub nlp: Arc<dyn NlpService>,
}

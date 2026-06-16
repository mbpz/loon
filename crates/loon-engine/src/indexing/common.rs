//! Shared types for indexing traits.

use loon_core::Guideline;

use crate::error::EngineResult;

/// Result of evaluating a guideline's expected behavioral impact.
pub struct BehavioralChangeEvaluation {
    pub guideline: Guideline,
    pub estimated_impact: f32,
}

/// Output of a `GuidelineActionProposer`: candidate guidelines that
/// could be associated with an action.
pub struct GuidelineActionProposerOutput {
    pub candidates: Vec<Guideline>,
}

// Re-export EngineResult for downstream modules that want a single import.
pub type IndexingResult<T> = EngineResult<T>;

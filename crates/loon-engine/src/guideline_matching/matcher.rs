//! Core `GuidelineMatcher` trait.

use async_trait::async_trait;

use crate::engine_context::GuidelineMatch;
use crate::error::EngineResult;
use crate::guideline_matching::context::GuidelineMatchingContext;

/// Implementations pick which guidelines apply to the current
/// interaction.
#[async_trait]
pub trait GuidelineMatcher: Send + Sync {
    async fn match_guidelines(
        &self,
        ctx: &GuidelineMatchingContext,
    ) -> EngineResult<Vec<GuidelineMatch>>;
}

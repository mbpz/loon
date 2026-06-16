//! User-defined per-tag guideline matching strategies.

use async_trait::async_trait;

use loon_core::Guideline;

use crate::engine_context::GuidelineMatch;
use crate::error::EngineResult;
use crate::guideline_matching::context::GuidelineMatchingContext;

/// Caller-supplied strategy that decides matches for a subset of
/// guidelines (typically those tagged with a particular TagId).
#[async_trait]
pub trait CustomGuidelineMatchingStrategy: Send + Sync {
    async fn match_guidelines(
        &self,
        guidelines: &[Guideline],
        ctx: &GuidelineMatchingContext,
    ) -> EngineResult<Vec<GuidelineMatch>>;
}

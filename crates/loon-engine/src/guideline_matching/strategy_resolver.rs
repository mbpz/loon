//! Resolve a `CustomGuidelineMatchingStrategy` for a given guideline.

use std::collections::HashMap;
use std::sync::Arc;

use loon_core::{Guideline, TagId};

use crate::guideline_matching::custom_strategy::CustomGuidelineMatchingStrategy;

/// Maps tags to user-supplied matching strategies.
#[derive(Default)]
pub struct GenericGuidelineMatchingStrategyResolver {
    pub strategies: HashMap<TagId, Arc<dyn CustomGuidelineMatchingStrategy>>,
}

impl GenericGuidelineMatchingStrategyResolver {
    pub fn strategy_for(
        &self,
        _guideline: &Guideline,
    ) -> Option<Arc<dyn CustomGuidelineMatchingStrategy>> {
        // Phase 1: no overrides; relationship traversal deferred.
        None
    }
}

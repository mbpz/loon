//! Resolves guideline relationships: exclusions and dependencies.

use std::sync::Arc;

use loon_core::stores::RelationshipStore;

use crate::engine_context::GuidelineMatch;
use crate::error::EngineResult;

pub struct RelationalResolver {
    pub relationship_store: Arc<dyn RelationshipStore>,
}

impl RelationalResolver {
    pub fn new(store: Arc<dyn RelationshipStore>) -> Self {
        Self {
            relationship_store: store,
        }
    }

    /// Phase 1: identity — relational traversal deferred.
    pub async fn resolve_exclusions(
        &self,
        matches: Vec<GuidelineMatch>,
    ) -> EngineResult<Vec<GuidelineMatch>> {
        Ok(matches)
    }

    pub async fn resolve_dependencies(
        &self,
        matches: Vec<GuidelineMatch>,
        _all_guidelines: &[loon_core::Guideline],
    ) -> EngineResult<Vec<GuidelineMatch>> {
        Ok(matches)
    }

    pub async fn resolve(
        &self,
        matches: Vec<GuidelineMatch>,
        all: &[loon_core::Guideline],
    ) -> EngineResult<Vec<GuidelineMatch>> {
        let after_excl = self.resolve_exclusions(matches).await?;
        self.resolve_dependencies(after_excl, all).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use loon_core::{
        AgentId, CoreError, CoreResult, Guideline, GuidelineContent, GuidelineId, Criticality,
        JsonValue, Relationship, RelationshipEntity, RelationshipEntityKind, RelationshipId,
        RelationshipKind, TagId,
    };
    use std::sync::Arc;

    struct StubRelStore;
    #[async_trait]
    impl RelationshipStore for StubRelStore {
        async fn create(&self, _r: Relationship) -> CoreResult<Relationship> {
            Err(CoreError::Internal("not used".into()))
        }
        async fn read(&self, _id: &RelationshipId) -> CoreResult<Option<Relationship>> {
            Ok(None)
        }
        async fn delete(&self, _id: &RelationshipId) -> CoreResult<()> {
            Ok(())
        }
        async fn list_for(
            &self,
            _e: &RelationshipEntity,
        ) -> CoreResult<Vec<Relationship>> {
            Ok(vec![])
        }
    }

    fn make_match() -> GuidelineMatch {
        let g = Guideline {
            id: GuidelineId::new(),
            agent_id: AgentId::new(),
            content: GuidelineContent {
                condition: "c".into(),
                action: "a".into(),
                description: None,
            },
            criticality: Criticality::Low,
            enabled: true,
            tags: vec![],
            creation_utc: chrono::Utc::now(),
            metadata: JsonValue::Null,
        };
        GuidelineMatch {
            guideline: g,
            confidence: 1.0,
            rationale: "r".into(),
        }
    }

    #[tokio::test]
    async fn resolve_exclusions_is_identity() {
        let resolver = RelationalResolver::new(Arc::new(StubRelStore));
        let input = vec![make_match(), make_match()];
        let out = resolver.resolve_exclusions(input.clone()).await.unwrap();
        assert_eq!(out.len(), input.len());

        let _ = TagId::new(); // exercise the import path
        let _ = RelationshipKind::Excludes;
        let _ = RelationshipEntityKind::Guideline;
    }
}

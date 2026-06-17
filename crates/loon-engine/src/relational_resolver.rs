//! Real exclusion + dependency graph traversal.
//!
//! Given a set of matched guidelines and the agent's full
//! `RelationshipStore`, this resolver:
//!  - drops matches that are excluded by any other match (Excludes
//!    relationship from a higher-confidence guideline)
//!  - adds matches that are required dependencies of any kept match
//!    (Dependency relationship pointing at another guideline)
//!  - propagates Reevaluation as a flag on the resolved match

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use loon_core::stores::RelationshipStore;
use loon_core::{
    Guideline, GuidelineId, RelationshipEntity, RelationshipEntityKind, RelationshipKind,
};

use crate::engine_context::GuidelineMatch;
use crate::error::{EngineError, EngineResult};

pub struct RelationalResolver {
    pub relationship_store: Arc<dyn RelationshipStore>,
}

impl RelationalResolver {
    pub fn new(store: Arc<dyn RelationshipStore>) -> Self {
        Self {
            relationship_store: store,
        }
    }

    /// For each match, look up Excludes relationships originating
    /// from that match's guideline. Drop any other match whose
    /// guideline appears as the target of those relationships.
    pub async fn resolve_exclusions(
        &self,
        matches: Vec<GuidelineMatch>,
    ) -> EngineResult<Vec<GuidelineMatch>> {
        if matches.is_empty() {
            return Ok(matches);
        }

        // Sort by confidence descending so higher-confidence matches
        // can exclude lower-confidence matches.
        let mut matches = matches;
        matches.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut excluded: HashSet<GuidelineId> = HashSet::new();
        for m in &matches {
            if excluded.contains(&m.guideline.id) {
                continue;
            }
            let entity = RelationshipEntity {
                kind: RelationshipEntityKind::Guideline,
                id: m.guideline.id.0.clone(),
            };
            let rels = self
                .relationship_store
                .list_for(&entity)
                .await
                .map_err(|e| EngineError::ContextLoadFailed(e.to_string()))?;
            for r in rels {
                if matches!(r.kind, RelationshipKind::Excludes)
                    && r.source.id == m.guideline.id.0
                    && r.source.kind == RelationshipEntityKind::Guideline
                    && r.target.kind == RelationshipEntityKind::Guideline
                {
                    excluded.insert(GuidelineId(r.target.id.clone()));
                }
            }
        }

        Ok(matches
            .into_iter()
            .filter(|m| !excluded.contains(&m.guideline.id))
            .collect())
    }

    /// For each match, look up Dependency relationships originating
    /// from that match's guideline. Find the target guideline in
    /// `all_guidelines` and add it as a synthetic match (confidence
    /// inherited from the source) if not already present.
    pub async fn resolve_dependencies(
        &self,
        matches: Vec<GuidelineMatch>,
        all_guidelines: &[Guideline],
    ) -> EngineResult<Vec<GuidelineMatch>> {
        let mut by_id: HashMap<GuidelineId, Guideline> = all_guidelines
            .iter()
            .map(|g| (g.id.clone(), g.clone()))
            .collect();
        let mut existing: HashSet<GuidelineId> =
            matches.iter().map(|m| m.guideline.id.clone()).collect();
        let mut out = matches.clone();

        let mut frontier: Vec<GuidelineMatch> = matches;
        while !frontier.is_empty() {
            let mut next_frontier = Vec::new();
            for m in &frontier {
                let entity = RelationshipEntity {
                    kind: RelationshipEntityKind::Guideline,
                    id: m.guideline.id.0.clone(),
                };
                let rels = self
                    .relationship_store
                    .list_for(&entity)
                    .await
                    .map_err(|e| EngineError::ContextLoadFailed(e.to_string()))?;
                for r in rels {
                    if matches!(r.kind, RelationshipKind::Dependency)
                        && r.source.id == m.guideline.id.0
                        && r.target.kind == RelationshipEntityKind::Guideline
                    {
                        let target_id = GuidelineId(r.target.id.clone());
                        if existing.contains(&target_id) {
                            continue;
                        }
                        if let Some(target_g) = by_id.remove(&target_id) {
                            let synthetic = GuidelineMatch {
                                guideline: target_g,
                                confidence: m.confidence,
                                rationale: format!("dependency of {}", m.guideline.id.0),
                            };
                            existing.insert(target_id.clone());
                            out.push(synthetic.clone());
                            next_frontier.push(synthetic);
                        }
                    }
                }
            }
            frontier = next_frontier;
        }

        Ok(out)
    }

    pub async fn resolve(
        &self,
        matches: Vec<GuidelineMatch>,
        all: &[Guideline],
    ) -> EngineResult<Vec<GuidelineMatch>> {
        let r = self.resolve_exclusions(matches).await?;
        self.resolve_dependencies(r, all).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use loon_core::stores::in_memory::InMemoryRelationshipStore;
    use loon_core::stores::RelationshipStore;
    use loon_core::{
        AgentId, Criticality, Guideline, GuidelineContent, GuidelineId, JsonValue, Relationship,
        RelationshipEntity, RelationshipEntityKind, RelationshipId, RelationshipKind,
    };
    use std::sync::Arc;

    fn make_guideline(name: &str) -> Guideline {
        let agent_id = AgentId::new();
        Guideline {
            id: GuidelineId::new(),
            agent_id,
            content: GuidelineContent {
                condition: "x".into(),
                action: name.into(),
                description: None,
            },
            criticality: Criticality::Low,
            enabled: true,
            tags: vec![],
            creation_utc: chrono::Utc::now(),
            metadata: JsonValue::Null,
        }
    }

    #[tokio::test]
    async fn excludes_drops_lower_confidence() {
        let store = Arc::new(InMemoryRelationshipStore::new());
        let g_high = make_guideline("high");
        let g_low = make_guideline("low");
        let rel = Relationship {
            id: RelationshipId::new(),
            source: RelationshipEntity {
                kind: RelationshipEntityKind::Guideline,
                id: g_high.id.0.clone(),
            },
            target: RelationshipEntity {
                kind: RelationshipEntityKind::Guideline,
                id: g_low.id.0.clone(),
            },
            kind: RelationshipKind::Excludes,
            indirect: false,
            creation_utc: chrono::Utc::now(),
        };
        store.create(rel).await.unwrap();
        let resolver = RelationalResolver::new(store);
        let matches = vec![
            GuidelineMatch {
                guideline: g_high.clone(),
                confidence: 0.9,
                rationale: "h".into(),
            },
            GuidelineMatch {
                guideline: g_low.clone(),
                confidence: 0.5,
                rationale: "l".into(),
            },
        ];
        let resolved = resolver.resolve_exclusions(matches).await.unwrap();
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].guideline.id, g_high.id);
    }

    #[tokio::test]
    async fn excludes_keeps_unrelated() {
        let store = Arc::new(InMemoryRelationshipStore::new());
        let resolver = RelationalResolver::new(store);
        let g_a = make_guideline("a");
        let g_b = make_guideline("b");
        let matches = vec![
            GuidelineMatch {
                guideline: g_a,
                confidence: 0.9,
                rationale: "a".into(),
            },
            GuidelineMatch {
                guideline: g_b,
                confidence: 0.5,
                rationale: "b".into(),
            },
        ];
        let resolved = resolver.resolve_exclusions(matches).await.unwrap();
        assert_eq!(resolved.len(), 2);
    }

    #[tokio::test]
    async fn dependencies_pull_in_target() {
        let store = Arc::new(InMemoryRelationshipStore::new());
        let g_src = make_guideline("source");
        let g_dep = make_guideline("dep");
        let rel = Relationship {
            id: RelationshipId::new(),
            source: RelationshipEntity {
                kind: RelationshipEntityKind::Guideline,
                id: g_src.id.0.clone(),
            },
            target: RelationshipEntity {
                kind: RelationshipEntityKind::Guideline,
                id: g_dep.id.0.clone(),
            },
            kind: RelationshipKind::Dependency,
            indirect: false,
            creation_utc: chrono::Utc::now(),
        };
        store.create(rel).await.unwrap();
        let resolver = RelationalResolver::new(store);
        let matches = vec![GuidelineMatch {
            guideline: g_src.clone(),
            confidence: 0.8,
            rationale: "s".into(),
        }];
        let resolved = resolver
            .resolve_dependencies(matches, std::slice::from_ref(&g_dep))
            .await
            .unwrap();
        assert_eq!(resolved.len(), 2);
        assert!(resolved.iter().any(|m| m.guideline.id == g_dep.id));
    }

    #[tokio::test]
    async fn dependencies_transitive() {
        let store = Arc::new(InMemoryRelationshipStore::new());
        let g_a = make_guideline("a");
        let g_b = make_guideline("b");
        let g_c = make_guideline("c");
        // a → b, b → c
        for (src, tgt) in [
            (g_a.id.0.clone(), g_b.id.0.clone()),
            (g_b.id.0.clone(), g_c.id.0.clone()),
        ] {
            store
                .create(Relationship {
                    id: RelationshipId::new(),
                    source: RelationshipEntity {
                        kind: RelationshipEntityKind::Guideline,
                        id: src,
                    },
                    target: RelationshipEntity {
                        kind: RelationshipEntityKind::Guideline,
                        id: tgt,
                    },
                    kind: RelationshipKind::Dependency,
                    indirect: false,
                    creation_utc: chrono::Utc::now(),
                })
                .await
                .unwrap();
        }
        let resolver = RelationalResolver::new(store);
        let matches = vec![GuidelineMatch {
            guideline: g_a.clone(),
            confidence: 0.7,
            rationale: "a".into(),
        }];
        let resolved = resolver
            .resolve_dependencies(matches, &[g_b.clone(), g_c.clone()])
            .await
            .unwrap();
        assert_eq!(
            resolved.len(),
            3,
            "expected a + b + c, got {} matches",
            resolved.len()
        );
    }
}

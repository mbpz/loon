//! Application-level wrapper around `RelationshipStore`.

use std::sync::Arc;

use loon_core::stores::RelationshipStore;
use loon_core::{
    CoreResult, Relationship, RelationshipEntity, RelationshipEntityKind, RelationshipId,
    RelationshipKind,
};

#[derive(Debug, Clone)]
pub struct RelationshipCreateParams {
    pub source: RelationshipEntity,
    pub target: RelationshipEntity,
    pub kind: RelationshipKind,
}

pub struct RelationshipAppModule {
    pub store: Arc<dyn RelationshipStore>,
}

impl RelationshipAppModule {
    pub fn new(store: Arc<dyn RelationshipStore>) -> Self {
        Self { store }
    }

    pub async fn create_relationship(
        &self,
        params: RelationshipCreateParams,
    ) -> CoreResult<Relationship> {
        let r = Relationship::new(params.source, params.target, params.kind);
        self.store.create(r).await
    }

    pub async fn read_relationship(
        &self,
        id: &RelationshipId,
    ) -> CoreResult<Option<Relationship>> {
        self.store.read(id).await
    }

    pub async fn delete_relationship(&self, id: &RelationshipId) -> CoreResult<()> {
        self.store.delete(id).await
    }

    pub async fn find_relationships_for(
        &self,
        entity: &RelationshipEntity,
    ) -> CoreResult<Vec<Relationship>> {
        self.store.list_for(entity).await
    }
}

// Convenience constructors kept on the entity side are mirrored here for
// callers that don't want to thread the kind enum through themselves.
pub fn guideline_entity(id: impl Into<String>) -> RelationshipEntity {
    RelationshipEntity {
        kind: RelationshipEntityKind::Guideline,
        id: id.into(),
    }
}

pub fn tag_entity(id: impl Into<String>) -> RelationshipEntity {
    RelationshipEntity {
        kind: RelationshipEntityKind::Tag,
        id: id.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use parking_lot::Mutex;
    use std::collections::HashMap;

    pub struct FakeRelationshipStore {
        pub data: Mutex<HashMap<RelationshipId, Relationship>>,
    }
    impl FakeRelationshipStore {
        pub fn new() -> Self {
            Self {
                data: Mutex::new(HashMap::new()),
            }
        }
    }

    #[async_trait]
    impl RelationshipStore for FakeRelationshipStore {
        async fn create(&self, r: Relationship) -> CoreResult<Relationship> {
            let id = r.id.clone();
            self.data.lock().insert(id, r.clone());
            Ok(r)
        }
        async fn read(&self, id: &RelationshipId) -> CoreResult<Option<Relationship>> {
            Ok(self.data.lock().get(id).cloned())
        }
        async fn delete(&self, id: &RelationshipId) -> CoreResult<()> {
            self.data.lock().remove(id);
            Ok(())
        }
        async fn list_for(&self, entity: &RelationshipEntity) -> CoreResult<Vec<Relationship>> {
            Ok(self
                .data
                .lock()
                .values()
                .filter(|r| r.source == *entity || r.target == *entity)
                .cloned()
                .collect())
        }
    }

    #[tokio::test]
    async fn relationship_create_and_find() {
        let store: Arc<dyn RelationshipStore> = Arc::new(FakeRelationshipStore::new());
        let module = RelationshipAppModule::new(store);
        let g = guideline_entity("g1");
        let t = tag_entity("t1");
        let _r = module
            .create_relationship(RelationshipCreateParams {
                source: g.clone(),
                target: t.clone(),
                kind: RelationshipKind::Dependency,
            })
            .await
            .unwrap();
        let _r2 = module
            .create_relationship(RelationshipCreateParams {
                source: guideline_entity("g2"),
                target: tag_entity("t2"),
                kind: RelationshipKind::Excludes,
            })
            .await
            .unwrap();
        let found = module.find_relationships_for(&g).await.unwrap();
        assert_eq!(found.len(), 1);
    }
}

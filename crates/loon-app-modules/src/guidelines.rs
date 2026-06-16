//! Application-level wrapper around `GuidelineStore` plus the
//! guideline-to-tag and guideline-to-tool relationship helpers.

use std::sync::Arc;

use loon_core::stores::{GuidelineStore, GuidelineToolAssociationStore, RelationshipStore};
use loon_core::{
    AgentId, CoreResult, Guideline, GuidelineContent, GuidelineId, GuidelineToolAssociation,
    GuidelineUpdateParams, Relationship, RelationshipEntity, RelationshipEntityKind,
    RelationshipKind, TagId, ToolId,
};

#[derive(Debug, Clone)]
pub struct GuidelineCreateParams {
    pub agent_id: AgentId,
    pub content: GuidelineContent,
    pub enabled: bool,
    pub criticality: i32,
}

pub struct GuidelineAppModule {
    pub store: Arc<dyn GuidelineStore>,
    pub relationships: Arc<dyn RelationshipStore>,
    pub tool_associations: Arc<dyn GuidelineToolAssociationStore>,
}

impl GuidelineAppModule {
    pub fn new(
        store: Arc<dyn GuidelineStore>,
        relationships: Arc<dyn RelationshipStore>,
        tool_associations: Arc<dyn GuidelineToolAssociationStore>,
    ) -> Self {
        Self {
            store,
            relationships,
            tool_associations,
        }
    }

    pub async fn create_guideline(&self, params: GuidelineCreateParams) -> CoreResult<Guideline> {
        let g = Guideline::new(params.content, &params.agent_id, params.enabled, params.criticality);
        self.store.create(g).await
    }

    pub async fn read_guideline(&self, id: &GuidelineId) -> CoreResult<Option<Guideline>> {
        self.store.read(id).await
    }

    pub async fn update_guideline(
        &self,
        id: &GuidelineId,
        params: GuidelineUpdateParams,
    ) -> CoreResult<Guideline> {
        self.store.update(id, params).await
    }

    pub async fn delete_guideline(&self, id: &GuidelineId) -> CoreResult<()> {
        self.store.delete(id).await
    }

    pub async fn list_guidelines(
        &self,
        agent_id: &AgentId,
        tags: &[TagId],
    ) -> CoreResult<Vec<Guideline>> {
        self.store.list(agent_id, tags).await
    }

    pub async fn add_dependency(
        &self,
        guideline_id: &GuidelineId,
        target_tag_id: &TagId,
    ) -> CoreResult<Relationship> {
        let source = RelationshipEntity {
            kind: RelationshipEntityKind::Guideline,
            id: guideline_id.0.clone(),
        };
        let target = RelationshipEntity {
            kind: RelationshipEntityKind::Tag,
            id: target_tag_id.0.clone(),
        };
        let rel = Relationship::new(source, target, RelationshipKind::Dependency);
        self.relationships.create(rel).await
    }

    pub async fn exclude(
        &self,
        guideline_id: &GuidelineId,
        excluded_tag_id: &TagId,
    ) -> CoreResult<Relationship> {
        let source = RelationshipEntity {
            kind: RelationshipEntityKind::Guideline,
            id: guideline_id.0.clone(),
        };
        let target = RelationshipEntity {
            kind: RelationshipEntityKind::Tag,
            id: excluded_tag_id.0.clone(),
        };
        let rel = Relationship::new(source, target, RelationshipKind::Excludes);
        self.relationships.create(rel).await
    }

    pub async fn associate_tool(
        &self,
        guideline_id: &GuidelineId,
        tool_id: &ToolId,
    ) -> CoreResult<GuidelineToolAssociation> {
        let assoc = GuidelineToolAssociation::new(guideline_id, tool_id);
        self.tool_associations.create(assoc).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use parking_lot::Mutex;
    use std::collections::HashMap;

    pub struct FakeGuidelineStore {
        pub data: Mutex<HashMap<GuidelineId, Guideline>>,
    }
    impl FakeGuidelineStore {
        pub fn new() -> Self {
            Self {
                data: Mutex::new(HashMap::new()),
            }
        }
    }

    #[async_trait]
    impl GuidelineStore for FakeGuidelineStore {
        async fn create(&self, g: Guideline) -> CoreResult<Guideline> {
            let id = g.id.clone();
            self.data.lock().insert(id, g.clone());
            Ok(g)
        }
        async fn read(&self, id: &GuidelineId) -> CoreResult<Option<Guideline>> {
            Ok(self.data.lock().get(id).cloned())
        }
        async fn update(
            &self,
            id: &GuidelineId,
            p: GuidelineUpdateParams,
        ) -> CoreResult<Guideline> {
            let mut g = self.data.lock();
            let gv = g.get_mut(id).unwrap();
            if let Some(c) = p.condition {
                gv.content.condition = c;
            }
            if let Some(a) = p.action {
                gv.content.action = a;
            }
            if let Some(e) = p.enabled {
                gv.enabled = e;
            }
            Ok(gv.clone())
        }
        async fn delete(&self, id: &GuidelineId) -> CoreResult<()> {
            self.data.lock().remove(id);
            Ok(())
        }
        async fn list(
            &self,
            _agent_id: &AgentId,
            _tags: &[TagId],
        ) -> CoreResult<Vec<Guideline>> {
            Ok(self.data.lock().values().cloned().collect())
        }
    }

    pub struct FakeRelationshipStore {
        pub data: Mutex<HashMap<loon_core::RelationshipId, Relationship>>,
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
        async fn read(
            &self,
            id: &loon_core::RelationshipId,
        ) -> CoreResult<Option<Relationship>> {
            Ok(self.data.lock().get(id).cloned())
        }
        async fn delete(&self, id: &loon_core::RelationshipId) -> CoreResult<()> {
            self.data.lock().remove(id);
            Ok(())
        }
        async fn list_for(
            &self,
            entity: &RelationshipEntity,
        ) -> CoreResult<Vec<Relationship>> {
            Ok(self
                .data
                .lock()
                .values()
                .filter(|r| r.source == *entity || r.target == *entity)
                .cloned()
                .collect())
        }
    }

    pub struct FakeGuidelineToolAssociationStore {
        pub data: Mutex<HashMap<loon_core::GuidelineToolAssociationId, GuidelineToolAssociation>>,
    }
    impl FakeGuidelineToolAssociationStore {
        pub fn new() -> Self {
            Self {
                data: Mutex::new(HashMap::new()),
            }
        }
    }

    #[async_trait]
    impl GuidelineToolAssociationStore for FakeGuidelineToolAssociationStore {
        async fn create(
            &self,
            a: GuidelineToolAssociation,
        ) -> CoreResult<GuidelineToolAssociation> {
            let id = a.id.clone();
            self.data.lock().insert(id, a.clone());
            Ok(a)
        }
        async fn read(
            &self,
            id: &loon_core::GuidelineToolAssociationId,
        ) -> CoreResult<Option<GuidelineToolAssociation>> {
            Ok(self.data.lock().get(id).cloned())
        }
        async fn delete(
            &self,
            id: &loon_core::GuidelineToolAssociationId,
        ) -> CoreResult<()> {
            self.data.lock().remove(id);
            Ok(())
        }
        async fn list_for_tool(
            &self,
            _tool_id: &ToolId,
        ) -> CoreResult<Vec<GuidelineToolAssociation>> {
            Ok(self.data.lock().values().cloned().collect())
        }
        async fn list_for_guideline(
            &self,
            _guideline_id: &GuidelineId,
        ) -> CoreResult<Vec<GuidelineToolAssociation>> {
            Ok(self.data.lock().values().cloned().collect())
        }
    }

    fn module() -> GuidelineAppModule {
        GuidelineAppModule::new(
            Arc::new(FakeGuidelineStore::new()),
            Arc::new(FakeRelationshipStore::new()),
            Arc::new(FakeGuidelineToolAssociationStore::new()),
        )
    }

    #[tokio::test]
    async fn guideline_create_and_read() {
        let m = module();
        let g = m
            .create_guideline(GuidelineCreateParams {
                agent_id: AgentId::new(),
                content: GuidelineContent {
                    condition: "if x".into(),
                    action: "do y".into(),
                    description: None,
                },
                enabled: true,
                criticality: 1,
            })
            .await
            .unwrap();
        let loaded = m.read_guideline(&g.id).await.unwrap().unwrap();
        assert_eq!(loaded.content.condition, "if x");
    }

    #[tokio::test]
    async fn guideline_add_dependency_creates_relationship() {
        let m = module();
        let g = m
            .create_guideline(GuidelineCreateParams {
                agent_id: AgentId::new(),
                content: GuidelineContent {
                    condition: "c".into(),
                    action: "a".into(),
                    description: None,
                },
                enabled: true,
                criticality: 0,
            })
            .await
            .unwrap();
        let tag = TagId::new();
        let rel = m.add_dependency(&g.id, &tag).await.unwrap();
        assert_eq!(rel.kind, RelationshipKind::Dependency);
        assert_eq!(rel.source.id, g.id.0);
        assert_eq!(rel.target.id, tag.0);
    }

    #[tokio::test]
    async fn guideline_exclude_and_associate_tool() {
        let m = module();
        let g = m
            .create_guideline(GuidelineCreateParams {
                agent_id: AgentId::new(),
                content: GuidelineContent {
                    condition: "c".into(),
                    action: "a".into(),
                    description: None,
                },
                enabled: true,
                criticality: 0,
            })
            .await
            .unwrap();
        let tag = TagId::new();
        let rel = m.exclude(&g.id, &tag).await.unwrap();
        assert_eq!(rel.kind, RelationshipKind::Excludes);

        let tool = ToolId::new();
        let assoc = m.associate_tool(&g.id, &tool).await.unwrap();
        assert_eq!(assoc.guideline_id, g.id);
        assert_eq!(assoc.tool_id, tool);
    }
}

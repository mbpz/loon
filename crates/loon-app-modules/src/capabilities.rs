//! Application-level wrappers around `CapabilityStore` and `RetrieverStore`.

use std::sync::Arc;

use loon_core::stores::{CapabilityStore, RetrieverStore};
use loon_core::{
    AgentId, Capability, CapabilityId, CapabilityUpdateParams, CoreResult, Retriever, RetrieverId,
    TagId,
};

#[derive(Debug, Clone)]
pub struct CapabilityCreateParams {
    pub agent_id: AgentId,
    pub name: String,
    pub description: String,
    pub tags: Vec<TagId>,
}

pub struct CapabilityAppModule {
    pub store: Arc<dyn CapabilityStore>,
}

impl CapabilityAppModule {
    pub fn new(store: Arc<dyn CapabilityStore>) -> Self {
        Self { store }
    }

    pub async fn create_capability(
        &self,
        params: CapabilityCreateParams,
    ) -> CoreResult<Capability> {
        let mut c = Capability::new(&params.agent_id, params.name, params.description);
        c.tags = params.tags;
        self.store.create(c).await
    }

    pub async fn read_capability(
        &self,
        id: &CapabilityId,
    ) -> CoreResult<Option<Capability>> {
        self.store.read(id).await
    }

    pub async fn update_capability(
        &self,
        id: &CapabilityId,
        params: CapabilityUpdateParams,
    ) -> CoreResult<Capability> {
        self.store.update(id, params).await
    }

    pub async fn delete_capability(&self, id: &CapabilityId) -> CoreResult<()> {
        self.store.delete(id).await
    }

    pub async fn list_capabilities(
        &self,
        agent_id: &AgentId,
    ) -> CoreResult<Vec<Capability>> {
        self.store.list(agent_id).await
    }
}

#[derive(Debug, Clone)]
pub struct RetrieverCreateParams {
    pub agent_id: AgentId,
    pub name: String,
    pub description: String,
    pub tags: Vec<TagId>,
}

pub struct RetrieverAppModule {
    pub store: Arc<dyn RetrieverStore>,
}

impl RetrieverAppModule {
    pub fn new(store: Arc<dyn RetrieverStore>) -> Self {
        Self { store }
    }

    pub async fn create_retriever(
        &self,
        params: RetrieverCreateParams,
    ) -> CoreResult<Retriever> {
        let mut r = Retriever::new(&params.agent_id, params.name);
        r.description = params.description;
        r.tags = params.tags;
        self.store.create(r).await
    }

    pub async fn read_retriever(
        &self,
        id: &RetrieverId,
    ) -> CoreResult<Option<Retriever>> {
        self.store.read(id).await
    }

    pub async fn delete_retriever(&self, id: &RetrieverId) -> CoreResult<()> {
        self.store.delete(id).await
    }

    pub async fn list_retrievers(
        &self,
        agent_id: &AgentId,
    ) -> CoreResult<Vec<Retriever>> {
        self.store.list(agent_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use parking_lot::Mutex;
    use std::collections::HashMap;

    pub struct FakeCapabilityStore {
        pub data: Mutex<HashMap<CapabilityId, Capability>>,
    }
    impl FakeCapabilityStore {
        pub fn new() -> Self {
            Self {
                data: Mutex::new(HashMap::new()),
            }
        }
    }

    #[async_trait]
    impl CapabilityStore for FakeCapabilityStore {
        async fn create(&self, c: Capability) -> CoreResult<Capability> {
            let id = c.id.clone();
            self.data.lock().insert(id, c.clone());
            Ok(c)
        }
        async fn read(&self, id: &CapabilityId) -> CoreResult<Option<Capability>> {
            Ok(self.data.lock().get(id).cloned())
        }
        async fn update(
            &self,
            id: &CapabilityId,
            p: CapabilityUpdateParams,
        ) -> CoreResult<Capability> {
            let mut g = self.data.lock();
            let c = g.get_mut(id).unwrap();
            if let Some(n) = p.name {
                c.name = n;
            }
            if let Some(d) = p.description {
                c.description = d;
            }
            Ok(c.clone())
        }
        async fn delete(&self, id: &CapabilityId) -> CoreResult<()> {
            self.data.lock().remove(id);
            Ok(())
        }
        async fn list(&self, _agent_id: &AgentId) -> CoreResult<Vec<Capability>> {
            Ok(self.data.lock().values().cloned().collect())
        }
    }

    pub struct FakeRetrieverStore {
        pub data: Mutex<HashMap<RetrieverId, Retriever>>,
    }
    impl FakeRetrieverStore {
        pub fn new() -> Self {
            Self {
                data: Mutex::new(HashMap::new()),
            }
        }
    }

    #[async_trait]
    impl RetrieverStore for FakeRetrieverStore {
        async fn create(&self, r: Retriever) -> CoreResult<Retriever> {
            let id = r.id.clone();
            self.data.lock().insert(id, r.clone());
            Ok(r)
        }
        async fn read(&self, id: &RetrieverId) -> CoreResult<Option<Retriever>> {
            Ok(self.data.lock().get(id).cloned())
        }
        async fn delete(&self, id: &RetrieverId) -> CoreResult<()> {
            self.data.lock().remove(id);
            Ok(())
        }
        async fn list(&self, _agent_id: &AgentId) -> CoreResult<Vec<Retriever>> {
            Ok(self.data.lock().values().cloned().collect())
        }
    }

    #[tokio::test]
    async fn capability_create_and_read() {
        let store: Arc<dyn CapabilityStore> = Arc::new(FakeCapabilityStore::new());
        let module = CapabilityAppModule::new(store);
        let c = module
            .create_capability(CapabilityCreateParams {
                agent_id: AgentId::new(),
                name: "cap".into(),
                description: "d".into(),
                tags: vec![],
            })
            .await
            .unwrap();
        let loaded = module.read_capability(&c.id).await.unwrap().unwrap();
        assert_eq!(loaded.name, "cap");
    }

    #[tokio::test]
    async fn retriever_create_and_list() {
        let store: Arc<dyn RetrieverStore> = Arc::new(FakeRetrieverStore::new());
        let module = RetrieverAppModule::new(store);
        let _r = module
            .create_retriever(RetrieverCreateParams {
                agent_id: AgentId::new(),
                name: "ret".into(),
                description: "".into(),
                tags: vec![],
            })
            .await
            .unwrap();
        let all = module.list_retrievers(&AgentId::new()).await.unwrap();
        assert_eq!(all.len(), 1);
    }
}

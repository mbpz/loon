//! Application-level wrapper around `AgentStore` that exposes a
//! high-level CRUD API for use by the engine / CLI / SDK layers.

use std::sync::Arc;

use loon_core::stores::AgentStore;
use loon_core::{
    Agent, AgentId, AgentUpdateParams, CompositionMode, CoreResult, MessageOutputMode, TagId,
};

#[derive(Default, Debug, Clone)]
pub struct AgentCreateParams {
    pub name: String,
    pub description: String,
    pub composition_mode: Option<CompositionMode>,
    pub message_output_mode: Option<MessageOutputMode>,
    pub tags: Vec<TagId>,
}

pub struct AgentAppModule {
    pub store: Arc<dyn AgentStore>,
}

impl AgentAppModule {
    pub fn new(store: Arc<dyn AgentStore>) -> Self {
        Self { store }
    }

    pub async fn create_agent(&self, params: AgentCreateParams) -> CoreResult<Agent> {
        let mut agent = Agent::new(params.name, params.description);
        if let Some(cm) = params.composition_mode {
            agent.composition_mode = cm;
        }
        if let Some(mom) = params.message_output_mode {
            agent.message_output_mode = mom;
        }
        agent.tags = params.tags;
        self.store.create(agent).await
    }

    pub async fn read_agent(&self, id: &AgentId) -> CoreResult<Option<Agent>> {
        self.store.read(id).await
    }

    pub async fn update_agent(&self, id: &AgentId, params: AgentUpdateParams) -> CoreResult<Agent> {
        self.store.update(id, params).await
    }

    pub async fn delete_agent(&self, id: &AgentId) -> CoreResult<()> {
        self.store.delete(id).await
    }

    pub async fn list_agents(&self, tags: &[TagId]) -> CoreResult<Vec<Agent>> {
        self.store.list(tags).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use parking_lot::Mutex;
    use std::collections::HashMap;

    pub struct FakeAgentStore {
        pub data: Mutex<HashMap<AgentId, Agent>>,
    }

    impl FakeAgentStore {
        pub fn new() -> Self {
            Self {
                data: Mutex::new(HashMap::new()),
            }
        }
    }

    #[async_trait]
    impl AgentStore for FakeAgentStore {
        async fn create(&self, agent: Agent) -> CoreResult<Agent> {
            let id = agent.id.clone();
            self.data.lock().insert(id, agent.clone());
            Ok(agent)
        }

        async fn read(&self, id: &AgentId) -> CoreResult<Option<Agent>> {
            Ok(self.data.lock().get(id).cloned())
        }

        async fn update(&self, id: &AgentId, params: AgentUpdateParams) -> CoreResult<Agent> {
            let mut g = self.data.lock();
            let a = g.get_mut(id).expect("agent must exist");
            if let Some(n) = params.name {
                a.name = n;
            }
            if let Some(d) = params.description {
                a.description = d;
            }
            if let Some(cm) = params.composition_mode {
                a.composition_mode = cm;
            }
            if let Some(mom) = params.message_output_mode {
                a.message_output_mode = mom;
            }
            if let Some(t) = params.tags {
                a.tags = t;
            }
            Ok(a.clone())
        }

        async fn delete(&self, id: &AgentId) -> CoreResult<()> {
            self.data.lock().remove(id);
            Ok(())
        }

        async fn list(&self, _tags: &[TagId]) -> CoreResult<Vec<Agent>> {
            Ok(self.data.lock().values().cloned().collect())
        }
    }

    #[tokio::test]
    async fn agent_module_create_and_read() {
        let store: Arc<dyn AgentStore> = Arc::new(FakeAgentStore::new());
        let module = AgentAppModule::new(store);
        let a = module
            .create_agent(AgentCreateParams {
                name: "a".into(),
                description: "x".into(),
                ..Default::default()
            })
            .await
            .unwrap();
        let loaded = module.read_agent(&a.id).await.unwrap().unwrap();
        assert_eq!(loaded.name, "a");
        assert_eq!(loaded.description, "x");
    }

    #[tokio::test]
    async fn agent_module_update_and_delete() {
        let store: Arc<dyn AgentStore> = Arc::new(FakeAgentStore::new());
        let module = AgentAppModule::new(store);
        let a = module
            .create_agent(AgentCreateParams {
                name: "a".into(),
                description: "x".into(),
                ..Default::default()
            })
            .await
            .unwrap();
        let updated = module
            .update_agent(
                &a.id,
                AgentUpdateParams {
                    name: Some("b".into()),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(updated.name, "b");
        module.delete_agent(&a.id).await.unwrap();
        assert!(module.read_agent(&a.id).await.unwrap().is_none());
    }
}

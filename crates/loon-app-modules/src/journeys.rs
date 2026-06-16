//! Application-level wrapper around `JourneyStore`.

use std::sync::Arc;

use loon_core::stores::JourneyStore;
use loon_core::{
    AgentId, CoreResult, Journey, JourneyId, JourneyNode, JourneyUpdateParams,
};

#[derive(Debug, Clone)]
pub struct JourneyCreateParams {
    pub agent_id: AgentId,
    pub title: String,
    pub description: String,
}

pub struct JourneyAppModule {
    pub store: Arc<dyn JourneyStore>,
}

impl JourneyAppModule {
    pub fn new(store: Arc<dyn JourneyStore>) -> Self {
        Self { store }
    }

    pub async fn create_journey(&self, params: JourneyCreateParams) -> CoreResult<Journey> {
        let j = Journey {
            id: JourneyId::new(),
            agent_id: params.agent_id,
            title: params.title,
            description: params.description,
            root_id: JourneyNode::initial().id,
            tags: vec![],
            creation_utc: chrono::Utc::now(),
        };
        self.store.create(j).await
    }

    pub async fn read_journey(&self, id: &JourneyId) -> CoreResult<Option<Journey>> {
        self.store.read(id).await
    }

    pub async fn update_journey(
        &self,
        id: &JourneyId,
        params: JourneyUpdateParams,
    ) -> CoreResult<Journey> {
        self.store.update(id, params).await
    }

    pub async fn delete_journey(&self, id: &JourneyId) -> CoreResult<()> {
        self.store.delete(id).await
    }

    pub async fn list_journeys(&self, agent_id: &AgentId) -> CoreResult<Vec<Journey>> {
        self.store.list(agent_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use parking_lot::Mutex;
    use std::collections::HashMap;

    pub struct FakeJourneyStore {
        pub data: Mutex<HashMap<JourneyId, Journey>>,
    }
    impl FakeJourneyStore {
        pub fn new() -> Self {
            Self {
                data: Mutex::new(HashMap::new()),
            }
        }
    }

    #[async_trait]
    impl JourneyStore for FakeJourneyStore {
        async fn create(&self, j: Journey) -> CoreResult<Journey> {
            let id = j.id.clone();
            self.data.lock().insert(id, j.clone());
            Ok(j)
        }
        async fn read(&self, id: &JourneyId) -> CoreResult<Option<Journey>> {
            Ok(self.data.lock().get(id).cloned())
        }
        async fn update(
            &self,
            id: &JourneyId,
            p: JourneyUpdateParams,
        ) -> CoreResult<Journey> {
            let mut g = self.data.lock();
            let j = g.get_mut(id).unwrap();
            if let Some(t) = p.title {
                j.title = t;
            }
            if let Some(d) = p.description {
                j.description = d;
            }
            Ok(j.clone())
        }
        async fn delete(&self, id: &JourneyId) -> CoreResult<()> {
            self.data.lock().remove(id);
            Ok(())
        }
        async fn list(&self, _agent_id: &AgentId) -> CoreResult<Vec<Journey>> {
            Ok(self.data.lock().values().cloned().collect())
        }
    }

    #[tokio::test]
    async fn journey_create_and_read() {
        let store: Arc<dyn JourneyStore> = Arc::new(FakeJourneyStore::new());
        let module = JourneyAppModule::new(store);
        let j = module
            .create_journey(JourneyCreateParams {
                agent_id: AgentId::new(),
                title: "t".into(),
                description: "d".into(),
            })
            .await
            .unwrap();
        let loaded = module.read_journey(&j.id).await.unwrap().unwrap();
        assert_eq!(loaded.title, "t");
    }

    #[tokio::test]
    async fn journey_list_returns_all() {
        let store: Arc<dyn JourneyStore> = Arc::new(FakeJourneyStore::new());
        let module = JourneyAppModule::new(store);
        module
            .create_journey(JourneyCreateParams {
                agent_id: AgentId::new(),
                title: "a".into(),
                description: "".into(),
            })
            .await
            .unwrap();
        module
            .create_journey(JourneyCreateParams {
                agent_id: AgentId::new(),
                title: "b".into(),
                description: "".into(),
            })
            .await
            .unwrap();
        let all = module.list_journeys(&AgentId::new()).await.unwrap();
        assert_eq!(all.len(), 2);
    }
}

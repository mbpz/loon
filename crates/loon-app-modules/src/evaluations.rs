//! Application-level wrapper around `EvaluationStore` (the entity being
//! persisted is `Observation`).

use std::sync::Arc;

use loon_core::stores::EvaluationStore;
use loon_core::{AgentId, CoreResult, EvaluationId, Observation, ToolId};

#[derive(Debug, Clone)]
pub struct ObservationCreateParams {
    pub agent_id: AgentId,
    pub condition: String,
    pub tools: Vec<ToolId>,
    pub enabled: bool,
}

pub struct EvaluationAppModule {
    pub store: Arc<dyn EvaluationStore>,
}

impl EvaluationAppModule {
    pub fn new(store: Arc<dyn EvaluationStore>) -> Self {
        Self { store }
    }

    pub async fn create_observation(
        &self,
        params: ObservationCreateParams,
    ) -> CoreResult<Observation> {
        let mut o = Observation::new(params.condition, params.tools, &params.agent_id);
        o.enabled = params.enabled;
        self.store.create(o).await
    }

    pub async fn read_observation(&self, id: &EvaluationId) -> CoreResult<Option<Observation>> {
        self.store.read(id).await
    }

    pub async fn delete_observation(&self, id: &EvaluationId) -> CoreResult<()> {
        self.store.delete(id).await
    }

    pub async fn list_observations(&self, agent_id: &AgentId) -> CoreResult<Vec<Observation>> {
        self.store.list(agent_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use parking_lot::Mutex;
    use std::collections::HashMap;

    pub struct FakeEvaluationStore {
        pub data: Mutex<HashMap<EvaluationId, Observation>>,
    }
    impl FakeEvaluationStore {
        pub fn new() -> Self {
            Self {
                data: Mutex::new(HashMap::new()),
            }
        }
    }

    #[async_trait]
    impl EvaluationStore for FakeEvaluationStore {
        async fn create(&self, e: Observation) -> CoreResult<Observation> {
            let id = e.id.clone();
            self.data.lock().insert(id, e.clone());
            Ok(e)
        }
        async fn read(&self, id: &EvaluationId) -> CoreResult<Option<Observation>> {
            Ok(self.data.lock().get(id).cloned())
        }
        async fn update(
            &self,
            id: &EvaluationId,
            params: loon_core::ObservationUpdateParams,
        ) -> CoreResult<Observation> {
            let mut d = self.data.lock();
            let o = d
                .get_mut(id)
                .ok_or_else(|| loon_core::CoreError::NotFound(loon_core::UniqueId(id.0.clone())))?;
            if let Some(c) = params.condition {
                o.condition = c;
            }
            if let Some(t) = params.tools {
                o.tools = t;
            }
            if let Some(en) = params.enabled {
                o.enabled = en;
            }
            Ok(o.clone())
        }
        async fn delete(&self, id: &EvaluationId) -> CoreResult<()> {
            self.data.lock().remove(id);
            Ok(())
        }
        async fn list(&self, _agent_id: &AgentId) -> CoreResult<Vec<Observation>> {
            Ok(self.data.lock().values().cloned().collect())
        }
    }

    #[tokio::test]
    async fn observation_create_and_read() {
        let store: Arc<dyn EvaluationStore> = Arc::new(FakeEvaluationStore::new());
        let module = EvaluationAppModule::new(store);
        let o = module
            .create_observation(ObservationCreateParams {
                agent_id: AgentId::new(),
                condition: "if x".into(),
                tools: vec![ToolId::new()],
                enabled: true,
            })
            .await
            .unwrap();
        let loaded = module.read_observation(&o.id).await.unwrap().unwrap();
        assert_eq!(loaded.condition, "if x");
        assert_eq!(loaded.tools.len(), 1);
    }
}

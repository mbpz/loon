//! Application-level wrapper around `CannedResponseStore`.

use std::sync::Arc;

use loon_core::stores::CannedResponseStore;
use loon_core::{
    AgentId, CannedResponse, CannedResponseId, CannedResponseUpdateParams, CoreResult, TagId,
};

#[derive(Debug, Clone)]
pub struct CannedResponseCreateParams {
    pub agent_id: AgentId,
    pub value: String,
    pub tags: Vec<TagId>,
    pub matchers: Vec<String>,
}

pub struct CannedResponseAppModule {
    pub store: Arc<dyn CannedResponseStore>,
}

impl CannedResponseAppModule {
    pub fn new(store: Arc<dyn CannedResponseStore>) -> Self {
        Self { store }
    }

    pub async fn create_canned_response(
        &self,
        params: CannedResponseCreateParams,
    ) -> CoreResult<CannedResponse> {
        let mut c = CannedResponse::new(&params.agent_id, params.value);
        c.tags = params.tags;
        c.matchers = params.matchers;
        self.store.create(c).await
    }

    pub async fn read_canned_response(
        &self,
        id: &CannedResponseId,
    ) -> CoreResult<Option<CannedResponse>> {
        self.store.read(id).await
    }

    pub async fn update_canned_response(
        &self,
        id: &CannedResponseId,
        params: CannedResponseUpdateParams,
    ) -> CoreResult<CannedResponse> {
        self.store.update(id, params).await
    }

    pub async fn delete_canned_response(&self, id: &CannedResponseId) -> CoreResult<()> {
        self.store.delete(id).await
    }

    pub async fn list_canned_responses(
        &self,
        agent_id: &AgentId,
    ) -> CoreResult<Vec<CannedResponse>> {
        self.store.list(agent_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use parking_lot::Mutex;
    use std::collections::HashMap;

    pub struct FakeCannedResponseStore {
        pub data: Mutex<HashMap<CannedResponseId, CannedResponse>>,
    }
    impl FakeCannedResponseStore {
        pub fn new() -> Self {
            Self {
                data: Mutex::new(HashMap::new()),
            }
        }
    }

    #[async_trait]
    impl CannedResponseStore for FakeCannedResponseStore {
        async fn create(&self, c: CannedResponse) -> CoreResult<CannedResponse> {
            let id = c.id.clone();
            self.data.lock().insert(id, c.clone());
            Ok(c)
        }
        async fn read(&self, id: &CannedResponseId) -> CoreResult<Option<CannedResponse>> {
            Ok(self.data.lock().get(id).cloned())
        }
        async fn update(
            &self,
            id: &CannedResponseId,
            p: CannedResponseUpdateParams,
        ) -> CoreResult<CannedResponse> {
            let mut g = self.data.lock();
            let c = g.get_mut(id).unwrap();
            if let Some(v) = p.value {
                c.value = v;
            }
            if let Some(m) = p.matchers {
                c.matchers = m;
            }
            Ok(c.clone())
        }
        async fn delete(&self, id: &CannedResponseId) -> CoreResult<()> {
            self.data.lock().remove(id);
            Ok(())
        }
        async fn list(&self, _agent_id: &AgentId) -> CoreResult<Vec<CannedResponse>> {
            Ok(self.data.lock().values().cloned().collect())
        }
    }

    #[tokio::test]
    async fn canned_response_create_and_read() {
        let store: Arc<dyn CannedResponseStore> = Arc::new(FakeCannedResponseStore::new());
        let module = CannedResponseAppModule::new(store);
        let c = module
            .create_canned_response(CannedResponseCreateParams {
                agent_id: AgentId::new(),
                value: "Hello!".into(),
                tags: vec![],
                matchers: vec!["hi".into()],
            })
            .await
            .unwrap();
        let loaded = module.read_canned_response(&c.id).await.unwrap().unwrap();
        assert_eq!(loaded.value, "Hello!");
        assert_eq!(loaded.matchers, vec!["hi".to_string()]);
    }
}

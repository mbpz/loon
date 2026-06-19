//! Application-level wrapper around `ContextVariableStore` with helpers
//! for upserting a single key/value pair on a context variable.

use std::sync::Arc;

use loon_core::stores::ContextVariableStore;
use loon_core::{
    AgentId, ContextVariable, ContextVariableId, ContextVariableUpdateParams, ContextVariableValue,
    CoreResult, FreshnessRule, JsonValue, TagId,
};

#[derive(Debug, Clone)]
pub struct ContextVariableCreateParams {
    pub agent_id: AgentId,
    pub key: String,
    pub freshness_rules: Vec<FreshnessRule>,
    pub tags: Vec<TagId>,
}

pub struct ContextVariableAppModule {
    pub store: Arc<dyn ContextVariableStore>,
}

impl ContextVariableAppModule {
    pub fn new(store: Arc<dyn ContextVariableStore>) -> Self {
        Self { store }
    }

    pub async fn create_context_variable(
        &self,
        params: ContextVariableCreateParams,
    ) -> CoreResult<ContextVariable> {
        let v = ContextVariable {
            id: ContextVariableId::new(),
            agent_id: params.agent_id,
            key: params.key,
            freshness_rules: params.freshness_rules,
            tags: params.tags,
            creation_utc: chrono::Utc::now(),
        };
        self.store.create(v).await
    }

    pub async fn read_context_variable(
        &self,
        id: &ContextVariableId,
    ) -> CoreResult<Option<ContextVariable>> {
        self.store.read(id).await
    }

    pub async fn update_context_variable(
        &self,
        id: &ContextVariableId,
        params: ContextVariableUpdateParams,
    ) -> CoreResult<ContextVariable> {
        self.store.update(id, params).await
    }

    pub async fn delete_context_variable(&self, id: &ContextVariableId) -> CoreResult<()> {
        self.store.delete(id).await
    }

    pub async fn list_context_variables(
        &self,
        agent_id: &AgentId,
    ) -> CoreResult<Vec<ContextVariable>> {
        self.store.list(agent_id).await
    }

    pub async fn update_value(
        &self,
        var_id: &ContextVariableId,
        key: &str,
        data: JsonValue,
    ) -> CoreResult<ContextVariableValue> {
        self.store.upsert_value(var_id, key, data).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use parking_lot::Mutex;
    use std::collections::HashMap;

    pub struct FakeContextVariableStore {
        pub data: Mutex<HashMap<ContextVariableId, ContextVariable>>,
        pub values: Mutex<HashMap<(ContextVariableId, String), ContextVariableValue>>,
    }
    impl FakeContextVariableStore {
        pub fn new() -> Self {
            Self {
                data: Mutex::new(HashMap::new()),
                values: Mutex::new(HashMap::new()),
            }
        }
    }

    #[async_trait]
    impl ContextVariableStore for FakeContextVariableStore {
        async fn create(&self, v: ContextVariable) -> CoreResult<ContextVariable> {
            let id = v.id.clone();
            self.data.lock().insert(id, v.clone());
            Ok(v)
        }
        async fn read(&self, id: &ContextVariableId) -> CoreResult<Option<ContextVariable>> {
            Ok(self.data.lock().get(id).cloned())
        }
        async fn update(
            &self,
            id: &ContextVariableId,
            p: ContextVariableUpdateParams,
        ) -> CoreResult<ContextVariable> {
            // Phase 1: capture the (id, key) from the descriptor while
            // holding the data lock, mirroring the in-memory impl.
            let (var_id, value_key) = {
                let mut g = self.data.lock();
                let v = g.get_mut(id).ok_or_else(|| {
                    loon_core::CoreError::NotFound(loon_core::UniqueId(id.0.clone()))
                })?;
                if let Some(k) = p.key {
                    v.key = k;
                }
                (v.id.clone(), v.key.clone())
            };
            // Phase 2: if data is provided, persist to the value store
            // keyed by the variable's own key.
            if let Some(payload) = p.data {
                self.upsert_value(&var_id, &value_key, payload).await?;
            }
            // Phase 3: re-read the descriptor and return it.
            let g = self.data.lock();
            Ok(g.get(&var_id)
                .cloned()
                .expect("descriptor must still exist"))
        }
        async fn delete(&self, id: &ContextVariableId) -> CoreResult<()> {
            self.data.lock().remove(id);
            Ok(())
        }
        async fn list(&self, _agent_id: &AgentId) -> CoreResult<Vec<ContextVariable>> {
            Ok(self.data.lock().values().cloned().collect())
        }
        async fn upsert_value(
            &self,
            var_id: &ContextVariableId,
            key: &str,
            data: JsonValue,
        ) -> CoreResult<ContextVariableValue> {
            let val = ContextVariableValue {
                key: key.into(),
                data,
                last_updated: chrono::Utc::now(),
            };
            self.values
                .lock()
                .insert((var_id.clone(), key.into()), val.clone());
            Ok(val)
        }
        async fn read_value(
            &self,
            var_id: &ContextVariableId,
            key: &str,
        ) -> CoreResult<Option<ContextVariableValue>> {
            Ok(self
                .values
                .lock()
                .get(&(var_id.clone(), key.into()))
                .cloned())
        }
    }

    #[tokio::test]
    async fn context_variable_create_read_update_value() {
        let store: Arc<dyn ContextVariableStore> = Arc::new(FakeContextVariableStore::new());
        let module = ContextVariableAppModule::new(store);
        let v = module
            .create_context_variable(ContextVariableCreateParams {
                agent_id: AgentId::new(),
                key: "k".into(),
                freshness_rules: vec![],
                tags: vec![],
            })
            .await
            .unwrap();
        let loaded = module.read_context_variable(&v.id).await.unwrap().unwrap();
        assert_eq!(loaded.key, "k");
        let val = module
            .update_value(&v.id, "k", serde_json::json!({"n": 1}))
            .await
            .unwrap();
        assert_eq!(val.key, "k");
    }
}

//! Application-level wrapper around `GlossaryStore`.

use std::sync::Arc;

use loon_core::stores::GlossaryStore;
use loon_core::{AgentId, CoreResult, GlossaryTermId, Term};

pub struct GlossaryAppModule {
    pub store: Arc<dyn GlossaryStore>,
}

impl GlossaryAppModule {
    pub fn new(store: Arc<dyn GlossaryStore>) -> Self {
        Self { store }
    }

    pub async fn create_term(&self, term: Term) -> CoreResult<Term> {
        self.store.create_term(term).await
    }

    pub async fn read_term(&self, id: &GlossaryTermId) -> CoreResult<Option<Term>> {
        self.store.read_term(id).await
    }

    pub async fn update_term(&self, id: &GlossaryTermId, term: Term) -> CoreResult<Term> {
        self.store.update_term(id, term).await
    }

    pub async fn delete_term(&self, id: &GlossaryTermId) -> CoreResult<()> {
        self.store.delete_term(id).await
    }

    pub async fn list_terms(&self, agent_id: &AgentId) -> CoreResult<Vec<Term>> {
        self.store.list_terms(agent_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use parking_lot::Mutex;
    use std::collections::HashMap;

    pub struct FakeGlossaryStore {
        pub data: Mutex<HashMap<GlossaryTermId, Term>>,
    }
    impl FakeGlossaryStore {
        pub fn new() -> Self {
            Self {
                data: Mutex::new(HashMap::new()),
            }
        }
    }

    #[async_trait]
    impl GlossaryStore for FakeGlossaryStore {
        async fn create_term(&self, t: Term) -> CoreResult<Term> {
            let id = t.id.clone();
            self.data.lock().insert(id, t.clone());
            Ok(t)
        }
        async fn read_term(&self, id: &GlossaryTermId) -> CoreResult<Option<Term>> {
            Ok(self.data.lock().get(id).cloned())
        }
        async fn update_term(&self, id: &GlossaryTermId, t: Term) -> CoreResult<Term> {
            self.data.lock().insert(id.clone(), t.clone());
            Ok(t)
        }
        async fn delete_term(&self, id: &GlossaryTermId) -> CoreResult<()> {
            self.data.lock().remove(id);
            Ok(())
        }
        async fn list_terms(&self, _agent_id: &AgentId) -> CoreResult<Vec<Term>> {
            Ok(self.data.lock().values().cloned().collect())
        }
    }

    #[tokio::test]
    async fn glossary_create_and_read_term() {
        let store: Arc<dyn GlossaryStore> = Arc::new(FakeGlossaryStore::new());
        let module = GlossaryAppModule::new(store);
        let t = module.create_term(Term::new("foo", "bar")).await.unwrap();
        let loaded = module.read_term(&t.id).await.unwrap().unwrap();
        assert_eq!(loaded.name, "foo");
        assert_eq!(loaded.description, "bar");
    }
}

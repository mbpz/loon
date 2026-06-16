//! Application-level wrapper around `CustomerStore`.

use std::sync::Arc;

use loon_core::stores::CustomerStore;
use loon_core::{CoreResult, Customer, CustomerId, CustomerUpdateParams, JsonValue, TagId};

#[derive(Debug, Clone)]
pub struct CustomerCreateParams {
    pub name: String,
    pub metadata: Option<JsonValue>,
    pub tags: Vec<TagId>,
}

pub struct CustomerAppModule {
    pub store: Arc<dyn CustomerStore>,
}

impl CustomerAppModule {
    pub fn new(store: Arc<dyn CustomerStore>) -> Self {
        Self { store }
    }

    pub async fn create_customer(&self, params: CustomerCreateParams) -> CoreResult<Customer> {
        let mut c = Customer::new(params.name);
        if let Some(m) = params.metadata {
            c.metadata = m;
        }
        c.tags = params.tags;
        self.store.create(c).await
    }

    pub async fn read_customer(&self, id: &CustomerId) -> CoreResult<Option<Customer>> {
        self.store.read(id).await
    }

    pub async fn update_customer(
        &self,
        id: &CustomerId,
        params: CustomerUpdateParams,
    ) -> CoreResult<Customer> {
        self.store.update(id, params).await
    }

    pub async fn delete_customer(&self, id: &CustomerId) -> CoreResult<()> {
        self.store.delete(id).await
    }

    pub async fn list_customers(&self, tags: &[TagId]) -> CoreResult<Vec<Customer>> {
        self.store.list(tags).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use parking_lot::Mutex;
    use std::collections::HashMap;

    pub struct FakeCustomerStore {
        pub data: Mutex<HashMap<CustomerId, Customer>>,
    }
    impl FakeCustomerStore {
        pub fn new() -> Self {
            Self {
                data: Mutex::new(HashMap::new()),
            }
        }
    }

    #[async_trait]
    impl CustomerStore for FakeCustomerStore {
        async fn create(&self, c: Customer) -> CoreResult<Customer> {
            let id = c.id.clone();
            self.data.lock().insert(id, c.clone());
            Ok(c)
        }
        async fn read(&self, id: &CustomerId) -> CoreResult<Option<Customer>> {
            Ok(self.data.lock().get(id).cloned())
        }
        async fn update(
            &self,
            id: &CustomerId,
            p: CustomerUpdateParams,
        ) -> CoreResult<Customer> {
            let mut g = self.data.lock();
            let c = g.get_mut(id).unwrap();
            if let Some(n) = p.name {
                c.name = n;
            }
            if let Some(m) = p.metadata {
                c.metadata = m;
            }
            Ok(c.clone())
        }
        async fn delete(&self, id: &CustomerId) -> CoreResult<()> {
            self.data.lock().remove(id);
            Ok(())
        }
        async fn list(&self, _tags: &[TagId]) -> CoreResult<Vec<Customer>> {
            Ok(self.data.lock().values().cloned().collect())
        }
    }

    #[tokio::test]
    async fn customer_create_and_read() {
        let store: Arc<dyn CustomerStore> = Arc::new(FakeCustomerStore::new());
        let module = CustomerAppModule::new(store);
        let c = module
            .create_customer(CustomerCreateParams {
                name: "alice".into(),
                metadata: None,
                tags: vec![],
            })
            .await
            .unwrap();
        let loaded = module.read_customer(&c.id).await.unwrap().unwrap();
        assert_eq!(loaded.name, "alice");
    }
}

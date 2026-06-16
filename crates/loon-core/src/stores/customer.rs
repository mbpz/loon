use async_trait::async_trait;
use crate::{CoreResult, Customer, CustomerId, CustomerUpdateParams, TagId};

#[async_trait]
pub trait CustomerStore: Send + Sync {
    async fn create(&self, c: Customer) -> CoreResult<Customer>;
    async fn read(&self, id: &CustomerId) -> CoreResult<Option<Customer>>;
    async fn update(&self, id: &CustomerId, p: CustomerUpdateParams) -> CoreResult<Customer>;
    async fn delete(&self, id: &CustomerId) -> CoreResult<()>;
    async fn list(&self, tags: &[TagId]) -> CoreResult<Vec<Customer>>;
}

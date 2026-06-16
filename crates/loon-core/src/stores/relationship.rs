use crate::{CoreResult, Relationship, RelationshipEntity, RelationshipId};
use async_trait::async_trait;

#[async_trait]
pub trait RelationshipStore: Send + Sync {
    async fn create(&self, r: Relationship) -> CoreResult<Relationship>;
    async fn read(&self, id: &RelationshipId) -> CoreResult<Option<Relationship>>;
    async fn delete(&self, id: &RelationshipId) -> CoreResult<()>;
    async fn list_for(&self, entity: &RelationshipEntity) -> CoreResult<Vec<Relationship>>;
}

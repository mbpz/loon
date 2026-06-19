use crate::{CoreResult, Tag, TagId, TagUpdateParams};
use async_trait::async_trait;

#[async_trait]
pub trait TagStore: Send + Sync {
    async fn create(&self, tag: Tag) -> CoreResult<Tag>;
    async fn read(&self, id: &TagId) -> CoreResult<Option<Tag>>;
    async fn update(&self, id: &TagId, params: TagUpdateParams) -> CoreResult<Tag>;
    async fn list(&self) -> CoreResult<Vec<Tag>>;
    async fn delete(&self, id: &TagId) -> CoreResult<()>;
}

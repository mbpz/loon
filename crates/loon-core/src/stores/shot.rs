use crate::{AgentId, CoreResult, Shot, ShotId};
use async_trait::async_trait;

#[async_trait]
pub trait ShotStore: Send + Sync {
    async fn create(&self, s: Shot) -> CoreResult<Shot>;
    async fn read(&self, id: &ShotId) -> CoreResult<Option<Shot>>;
    async fn delete(&self, id: &ShotId) -> CoreResult<()>;
    async fn list(&self, agent_id: &AgentId) -> CoreResult<Vec<Shot>>;
}

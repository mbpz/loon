use crate::{AgentId, CoreResult, Journey, JourneyId, JourneyUpdateParams};
use async_trait::async_trait;

#[async_trait]
pub trait JourneyStore: Send + Sync {
    async fn create(&self, j: Journey) -> CoreResult<Journey>;
    async fn read(&self, id: &JourneyId) -> CoreResult<Option<Journey>>;
    async fn update(&self, id: &JourneyId, p: JourneyUpdateParams) -> CoreResult<Journey>;
    async fn delete(&self, id: &JourneyId) -> CoreResult<()>;
    async fn list(&self, agent_id: &AgentId) -> CoreResult<Vec<Journey>>;
}

use crate::{AgentId, CoreResult, Guideline, GuidelineId, GuidelineUpdateParams, TagId};
use async_trait::async_trait;

#[async_trait]
pub trait GuidelineStore: Send + Sync {
    async fn create(&self, g: Guideline) -> CoreResult<Guideline>;
    async fn read(&self, id: &GuidelineId) -> CoreResult<Option<Guideline>>;
    async fn update(&self, id: &GuidelineId, p: GuidelineUpdateParams) -> CoreResult<Guideline>;
    async fn delete(&self, id: &GuidelineId) -> CoreResult<()>;
    async fn list(&self, agent_id: &AgentId, tags: &[TagId]) -> CoreResult<Vec<Guideline>>;
}

use crate::{Agent, AgentId, AgentUpdateParams, CoreResult, TagId};
use async_trait::async_trait;

#[async_trait]
pub trait AgentStore: Send + Sync {
    async fn create(&self, agent: Agent) -> CoreResult<Agent>;
    async fn read(&self, id: &AgentId) -> CoreResult<Option<Agent>>;
    async fn update(&self, id: &AgentId, params: AgentUpdateParams) -> CoreResult<Agent>;
    async fn delete(&self, id: &AgentId) -> CoreResult<()>;
    async fn list(&self, tags: &[TagId]) -> CoreResult<Vec<Agent>>;
}

use crate::{AgentId, CoreResult, Tool, ToolId};
use async_trait::async_trait;

#[async_trait]
pub trait ToolStore: Send + Sync {
    async fn create(&self, t: Tool) -> CoreResult<Tool>;
    async fn read(&self, id: &ToolId) -> CoreResult<Option<Tool>>;
    async fn delete(&self, id: &ToolId) -> CoreResult<()>;
    async fn list(&self, agent_id: &AgentId) -> CoreResult<Vec<Tool>>;
}

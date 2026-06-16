use async_trait::async_trait;
use crate::{AgentId, CannedResponse, CannedResponseId, CannedResponseUpdateParams, CoreResult};

#[async_trait]
pub trait CannedResponseStore: Send + Sync {
    async fn create(&self, c: CannedResponse) -> CoreResult<CannedResponse>;
    async fn read(&self, id: &CannedResponseId) -> CoreResult<Option<CannedResponse>>;
    async fn update(&self, id: &CannedResponseId, p: CannedResponseUpdateParams) -> CoreResult<CannedResponse>;
    async fn delete(&self, id: &CannedResponseId) -> CoreResult<()>;
    async fn list(&self, agent_id: &AgentId) -> CoreResult<Vec<CannedResponse>>;
}

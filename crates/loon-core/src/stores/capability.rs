use async_trait::async_trait;
use crate::{AgentId, Capability, CapabilityId, CapabilityUpdateParams, CoreResult, Retriever, RetrieverId};

#[async_trait]
pub trait CapabilityStore: Send + Sync {
    async fn create(&self, c: Capability) -> CoreResult<Capability>;
    async fn read(&self, id: &CapabilityId) -> CoreResult<Option<Capability>>;
    async fn update(&self, id: &CapabilityId, p: CapabilityUpdateParams) -> CoreResult<Capability>;
    async fn delete(&self, id: &CapabilityId) -> CoreResult<()>;
    async fn list(&self, agent_id: &AgentId) -> CoreResult<Vec<Capability>>;
}

#[async_trait]
pub trait RetrieverStore: Send + Sync {
    async fn create(&self, r: Retriever) -> CoreResult<Retriever>;
    async fn read(&self, id: &RetrieverId) -> CoreResult<Option<Retriever>>;
    async fn delete(&self, id: &RetrieverId) -> CoreResult<()>;
    async fn list(&self, agent_id: &AgentId) -> CoreResult<Vec<Retriever>>;
}

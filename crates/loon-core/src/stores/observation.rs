use crate::{AgentId, CoreResult, EvaluationId, Observation};
use async_trait::async_trait;

#[async_trait]
pub trait EvaluationStore: Send + Sync {
    async fn create(&self, e: Observation) -> CoreResult<Observation>;
    async fn read(&self, id: &EvaluationId) -> CoreResult<Option<Observation>>;
    async fn delete(&self, id: &EvaluationId) -> CoreResult<()>;
    async fn list(&self, agent_id: &AgentId) -> CoreResult<Vec<Observation>>;
}

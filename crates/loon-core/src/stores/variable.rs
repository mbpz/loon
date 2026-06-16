use async_trait::async_trait;
use crate::common::JsonValue;
use crate::{AgentId, ContextVariable, ContextVariableId, ContextVariableValue, ContextVariableUpdateParams, CoreResult, FreshnessRule};

#[async_trait]
pub trait ContextVariableStore: Send + Sync {
    async fn create(&self, v: ContextVariable) -> CoreResult<ContextVariable>;
    async fn read(&self, id: &ContextVariableId) -> CoreResult<Option<ContextVariable>>;
    async fn update(&self, id: &ContextVariableId, p: ContextVariableUpdateParams) -> CoreResult<ContextVariable>;
    async fn delete(&self, id: &ContextVariableId) -> CoreResult<()>;
    async fn list(&self, agent_id: &AgentId) -> CoreResult<Vec<ContextVariable>>;
    async fn upsert_value(&self, var_id: &ContextVariableId, key: &str, data: JsonValue) -> CoreResult<ContextVariableValue>;
}

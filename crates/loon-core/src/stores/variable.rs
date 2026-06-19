use crate::common::JsonValue;
use crate::{
    AgentId, ContextVariable, ContextVariableId, ContextVariableUpdateParams, ContextVariableValue,
    CoreResult,
};
use async_trait::async_trait;

#[async_trait]
pub trait ContextVariableStore: Send + Sync {
    async fn create(&self, v: ContextVariable) -> CoreResult<ContextVariable>;
    async fn read(&self, id: &ContextVariableId) -> CoreResult<Option<ContextVariable>>;
    async fn update(
        &self,
        id: &ContextVariableId,
        p: ContextVariableUpdateParams,
    ) -> CoreResult<ContextVariable>;
    async fn delete(&self, id: &ContextVariableId) -> CoreResult<()>;
    async fn list(&self, agent_id: &AgentId) -> CoreResult<Vec<ContextVariable>>;
    async fn upsert_value(
        &self,
        var_id: &ContextVariableId,
        key: &str,
        data: JsonValue,
    ) -> CoreResult<ContextVariableValue>;

    /// Read the value stored at \`(var_id, key)\`. Returns
    /// \`None\` if no value has been upserted for that pair.
    async fn read_value(
        &self,
        var_id: &ContextVariableId,
        key: &str,
    ) -> CoreResult<Option<ContextVariableValue>>;
}

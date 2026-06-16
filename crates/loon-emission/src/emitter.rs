use async_trait::async_trait;
use loon_core::{AgentId, SessionId, JsonValue, StatusEventData, ToolEventData};
use std::collections::HashMap;
use crate::{EmissionResult, EmittedEvent, MessageEmitData, MessageEventHandle};

#[async_trait]
pub trait EventEmitter: Send + Sync {
    async fn emit_status_event(
        &self,
        trace_id: &str,
        data: StatusEventData,
        metadata: Option<HashMap<String, JsonValue>>,
    ) -> EmissionResult<EmittedEvent>;

    async fn emit_message_event(
        &self,
        trace_id: &str,
        data: MessageEmitData,
        metadata: Option<HashMap<String, JsonValue>>,
    ) -> EmissionResult<MessageEventHandle>;

    async fn emit_tool_event(
        &self,
        trace_id: &str,
        data: ToolEventData,
        metadata: Option<HashMap<String, JsonValue>>,
    ) -> EmissionResult<EmittedEvent>;

    async fn emit_custom_event(
        &self,
        trace_id: &str,
        data: JsonValue,
        metadata: Option<HashMap<String, JsonValue>>,
    ) -> EmissionResult<EmittedEvent>;
}

#[async_trait]
pub trait EventEmitterFactory: Send + Sync {
    async fn create_event_emitter(
        &self,
        agent_id: &AgentId,
        session_id: &SessionId,
    ) -> EmissionResult<Box<dyn EventEmitter>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // Compile-time assertion that the trait is dyn-compatible.
    fn _accepts_trait(_: &dyn EventEmitter) {}
    fn _accepts_factory(_: &dyn EventEmitterFactory) {}
}

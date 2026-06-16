use async_trait::async_trait;
use crate::{AgentId, CoreResult, CustomerId, Event, EventId, EventUpdateParams, Session, SessionId, SessionUpdateParams};

#[async_trait]
pub trait SessionStore: Send + Sync {
    async fn create(&self, s: Session) -> CoreResult<Session>;
    async fn read(&self, id: &SessionId) -> CoreResult<Option<Session>>;
    async fn update(&self, id: &SessionId, p: SessionUpdateParams) -> CoreResult<Session>;
    async fn delete(&self, id: &SessionId) -> CoreResult<()>;
    async fn list(&self, agent_id: Option<&AgentId>, customer_id: Option<&CustomerId>) -> CoreResult<Vec<Session>>;
    async fn create_event(&self, session_id: SessionId, event: Event) -> CoreResult<Event>;
    async fn update_event(&self, session_id: &SessionId, event_id: &EventId, p: EventUpdateParams) -> CoreResult<Event>;
    async fn read_events(&self, session_id: &SessionId) -> CoreResult<Vec<Event>>;
    async fn find_events(&self, session_id: &SessionId) -> CoreResult<Vec<Event>>;
}

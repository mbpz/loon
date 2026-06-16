use crate::{AgentId, CoreResult, Term};
use async_trait::async_trait;

pub use crate::glossary::Glossary;

#[async_trait]
pub trait GlossaryStore: Send + Sync {
    async fn create_term(&self, t: Term) -> CoreResult<Term>;
    async fn read_term(&self, id: &crate::GlossaryTermId) -> CoreResult<Option<Term>>;
    async fn update_term(&self, id: &crate::GlossaryTermId, t: Term) -> CoreResult<Term>;
    async fn delete_term(&self, id: &crate::GlossaryTermId) -> CoreResult<()>;
    async fn list_terms(&self, agent_id: &AgentId) -> CoreResult<Vec<Term>>;
}

use async_trait::async_trait;
use crate::GuidelineId;
use crate::ToolId;
use crate::CoreResult;
use crate::GuidelineToolAssociation;
use crate::GuidelineToolAssociationId;

#[async_trait]
pub trait GuidelineToolAssociationStore: Send + Sync {
    async fn create(&self, a: GuidelineToolAssociation) -> CoreResult<GuidelineToolAssociation>;
    async fn read(&self, id: &GuidelineToolAssociationId) -> CoreResult<Option<GuidelineToolAssociation>>;
    async fn delete(&self, id: &GuidelineToolAssociationId) -> CoreResult<()>;
    async fn list_for_tool(&self, tool_id: &ToolId) -> CoreResult<Vec<GuidelineToolAssociation>>;
    async fn list_for_guideline(&self, guideline_id: &GuidelineId) -> CoreResult<Vec<GuidelineToolAssociation>>;
}

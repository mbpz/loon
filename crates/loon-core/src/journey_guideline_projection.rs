use std::sync::Arc;
use crate::{
    stores::{GuidelineStore, JourneyStore},
    CoreError, CoreResult, Guideline, GuidelineId, JourneyId, JourneyNodeId, UniqueId,
};

pub struct JourneyGuidelineProjection {
    pub journey_store: Arc<dyn JourneyStore>,
    pub guideline_store: Arc<dyn GuidelineStore>,
}

impl JourneyGuidelineProjection {
    pub fn new(journey_store: Arc<dyn JourneyStore>, guideline_store: Arc<dyn GuidelineStore>) -> Self {
        Self { journey_store, guideline_store }
    }

    pub async fn project_journey_to_guidelines(&self, journey_id: &JourneyId) -> CoreResult<Vec<Guideline>> {
        let _journey = self
            .journey_store
            .read(journey_id)
            .await?
            .ok_or_else(|| CoreError::NotFound(UniqueId(journey_id.0.clone())))?;
        // Phase-1: projection is a stub. Full BFS-from-root traversal
        // and synthetic Guideline generation lives in a later milestone.
        let _ = self.guideline_store.as_ref();
        Ok(vec![])
    }
}

pub fn extract_node_id_from_journey_node_guideline_id(id: &GuidelineId) -> Option<JourneyNodeId> {
    let prefix = "journey_node:";
    id.0.strip_prefix(prefix)
        .and_then(|rest| rest.split(':').next())
        .map(|s| JourneyNodeId(s.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn extract_node_id_simple() {
        let gid = GuidelineId("journey_node:abc123:foo".to_string());
        assert_eq!(
            extract_node_id_from_journey_node_guideline_id(&gid),
            Some(JourneyNodeId("abc123".to_string()))
        );
    }

    #[test]
    fn extract_node_id_returns_none_for_other_prefix() {
        let gid = GuidelineId("guideline:xyz".to_string());
        assert_eq!(extract_node_id_from_journey_node_guideline_id(&gid), None);
    }
}

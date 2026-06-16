use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::GuidelineToolAssociationId;
use crate::ToolId;
use crate::GuidelineId;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GuidelineToolAssociation {
    pub id: GuidelineToolAssociationId,
    pub guideline_id: GuidelineId,
    pub tool_id: ToolId,
    pub creation_utc: DateTime<Utc>,
}

impl GuidelineToolAssociation {
    pub fn new(guideline_id: &GuidelineId, tool_id: &ToolId) -> Self {
        Self {
            id: GuidelineToolAssociationId::new(),
            guideline_id: guideline_id.clone(),
            tool_id: tool_id.clone(),
            creation_utc: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn association_round_trips_with_both_ids() {
        let g = GuidelineId::new();
        let t = ToolId::new();
        let a = GuidelineToolAssociation::new(&g, &t);
        assert_eq!(a.guideline_id, g);
        assert_eq!(a.tool_id, t);
    }
}

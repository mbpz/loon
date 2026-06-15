use crate::define_id;

define_id!(AgentId);
define_id!(GuidelineId);
define_id!(JourneyId);
define_id!(JourneyNodeId);
define_id!(JourneyEdgeId);
define_id!(ToolId);
define_id!(SessionId);
define_id!(CustomerId);
define_id!(TagId);
define_id!(RelationshipId);
define_id!(CannedResponseId);
define_id!(CapabilityId);
define_id!(ContextVariableId);
define_id!(RetrieverId);
define_id!(GlossaryTermId);
define_id!(EventId);
define_id!(MessageId);
define_id!(GuidelineToolAssociationId);
define_id!(ShotId);

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn agent_id_generates_unique() {
        let a = AgentId::new();
        let b = AgentId::new();
        assert_ne!(a, b);
        assert_eq!(a.as_str().len(), 10);
    }
}

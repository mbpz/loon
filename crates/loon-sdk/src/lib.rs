//! `loon-sdk` — the user-facing facade crate that embeds a `loon`
//! engine into another application.
//!
//! Phase 1 exposes:
//! - [`SdkError`] / [`SdkResult`] for unified error handling across
//!   every `loon` subsystem.
//! - [`Server`] / [`ServerBuilder`] for assembling and running a
//!   `Server` lifecycle.
//! - Re-exports of every core entity type so consumers can `use
//!   loon_sdk::*;` and have the full domain surface available.
//! - A [`MATCH_ALWAYS`] constant for guideline `add_always` flows.

pub mod error;
pub mod server;
pub mod tags;
pub mod tool_context;
pub mod variable;

pub use error::*;
pub use server::*;
pub use tags::*;
pub use tool_context::*;
pub use variable::*;

// Re-export core types as SDK handles so downstream users can write
// `use loon_sdk::*;` and reach every entity. The module-level glob
// already covers the entity modules; the explicit `pub use`s below
// document which entity surfaces are considered the public SDK
// contract (and let us add aliases later without touching the glob).
pub use loon_core::agent as agent_handle;
pub use loon_core::canned_response as canned_response_handle;
pub use loon_core::capability as capability_handle;
pub use loon_core::customer as customer_handle;
pub use loon_core::glossary as glossary_handle;
pub use loon_core::guideline as guideline_handle;
pub use loon_core::journey as journey_handle;
pub use loon_core::observation as observation_handle;
pub use loon_core::relationship as relationship_handle;
pub use loon_core::session as session_handle;
pub use loon_core::shot as shot_handle;
pub use loon_core::tag as tag_handle;
pub use loon_core::tool as tool_handle;
pub use loon_core::variable as context_variables;
pub use loon_core::*;

/// Built-in guideline matcher constant: the well-known "always"
/// token used by `GuidelineMatcher` strategies to mark a guideline
/// that should run on every turn.
pub const MATCH_ALWAYS: &str = "always";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn match_always_constant_value() {
        assert_eq!(MATCH_ALWAYS, "always");
    }

    #[test]
    fn reexports_core_types_accessible() {
        // Spot-check that the major entity types are reachable.
        let _: AgentId = AgentId::new();
        let _: GuidelineId = GuidelineId::new();
        let _: JourneyId = JourneyId::new();
        let _: SessionId = SessionId::new();
        let _: ToolId = ToolId::new();
        let _: CustomerId = CustomerId::new();
        let _: CannedResponseId = CannedResponseId::new();
        let _: TagId = TagId::new();
        let _: RelationshipId = RelationshipId::new();
        let _: CapabilityId = CapabilityId::new();
        // Variable type is reachable through both paths.
        let _: ContextVariable = ContextVariable {
            id: ContextVariableId::new(),
            agent_id: AgentId::new(),
            key: "ctx".into(),
            freshness_rules: vec![],
            tags: vec![],
            creation_utc: chrono::Utc::now(),
        };
        // Alias modules are reachable too.
        let _: &str = std::any::type_name::<context_variables::ContextVariable>();
    }

    #[test]
    fn sdk_error_is_constructible() {
        let e = SdkError::Validation("bad".into());
        assert!(e.to_string().contains("validation"));
    }
}

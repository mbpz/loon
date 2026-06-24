use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;

use crate::journey_guideline_projection::JourneyGuidelineProjection;
use crate::stores::{
    AgentStore, CannedResponseStore, CapabilityStore, ContextVariableStore, CustomerStore,
    EvaluationStore, GlossaryStore, GuidelineStore, GuidelineToolAssociationStore, JourneyStore,
    RelationshipStore, RetrieverStore, SessionStore, ShotStore, TagStore, ToolStore,
};
use crate::{
    Agent, AgentId, CannedResponse, Capability, ContextVariable, ContextVariableId,
    ContextVariableValue, CoreError, CoreResult, Customer, CustomerId, Event, Guideline,
    GuidelineId, Journey, JsonValue, Session, SessionId, SessionUpdateParams, Term, ToolInsights,
};

/// Read-side: domain-aware query methods backed by individual stores.
pub struct EntityQueries {
    pub agent_store: Arc<dyn AgentStore>,
    pub session_store: Arc<dyn SessionStore>,
    pub guideline_store: Arc<dyn GuidelineStore>,
    pub customer_store: Arc<dyn CustomerStore>,
    pub context_variable_store: Arc<dyn ContextVariableStore>,
    pub relationship_store: Arc<dyn RelationshipStore>,
    pub guideline_tool_association_store: Arc<dyn GuidelineToolAssociationStore>,
    pub glossary_store: Arc<dyn GlossaryStore>,
    pub journey_store: Arc<dyn JourneyStore>,
    pub canned_response_store: Arc<dyn CannedResponseStore>,
    pub capability_store: Arc<dyn CapabilityStore>,
    pub retriever_store: Arc<dyn RetrieverStore>,
    pub tool_store: Arc<dyn ToolStore>,
    pub evaluation_store: Arc<dyn EvaluationStore>,
    pub tag_store: Arc<dyn TagStore>,
    pub shot_store: Arc<dyn ShotStore>,
    pub journey_guideline_projection: Arc<JourneyGuidelineProjection>,
}

impl EntityQueries {
    pub async fn read_agent(&self, id: &AgentId) -> CoreResult<Agent> {
        self.agent_store
            .read(id)
            .await?
            .ok_or_else(|| CoreError::NotFound(crate::UniqueId(id.0.clone())))
    }

    pub async fn read_session(&self, id: &SessionId) -> CoreResult<Session> {
        self.session_store
            .read(id)
            .await?
            .ok_or_else(|| CoreError::NotFound(crate::UniqueId(id.0.clone())))
    }

    pub async fn read_customer(&self, id: &CustomerId) -> CoreResult<Customer> {
        self.customer_store
            .read(id)
            .await?
            .ok_or_else(|| CoreError::NotFound(crate::UniqueId(id.0.clone())))
    }

    pub async fn find_events(&self, session_id: &SessionId) -> CoreResult<Vec<Event>> {
        self.session_store.find_events(session_id).await
    }

    pub async fn find_guidelines_for_context(
        &self,
        agent_id: &AgentId,
        _journeys: &[Journey],
    ) -> CoreResult<Vec<Guideline>> {
        self.guideline_store.list(agent_id, &[]).await
    }

    pub async fn find_context_variables_for_context(
        &self,
        agent_id: &AgentId,
    ) -> CoreResult<Vec<ContextVariable>> {
        self.context_variable_store.list(agent_id).await
    }

    pub async fn find_capabilities_for_agent(
        &self,
        agent_id: &AgentId,
        _query: &str,
        _max: usize,
    ) -> CoreResult<Vec<Capability>> {
        self.capability_store.list(agent_id).await
    }

    pub async fn find_glossary_terms_for_context(
        &self,
        agent_id: &AgentId,
        _query: &str,
    ) -> CoreResult<Vec<Term>> {
        self.glossary_store.list_terms(agent_id).await
    }

    pub async fn find_journeys_for_context(&self, agent_id: &AgentId) -> CoreResult<Vec<Journey>> {
        self.journey_store.list(agent_id).await
    }

    pub async fn find_canned_responses_for_context(
        &self,
        agent: &Agent,
        _journeys: &[Journey],
        _guidelines: &[Guideline],
    ) -> CoreResult<Vec<CannedResponse>> {
        self.canned_response_store.list(&agent.id).await
    }

    pub async fn find_guidelines_that_need_reevaluation(
        &self,
        _available: &HashMap<GuidelineId, Guideline>,
        _journeys: &[Journey],
        _insights: &ToolInsights,
    ) -> CoreResult<Vec<Guideline>> {
        Ok(vec![])
    }

    pub async fn find_journey_related_guidelines(
        &self,
        _journey: &Journey,
    ) -> CoreResult<Vec<GuidelineId>> {
        Ok(vec![])
    }

    /// Build an [`EntityQueries`] instance backed entirely by
    /// in-memory stores (`InMemory*Store`). Used as the default
    /// wiring when no persistent store is provided — quick-start
    /// examples, integration tests, and the SDK's default
    /// [`crate::ServerBuilder::build`] all rely on this.
    pub fn in_memory() -> Arc<Self> {
        use crate::stores::{
            InMemoryAgentStore, InMemoryCannedResponseStore, InMemoryCapabilityStore,
            InMemoryContextVariableStore, InMemoryCustomerStore, InMemoryEvaluationStore,
            InMemoryGlossaryStore, InMemoryGuidelineStore, InMemoryGuidelineToolAssociationStore,
            InMemoryJourneyStore, InMemoryRelationshipStore, InMemoryRetrieverStore,
            InMemorySessionStore, InMemoryShotStore, InMemoryTagStore, InMemoryToolStore,
        };

        let agent_store: Arc<dyn AgentStore> = Arc::new(InMemoryAgentStore::new());
        let session_store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
        let guideline_store: Arc<dyn GuidelineStore> = Arc::new(InMemoryGuidelineStore::new());
        let customer_store: Arc<dyn CustomerStore> = Arc::new(InMemoryCustomerStore::new());
        let context_variable_store: Arc<dyn ContextVariableStore> =
            Arc::new(InMemoryContextVariableStore::new());
        let relationship_store: Arc<dyn RelationshipStore> =
            Arc::new(InMemoryRelationshipStore::new());
        let guideline_tool_association_store: Arc<dyn GuidelineToolAssociationStore> =
            Arc::new(InMemoryGuidelineToolAssociationStore::new());
        let glossary_store: Arc<dyn GlossaryStore> = Arc::new(InMemoryGlossaryStore::new());
        let journey_store: Arc<dyn JourneyStore> = Arc::new(InMemoryJourneyStore::new());
        let canned_response_store: Arc<dyn CannedResponseStore> =
            Arc::new(InMemoryCannedResponseStore::new());
        let capability_store: Arc<dyn CapabilityStore> = Arc::new(InMemoryCapabilityStore::new());
        let retriever_store: Arc<dyn RetrieverStore> = Arc::new(InMemoryRetrieverStore::new());
        let tool_store: Arc<dyn ToolStore> = Arc::new(InMemoryToolStore::new());
        let evaluation_store: Arc<dyn EvaluationStore> = Arc::new(InMemoryEvaluationStore::new());
        let tag_store: Arc<dyn TagStore> = Arc::new(InMemoryTagStore::new());
        let shot_store: Arc<dyn ShotStore> = Arc::new(InMemoryShotStore::new());
        let projection = Arc::new(JourneyGuidelineProjection {
            journey_store: journey_store.clone(),
            guideline_store: guideline_store.clone(),
        });
        Arc::new(Self {
            agent_store,
            session_store,
            guideline_store,
            customer_store,
            context_variable_store,
            relationship_store,
            guideline_tool_association_store,
            glossary_store,
            journey_store,
            canned_response_store,
            capability_store,
            retriever_store,
            tool_store,
            evaluation_store,
            tag_store,
            shot_store,
            journey_guideline_projection: projection,
        })
    }

    /// Build an [`EntityQueries`] instance backed by a
    /// [`loon_persistence::DocumentDatabaseHandle`]. Opens one named
    /// collection per entity kind (`"agents"`, `"sessions"`, …); the
    /// per-session event sub-collections (`"events_{sid}"`) are
    /// opened lazily by `SessionStore::create_event`. Use this from
    /// `ServerBuilder::with_document_db` when wiring real
    /// persistence — `in_memory()` stays the default for quick-start
    /// examples and tests.
    pub async fn from_document_database(
        handle: Arc<dyn loon_persistence::DocumentDatabaseHandle>,
    ) -> Result<Arc<Self>, loon_persistence::PersistenceError> {
        use crate::stores::{
            DocumentBackedAgentStore, DocumentBackedCannedResponseStore,
            DocumentBackedCapabilityStore, DocumentBackedContextVariableStore,
            DocumentBackedCustomerStore, DocumentBackedEvaluationStore,
            DocumentBackedGlossaryStore, DocumentBackedGuidelineStore,
            DocumentBackedGuidelineToolAssociationStore, DocumentBackedJourneyStore,
            DocumentBackedRelationshipStore, DocumentBackedRetrieverStore,
            DocumentBackedSessionStore, DocumentBackedShotStore, DocumentBackedTagStore,
            DocumentBackedToolStore,
        };

        let agent_store: Arc<dyn AgentStore> =
            Arc::new(DocumentBackedAgentStore::new(handle.collection("agents").await?));
        let session_store: Arc<dyn SessionStore> = Arc::new(DocumentBackedSessionStore::new(
            handle.collection("sessions").await?,
            handle.clone(),
        ));
        let guideline_store: Arc<dyn GuidelineStore> = Arc::new(
            DocumentBackedGuidelineStore::new(handle.collection("guidelines").await?),
        );
        let customer_store: Arc<dyn CustomerStore> = Arc::new(DocumentBackedCustomerStore::new(
            handle.collection("customers").await?,
        ));
        let context_variable_store: Arc<dyn ContextVariableStore> =
            Arc::new(DocumentBackedContextVariableStore::new(
                handle.collection("context_variables").await?,
                handle.collection("context_variable_values").await?,
            ));
        let relationship_store: Arc<dyn RelationshipStore> = Arc::new(
            DocumentBackedRelationshipStore::new(handle.collection("relationships").await?),
        );
        let guideline_tool_association_store: Arc<dyn GuidelineToolAssociationStore> =
            Arc::new(DocumentBackedGuidelineToolAssociationStore::new(
                handle.collection("guideline_tool_associations").await?,
            ));
        let glossary_store: Arc<dyn GlossaryStore> = Arc::new(DocumentBackedGlossaryStore::new(
            handle.collection("glossary").await?,
        ));
        let journey_store: Arc<dyn JourneyStore> = Arc::new(DocumentBackedJourneyStore::new(
            handle.collection("journeys").await?,
        ));
        let canned_response_store: Arc<dyn CannedResponseStore> = Arc::new(
            DocumentBackedCannedResponseStore::new(handle.collection("canned_responses").await?),
        );
        let capability_store: Arc<dyn CapabilityStore> = Arc::new(
            DocumentBackedCapabilityStore::new(handle.collection("capabilities").await?),
        );
        let retriever_store: Arc<dyn RetrieverStore> = Arc::new(DocumentBackedRetrieverStore::new(
            handle.collection("retrievers").await?,
        ));
        let tool_store: Arc<dyn ToolStore> =
            Arc::new(DocumentBackedToolStore::new(handle.collection("tools").await?));
        let evaluation_store: Arc<dyn EvaluationStore> = Arc::new(
            DocumentBackedEvaluationStore::new(handle.collection("evaluations").await?),
        );
        let tag_store: Arc<dyn TagStore> =
            Arc::new(DocumentBackedTagStore::new(handle.collection("tags").await?));
        let shot_store: Arc<dyn ShotStore> =
            Arc::new(DocumentBackedShotStore::new(handle.collection("shots").await?));

        let projection = Arc::new(JourneyGuidelineProjection {
            journey_store: journey_store.clone(),
            guideline_store: guideline_store.clone(),
        });
        Ok(Arc::new(Self {
            agent_store,
            session_store,
            guideline_store,
            customer_store,
            context_variable_store,
            relationship_store,
            guideline_tool_association_store,
            glossary_store,
            journey_store,
            canned_response_store,
            capability_store,
            retriever_store,
            tool_store,
            evaluation_store,
            tag_store,
            shot_store,
            journey_guideline_projection: projection,
        }))
    }
}

/// Write-side: command methods backed by individual stores.
pub struct EntityCommands {
    pub session_store: Arc<dyn SessionStore>,
    pub context_variable_store: Arc<dyn ContextVariableStore>,
}

impl EntityCommands {
    /// Update a session's mutable fields and return the freshly
    /// persisted [`Session`]. Returning the updated entity lets
    /// route handlers and tool-context callers avoid a second
    /// `session_store.read` after every patch — the underlying
    /// store already builds the post-write snapshot, so forwarding
    /// it is free.
    pub async fn update_session(
        &self,
        id: &SessionId,
        p: SessionUpdateParams,
    ) -> CoreResult<Session> {
        self.session_store.update(id, p).await
    }

    pub async fn update_context_variable_value(
        &self,
        var_id: &ContextVariableId,
        key: &str,
        data: JsonValue,
    ) -> CoreResult<ContextVariableValue> {
        self.context_variable_store
            .upsert_value(var_id, key, data)
            .await
    }

    pub async fn upsert_session_labels(
        &self,
        id: &SessionId,
        labels: HashSet<String>,
    ) -> CoreResult<Session> {
        self.session_store
            .update(
                id,
                SessionUpdateParams {
                    labels: Some(labels),
                    ..Default::default()
                },
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn session_update_params_can_carry_labels() {
        let mut labels = HashSet::new();
        labels.insert("vip".to_string());
        let p = SessionUpdateParams {
            labels: Some(labels.clone()),
            ..Default::default()
        };
        assert_eq!(p.labels.unwrap(), labels);
    }

    #[tokio::test]
    async fn in_memory_queries_round_trip_agent() {
        let q = EntityQueries::in_memory();
        let agent = Agent::new("test", "x");
        let id = agent.id.clone();
        q.agent_store.create(agent).await.unwrap();
        let loaded = q.read_agent(&id).await.unwrap();
        assert_eq!(loaded.name, "test");
    }

    /// `from_document_database` builds an [`EntityQueries`] whose
    /// stores round-trip through the underlying on-disk database.
    /// Data written via the queries graph must survive opening a
    /// fresh handle pointed at the same directory — proving the
    /// factory wires the document-backed stores rather than silently
    /// falling back to in-memory.
    #[tokio::test]
    async fn from_document_database_persists_across_handles() {
        use loon_persistence::backends::json_file::JsonFileDocumentDatabase;
        use loon_persistence::DocumentDatabaseHandle;
        use std::time::Duration;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().to_path_buf();

        // First handle: create + persist an agent.
        let id = {
            let db: Arc<dyn DocumentDatabaseHandle> = Arc::new(
                JsonFileDocumentDatabase::new(&path, Duration::from_millis(50)).unwrap(),
            );
            let queries = EntityQueries::from_document_database(db).await.unwrap();
            let agent = Agent::new("persisted", "y");
            let id = agent.id.clone();
            queries.agent_store.create(agent).await.unwrap();
            id
        };

        // Second handle on the same directory: agent must still be readable.
        let db: Arc<dyn DocumentDatabaseHandle> = Arc::new(
            JsonFileDocumentDatabase::new(&path, Duration::from_millis(50)).unwrap(),
        );
        let queries = EntityQueries::from_document_database(db).await.unwrap();
        let agent = queries.agent_store.read(&id).await.unwrap();
        assert!(
            agent.is_some(),
            "agent created via document-backed queries must persist across handle re-open"
        );
        assert_eq!(agent.unwrap().name, "persisted");
    }
}

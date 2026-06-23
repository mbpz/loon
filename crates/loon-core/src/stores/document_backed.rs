//! Document-backed implementations of the 16 Store traits.
//!
//! Each store wraps an `Arc<dyn DocumentCollectionHandle>` from
//! `loon-persistence`. Entities are serialized to `serde_json::Value`
//! on write and deserialized on read, with filters expressed at the
//! `BaseDocument` field level (`id`, `agent_id`, etc.). The backing
//! handle is responsible for atomic writes / cache invalidation /
//! disk persistence.
//!
//! Construction is via `EntityQueries::from_document_database`
//! (declared in `crate::entity_cq`); each store opens its own named
//! collection on the same handle (`"agents"`, `"sessions"`, …).
//!
//! For the `SessionStore` event sub-resource, events go in a separate
//! per-session collection named `"events_{session_id}"`.

use std::sync::Arc;

use async_trait::async_trait;
use loon_persistence::{DocumentCollectionHandle, DocumentDatabaseHandle, DocumentFilter};
use serde_json::{json, Value as JsonValue};

use crate::stores::{
    AgentStore, CannedResponseStore, CapabilityStore, ContextVariableStore, CustomerStore,
    EvaluationStore, GlossaryStore, GuidelineStore, GuidelineToolAssociationStore, JourneyStore,
    RelationshipStore, RetrieverStore, SessionStore, ShotStore, TagStore, ToolStore,
};
use crate::{
    Agent, AgentId, AgentUpdateParams, CannedResponse, CannedResponseId,
    CannedResponseUpdateParams, Capability, CapabilityId, CapabilityUpdateParams, ContextVariable,
    ContextVariableId, ContextVariableUpdateParams, ContextVariableValue, CoreError, CoreResult,
    Customer, CustomerId, CustomerUpdateParams, EvaluationId, Event, EventId, EventUpdateParams,
    GlossaryTermId, Guideline, GuidelineId, GuidelineToolAssociation, GuidelineToolAssociationId,
    GuidelineUpdateParams, Journey, JourneyId, JourneyUpdateParams, JsonValue as CoreJsonValue,
    Observation, ObservationUpdateParams, Relationship, RelationshipEntity, RelationshipId,
    Retriever, RetrieverId, Session, SessionId, SessionUpdateParams, Shot, ShotId, Tag, TagId,
    TagUpdateParams, Term, Tool, ToolId, ToolUpdateParams, UniqueId,
};

// ---------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------

fn id_filter(field: &str, id: &str) -> DocumentFilter {
    DocumentFilter::Eq {
        field: field.into(),
        value: json!(id),
    }
}

fn to_value<T: serde::Serialize>(entity: &T) -> CoreResult<JsonValue> {
    serde_json::to_value(entity)
        .map_err(|e| CoreError::Internal(format!("serialize: {e}")))
}

fn from_value<T: serde::de::DeserializeOwned>(value: JsonValue) -> CoreResult<T> {
    serde_json::from_value::<T>(value)
        .map_err(|e| CoreError::Internal(format!("deserialize: {e}")))
}

fn persistence_to_core(err: loon_persistence::PersistenceError) -> CoreError {
    CoreError::Internal(format!("persistence: {err}"))
}

fn all_filter() -> DocumentFilter {
    DocumentFilter::And(vec![])
}

/// Parlcant-style AND-semantics tag filter — `requested.is_empty() ⇒
/// no filter; non-empty ⇒ entity must carry every requested tag`.
/// Operates on a `Vec<TagId>` decoded from the entity JSON.
fn entity_has_all_tags(entity: &JsonValue, requested: &[TagId]) -> bool {
    if requested.is_empty() {
        return true;
    }
    let owned: Vec<String> = entity
        .get("tags")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    requested.iter().all(|t| owned.contains(&t.0))
}

// ---------------------------------------------------------------------
// 1. AgentStore
// ---------------------------------------------------------------------

pub struct DocumentBackedAgentStore {
    handle: Arc<dyn DocumentCollectionHandle>,
}

impl DocumentBackedAgentStore {
    pub fn new(handle: Arc<dyn DocumentCollectionHandle>) -> Self {
        Self { handle }
    }
}

#[async_trait]
impl AgentStore for DocumentBackedAgentStore {
    async fn create(&self, a: Agent) -> CoreResult<Agent> {
        let v = to_value(&a)?;
        self.handle.insert_one(v).await.map_err(persistence_to_core)?;
        Ok(a)
    }
    async fn read(&self, id: &AgentId) -> CoreResult<Option<Agent>> {
        let f = id_filter("id", id.as_str());
        match self.handle.find_one(&f).await.map_err(persistence_to_core)? {
            Some(v) => Ok(Some(from_value(v)?)),
            None => Ok(None),
        }
    }
    async fn update(&self, id: &AgentId, p: AgentUpdateParams) -> CoreResult<Agent> {
        let f = id_filter("id", id.as_str());
        let v = self
            .handle
            .find_one(&f)
            .await
            .map_err(persistence_to_core)?
            .ok_or_else(|| CoreError::NotFound(UniqueId(id.0.clone())))?;
        let mut a: Agent = from_value(v)?;
        if let Some(n) = p.name {
            a.name = n;
        }
        if let Some(desc) = p.description {
            a.description = desc;
        }
        if let Some(cm) = p.composition_mode {
            a.composition_mode = cm;
        }
        if let Some(mom) = p.message_output_mode {
            a.message_output_mode = mom;
        }
        if let Some(t) = p.tags {
            a.tags = t;
        }
        if let Some(m) = p.metadata {
            a.metadata = m;
        }
        let new_v = to_value(&a)?;
        // Replace by delete + insert (the handle's update_one only supports Set/Inc on one field).
        self.handle.delete_one(&f).await.map_err(persistence_to_core)?;
        self.handle.insert_one(new_v).await.map_err(persistence_to_core)?;
        Ok(a)
    }
    async fn delete(&self, id: &AgentId) -> CoreResult<()> {
        let f = id_filter("id", id.as_str());
        self.handle.delete_one(&f).await.map_err(persistence_to_core)?;
        Ok(())
    }
    async fn list(&self, tags: &[TagId]) -> CoreResult<Vec<Agent>> {
        let docs = self
            .handle
            .find(&all_filter())
            .await
            .map_err(persistence_to_core)?;
        let mut out = Vec::new();
        for v in docs {
            if entity_has_all_tags(&v, tags) {
                out.push(from_value::<Agent>(v)?);
            }
        }
        Ok(out)
    }
}

// ---------------------------------------------------------------------
// 2. SessionStore (with per-session event sub-collection)
// ---------------------------------------------------------------------

pub struct DocumentBackedSessionStore {
    sessions: Arc<dyn DocumentCollectionHandle>,
    /// Opener for `events_{session_id}` sub-collections. Stored as a
    /// closure-like trait object so the store can open new event
    /// collections on demand without keeping a `Database` reference.
    db: Arc<dyn DocumentDatabaseHandle>,
}

impl DocumentBackedSessionStore {
    pub fn new(
        sessions: Arc<dyn DocumentCollectionHandle>,
        db: Arc<dyn DocumentDatabaseHandle>,
    ) -> Self {
        Self { sessions, db }
    }

    async fn events_collection(
        &self,
        sid: &SessionId,
    ) -> CoreResult<Arc<dyn DocumentCollectionHandle>> {
        self.db
            .collection(&format!("events_{}", sid.as_str()))
            .await
            .map_err(persistence_to_core)
    }
}

#[async_trait]
impl SessionStore for DocumentBackedSessionStore {
    async fn create(&self, s: Session) -> CoreResult<Session> {
        self.sessions
            .insert_one(to_value(&s)?)
            .await
            .map_err(persistence_to_core)?;
        Ok(s)
    }
    async fn read(&self, id: &SessionId) -> CoreResult<Option<Session>> {
        let f = id_filter("id", id.as_str());
        match self.sessions.find_one(&f).await.map_err(persistence_to_core)? {
            Some(v) => Ok(Some(from_value(v)?)),
            None => Ok(None),
        }
    }
    async fn update(&self, id: &SessionId, p: SessionUpdateParams) -> CoreResult<Session> {
        let f = id_filter("id", id.as_str());
        let v = self
            .sessions
            .find_one(&f)
            .await
            .map_err(persistence_to_core)?
            .ok_or_else(|| CoreError::NotFound(UniqueId(id.0.clone())))?;
        let mut s: Session = from_value(v)?;
        if let Some(t) = p.title {
            s.title = Some(t);
        }
        if let Some(m) = p.mode {
            s.mode = m;
        }
        if let Some(l) = p.labels {
            s.labels = l;
        }
        self.sessions.delete_one(&f).await.map_err(persistence_to_core)?;
        self.sessions
            .insert_one(to_value(&s)?)
            .await
            .map_err(persistence_to_core)?;
        Ok(s)
    }
    async fn delete(&self, id: &SessionId) -> CoreResult<()> {
        let f = id_filter("id", id.as_str());
        self.sessions
            .delete_one(&f)
            .await
            .map_err(persistence_to_core)?;
        // Best-effort: drop the events sub-collection's documents too.
        if let Ok(events) = self.events_collection(id).await {
            let all = events
                .find(&all_filter())
                .await
                .map_err(persistence_to_core)?;
            for ev in all {
                if let Some(eid) = ev.get("id").and_then(|v| v.as_str()) {
                    let _ = events.delete_one(&id_filter("id", eid)).await;
                }
            }
        }
        Ok(())
    }
    async fn list(
        &self,
        agent_id: Option<&AgentId>,
        customer_id: Option<&CustomerId>,
    ) -> CoreResult<Vec<Session>> {
        let docs = self
            .sessions
            .find(&all_filter())
            .await
            .map_err(persistence_to_core)?;
        let mut out = Vec::new();
        for v in docs {
            let s: Session = from_value(v)?;
            if agent_id.map(|a| &s.agent_id != a).unwrap_or(false) {
                continue;
            }
            if customer_id
                .map(|c| s.customer_id.as_ref() != Some(c))
                .unwrap_or(false)
            {
                continue;
            }
            out.push(s);
        }
        Ok(out)
    }
    async fn create_event(&self, sid: SessionId, e: Event) -> CoreResult<Event> {
        let events = self.events_collection(&sid).await?;
        events
            .insert_one(to_value(&e)?)
            .await
            .map_err(persistence_to_core)?;
        Ok(e)
    }
    async fn update_event(
        &self,
        sid: &SessionId,
        eid: &EventId,
        p: EventUpdateParams,
    ) -> CoreResult<Event> {
        let events = self.events_collection(sid).await?;
        let f = id_filter("id", eid.as_str());
        let v = events
            .find_one(&f)
            .await
            .map_err(persistence_to_core)?
            .ok_or_else(|| CoreError::NotFound(UniqueId(eid.0.clone())))?;
        let mut e: Event = from_value(v)?;
        if let Some(data) = p.data {
            e.data = data;
        }
        if let Some(m) = p.metadata {
            e.metadata = Some(m);
        }
        events.delete_one(&f).await.map_err(persistence_to_core)?;
        events
            .insert_one(to_value(&e)?)
            .await
            .map_err(persistence_to_core)?;
        Ok(e)
    }
    async fn read_events(&self, sid: &SessionId) -> CoreResult<Vec<Event>> {
        let events = self.events_collection(sid).await?;
        let docs = events
            .find(&all_filter())
            .await
            .map_err(persistence_to_core)?;
        let mut out: Vec<Event> = Vec::with_capacity(docs.len());
        for v in docs {
            out.push(from_value(v)?);
        }
        out.sort_by_key(|e| e.creation_utc);
        Ok(out)
    }
    async fn find_events(&self, sid: &SessionId) -> CoreResult<Vec<Event>> {
        self.read_events(sid).await
    }
}

// ---------------------------------------------------------------------
// 3. GuidelineStore
// ---------------------------------------------------------------------

pub struct DocumentBackedGuidelineStore {
    handle: Arc<dyn DocumentCollectionHandle>,
}

impl DocumentBackedGuidelineStore {
    pub fn new(handle: Arc<dyn DocumentCollectionHandle>) -> Self {
        Self { handle }
    }
}

#[async_trait]
impl GuidelineStore for DocumentBackedGuidelineStore {
    async fn create(&self, g: Guideline) -> CoreResult<Guideline> {
        self.handle
            .insert_one(to_value(&g)?)
            .await
            .map_err(persistence_to_core)?;
        Ok(g)
    }
    async fn read(&self, id: &GuidelineId) -> CoreResult<Option<Guideline>> {
        let f = id_filter("id", id.as_str());
        match self.handle.find_one(&f).await.map_err(persistence_to_core)? {
            Some(v) => Ok(Some(from_value(v)?)),
            None => Ok(None),
        }
    }
    async fn update(
        &self,
        id: &GuidelineId,
        p: GuidelineUpdateParams,
    ) -> CoreResult<Guideline> {
        let f = id_filter("id", id.as_str());
        let v = self
            .handle
            .find_one(&f)
            .await
            .map_err(persistence_to_core)?
            .ok_or_else(|| CoreError::NotFound(UniqueId(id.0.clone())))?;
        let mut g: Guideline = from_value(v)?;
        if let Some(c) = p.condition {
            g.content.condition = c;
        }
        if let Some(a) = p.action {
            g.content.action = a;
        }
        if let Some(e) = p.enabled {
            g.enabled = e;
        }
        self.handle.delete_one(&f).await.map_err(persistence_to_core)?;
        self.handle
            .insert_one(to_value(&g)?)
            .await
            .map_err(persistence_to_core)?;
        Ok(g)
    }
    async fn delete(&self, id: &GuidelineId) -> CoreResult<()> {
        let f = id_filter("id", id.as_str());
        self.handle.delete_one(&f).await.map_err(persistence_to_core)?;
        Ok(())
    }
    async fn list(&self, agent_id: &AgentId, tags: &[TagId]) -> CoreResult<Vec<Guideline>> {
        let docs = self
            .handle
            .find(&id_filter("agent_id", agent_id.as_str()))
            .await
            .map_err(persistence_to_core)?;
        let mut out = Vec::new();
        for v in docs {
            if entity_has_all_tags(&v, tags) {
                out.push(from_value::<Guideline>(v)?);
            }
        }
        Ok(out)
    }
}

// ---------------------------------------------------------------------
// 4. JourneyStore
// ---------------------------------------------------------------------

pub struct DocumentBackedJourneyStore {
    handle: Arc<dyn DocumentCollectionHandle>,
}

impl DocumentBackedJourneyStore {
    pub fn new(handle: Arc<dyn DocumentCollectionHandle>) -> Self {
        Self { handle }
    }
}

#[async_trait]
impl JourneyStore for DocumentBackedJourneyStore {
    async fn create(&self, j: Journey) -> CoreResult<Journey> {
        self.handle
            .insert_one(to_value(&j)?)
            .await
            .map_err(persistence_to_core)?;
        Ok(j)
    }
    async fn read(&self, id: &JourneyId) -> CoreResult<Option<Journey>> {
        let f = id_filter("id", id.as_str());
        match self.handle.find_one(&f).await.map_err(persistence_to_core)? {
            Some(v) => Ok(Some(from_value(v)?)),
            None => Ok(None),
        }
    }
    async fn update(&self, id: &JourneyId, p: JourneyUpdateParams) -> CoreResult<Journey> {
        let f = id_filter("id", id.as_str());
        let v = self
            .handle
            .find_one(&f)
            .await
            .map_err(persistence_to_core)?
            .ok_or_else(|| CoreError::NotFound(UniqueId(id.0.clone())))?;
        let mut j: Journey = from_value(v)?;
        if let Some(t) = p.title {
            j.title = t;
        }
        if let Some(desc) = p.description {
            j.description = desc;
        }
        self.handle.delete_one(&f).await.map_err(persistence_to_core)?;
        self.handle
            .insert_one(to_value(&j)?)
            .await
            .map_err(persistence_to_core)?;
        Ok(j)
    }
    async fn delete(&self, id: &JourneyId) -> CoreResult<()> {
        let f = id_filter("id", id.as_str());
        self.handle.delete_one(&f).await.map_err(persistence_to_core)?;
        Ok(())
    }
    async fn list(&self, agent_id: &AgentId) -> CoreResult<Vec<Journey>> {
        let docs = self
            .handle
            .find(&id_filter("agent_id", agent_id.as_str()))
            .await
            .map_err(persistence_to_core)?;
        let mut out = Vec::with_capacity(docs.len());
        for v in docs {
            out.push(from_value(v)?);
        }
        Ok(out)
    }
}

// ---------------------------------------------------------------------
// 5. ToolStore
// ---------------------------------------------------------------------
//
// Tools don't carry an `agent_id` field on their own (the relationship
// is held by `Agent → tools` elsewhere), so `list(agent_id)` mirrors
// the in-memory store and returns an empty list here. Real
// per-agent registration would live in a separate `agent_tools`
// collection.

pub struct DocumentBackedToolStore {
    handle: Arc<dyn DocumentCollectionHandle>,
}

impl DocumentBackedToolStore {
    pub fn new(handle: Arc<dyn DocumentCollectionHandle>) -> Self {
        Self { handle }
    }
}

#[async_trait]
impl ToolStore for DocumentBackedToolStore {
    async fn create(&self, t: Tool) -> CoreResult<Tool> {
        self.handle
            .insert_one(to_value(&t)?)
            .await
            .map_err(persistence_to_core)?;
        Ok(t)
    }
    async fn read(&self, id: &ToolId) -> CoreResult<Option<Tool>> {
        let f = id_filter("id", id.as_str());
        match self.handle.find_one(&f).await.map_err(persistence_to_core)? {
            Some(v) => Ok(Some(from_value(v)?)),
            None => Ok(None),
        }
    }
    async fn update(&self, id: &ToolId, params: ToolUpdateParams) -> CoreResult<Tool> {
        let f = id_filter("id", id.as_str());
        let v = self
            .handle
            .find_one(&f)
            .await
            .map_err(persistence_to_core)?
            .ok_or_else(|| CoreError::NotFound(UniqueId(id.0.clone())))?;
        let mut t: Tool = from_value(v)?;
        if let Some(name) = params.name {
            t.name = name;
        }
        if let Some(description) = params.description {
            t.description = description;
        }
        if let Some(parameters_schema) = params.parameters_schema {
            t.parameters_schema = parameters_schema;
        }
        self.handle.delete_one(&f).await.map_err(persistence_to_core)?;
        self.handle
            .insert_one(to_value(&t)?)
            .await
            .map_err(persistence_to_core)?;
        Ok(t)
    }
    async fn delete(&self, id: &ToolId) -> CoreResult<()> {
        let f = id_filter("id", id.as_str());
        self.handle.delete_one(&f).await.map_err(persistence_to_core)?;
        Ok(())
    }
    async fn list(&self, _agent_id: &AgentId) -> CoreResult<Vec<Tool>> {
        // No `agent_id` field on `Tool`; per-agent registration is
        // expected to come from a separate collection in a later
        // phase. For now, return all tools so callers see what's
        // registered.
        let docs = self
            .handle
            .find(&all_filter())
            .await
            .map_err(persistence_to_core)?;
        let mut out = Vec::with_capacity(docs.len());
        for v in docs {
            out.push(from_value(v)?);
        }
        Ok(out)
    }
}

// ---------------------------------------------------------------------
// 6. EvaluationStore
// ---------------------------------------------------------------------

pub struct DocumentBackedEvaluationStore {
    handle: Arc<dyn DocumentCollectionHandle>,
}

impl DocumentBackedEvaluationStore {
    pub fn new(handle: Arc<dyn DocumentCollectionHandle>) -> Self {
        Self { handle }
    }
}

#[async_trait]
impl EvaluationStore for DocumentBackedEvaluationStore {
    async fn create(&self, e: Observation) -> CoreResult<Observation> {
        self.handle
            .insert_one(to_value(&e)?)
            .await
            .map_err(persistence_to_core)?;
        Ok(e)
    }
    async fn read(&self, id: &EvaluationId) -> CoreResult<Option<Observation>> {
        let f = id_filter("id", id.as_str());
        match self.handle.find_one(&f).await.map_err(persistence_to_core)? {
            Some(v) => Ok(Some(from_value(v)?)),
            None => Ok(None),
        }
    }
    async fn update(
        &self,
        id: &EvaluationId,
        params: ObservationUpdateParams,
    ) -> CoreResult<Observation> {
        let f = id_filter("id", id.as_str());
        let v = self
            .handle
            .find_one(&f)
            .await
            .map_err(persistence_to_core)?
            .ok_or_else(|| CoreError::NotFound(UniqueId(id.0.clone())))?;
        let mut o: Observation = from_value(v)?;
        if let Some(c) = params.condition {
            o.condition = c;
        }
        if let Some(t) = params.tools {
            o.tools = t;
        }
        if let Some(e) = params.enabled {
            o.enabled = e;
        }
        self.handle.delete_one(&f).await.map_err(persistence_to_core)?;
        self.handle
            .insert_one(to_value(&o)?)
            .await
            .map_err(persistence_to_core)?;
        Ok(o)
    }
    async fn delete(&self, id: &EvaluationId) -> CoreResult<()> {
        let f = id_filter("id", id.as_str());
        self.handle.delete_one(&f).await.map_err(persistence_to_core)?;
        Ok(())
    }
    async fn list(&self, agent_id: &AgentId) -> CoreResult<Vec<Observation>> {
        let docs = self
            .handle
            .find(&id_filter("agent_id", agent_id.as_str()))
            .await
            .map_err(persistence_to_core)?;
        let mut out = Vec::with_capacity(docs.len());
        for v in docs {
            out.push(from_value(v)?);
        }
        Ok(out)
    }
}

// ---------------------------------------------------------------------
// 7. CustomerStore
// ---------------------------------------------------------------------

pub struct DocumentBackedCustomerStore {
    handle: Arc<dyn DocumentCollectionHandle>,
}

impl DocumentBackedCustomerStore {
    pub fn new(handle: Arc<dyn DocumentCollectionHandle>) -> Self {
        Self { handle }
    }
}

#[async_trait]
impl CustomerStore for DocumentBackedCustomerStore {
    async fn create(&self, c: Customer) -> CoreResult<Customer> {
        self.handle
            .insert_one(to_value(&c)?)
            .await
            .map_err(persistence_to_core)?;
        Ok(c)
    }
    async fn read(&self, id: &CustomerId) -> CoreResult<Option<Customer>> {
        let f = id_filter("id", id.as_str());
        match self.handle.find_one(&f).await.map_err(persistence_to_core)? {
            Some(v) => Ok(Some(from_value(v)?)),
            None => Ok(None),
        }
    }
    async fn update(&self, id: &CustomerId, p: CustomerUpdateParams) -> CoreResult<Customer> {
        let f = id_filter("id", id.as_str());
        let v = self
            .handle
            .find_one(&f)
            .await
            .map_err(persistence_to_core)?
            .ok_or_else(|| CoreError::NotFound(UniqueId(id.0.clone())))?;
        let mut c: Customer = from_value(v)?;
        if let Some(n) = p.name {
            c.name = n;
        }
        if let Some(m) = p.metadata {
            c.metadata = m;
        }
        self.handle.delete_one(&f).await.map_err(persistence_to_core)?;
        self.handle
            .insert_one(to_value(&c)?)
            .await
            .map_err(persistence_to_core)?;
        Ok(c)
    }
    async fn delete(&self, id: &CustomerId) -> CoreResult<()> {
        let f = id_filter("id", id.as_str());
        self.handle.delete_one(&f).await.map_err(persistence_to_core)?;
        Ok(())
    }
    async fn list(&self, tags: &[TagId]) -> CoreResult<Vec<Customer>> {
        let docs = self
            .handle
            .find(&all_filter())
            .await
            .map_err(persistence_to_core)?;
        let mut out = Vec::new();
        for v in docs {
            if entity_has_all_tags(&v, tags) {
                out.push(from_value::<Customer>(v)?);
            }
        }
        Ok(out)
    }
}

// ---------------------------------------------------------------------
// 8. GlossaryStore
// ---------------------------------------------------------------------

pub struct DocumentBackedGlossaryStore {
    handle: Arc<dyn DocumentCollectionHandle>,
}

impl DocumentBackedGlossaryStore {
    pub fn new(handle: Arc<dyn DocumentCollectionHandle>) -> Self {
        Self { handle }
    }
}

#[async_trait]
impl GlossaryStore for DocumentBackedGlossaryStore {
    async fn create_term(&self, t: Term) -> CoreResult<Term> {
        self.handle
            .insert_one(to_value(&t)?)
            .await
            .map_err(persistence_to_core)?;
        Ok(t)
    }
    async fn read_term(&self, id: &GlossaryTermId) -> CoreResult<Option<Term>> {
        let f = id_filter("id", id.as_str());
        match self.handle.find_one(&f).await.map_err(persistence_to_core)? {
            Some(v) => Ok(Some(from_value(v)?)),
            None => Ok(None),
        }
    }
    async fn update_term(&self, id: &GlossaryTermId, t: Term) -> CoreResult<Term> {
        let f = id_filter("id", id.as_str());
        let existing = self
            .handle
            .find_one(&f)
            .await
            .map_err(persistence_to_core)?;
        if existing.is_none() {
            return Err(CoreError::NotFound(UniqueId(id.0.clone())));
        }
        self.handle.delete_one(&f).await.map_err(persistence_to_core)?;
        self.handle
            .insert_one(to_value(&t)?)
            .await
            .map_err(persistence_to_core)?;
        Ok(t)
    }
    async fn delete_term(&self, id: &GlossaryTermId) -> CoreResult<()> {
        let f = id_filter("id", id.as_str());
        self.handle.delete_one(&f).await.map_err(persistence_to_core)?;
        Ok(())
    }
    async fn list_terms(&self, _agent_id: &AgentId) -> CoreResult<Vec<Term>> {
        // `Term` carries no agent_id field; mirror the in-memory store
        // and return every term for now. Per-agent term registration
        // can come later via a separate `agent_terms` collection.
        let docs = self
            .handle
            .find(&all_filter())
            .await
            .map_err(persistence_to_core)?;
        let mut out = Vec::with_capacity(docs.len());
        for v in docs {
            out.push(from_value(v)?);
        }
        Ok(out)
    }
}

// ---------------------------------------------------------------------
// 9. ContextVariableStore (descriptors + values)
// ---------------------------------------------------------------------

pub struct DocumentBackedContextVariableStore {
    descriptors: Arc<dyn DocumentCollectionHandle>,
    values: Arc<dyn DocumentCollectionHandle>,
}

impl DocumentBackedContextVariableStore {
    pub fn new(
        descriptors: Arc<dyn DocumentCollectionHandle>,
        values: Arc<dyn DocumentCollectionHandle>,
    ) -> Self {
        Self {
            descriptors,
            values,
        }
    }
}

fn value_doc_id(var_id: &ContextVariableId, key: &str) -> String {
    format!("{}:{}", var_id.as_str(), key)
}

#[async_trait]
impl ContextVariableStore for DocumentBackedContextVariableStore {
    async fn create(&self, v: ContextVariable) -> CoreResult<ContextVariable> {
        self.descriptors
            .insert_one(to_value(&v)?)
            .await
            .map_err(persistence_to_core)?;
        Ok(v)
    }
    async fn read(&self, id: &ContextVariableId) -> CoreResult<Option<ContextVariable>> {
        let f = id_filter("id", id.as_str());
        match self.descriptors.find_one(&f).await.map_err(persistence_to_core)? {
            Some(v) => Ok(Some(from_value(v)?)),
            None => Ok(None),
        }
    }
    async fn update(
        &self,
        id: &ContextVariableId,
        p: ContextVariableUpdateParams,
    ) -> CoreResult<ContextVariable> {
        let f = id_filter("id", id.as_str());
        let v = self
            .descriptors
            .find_one(&f)
            .await
            .map_err(persistence_to_core)?
            .ok_or_else(|| CoreError::NotFound(UniqueId(id.0.clone())))?;
        let mut desc: ContextVariable = from_value(v)?;
        if let Some(k) = p.key {
            desc.key = k;
        }
        let var_id = desc.id.clone();
        let value_key = desc.key.clone();

        self.descriptors.delete_one(&f).await.map_err(persistence_to_core)?;
        self.descriptors
            .insert_one(to_value(&desc)?)
            .await
            .map_err(persistence_to_core)?;

        if let Some(payload) = p.data {
            self.upsert_value(&var_id, &value_key, payload).await?;
        }
        Ok(desc)
    }
    async fn delete(&self, id: &ContextVariableId) -> CoreResult<()> {
        let f = id_filter("id", id.as_str());
        self.descriptors.delete_one(&f).await.map_err(persistence_to_core)?;
        // Drop all per-(var_id, key) values too.
        let all = self
            .values
            .find(&id_filter("var_id", id.as_str()))
            .await
            .map_err(persistence_to_core)?;
        for v in all {
            if let Some(vid) = v.get("id").and_then(|x| x.as_str()) {
                let _ = self.values.delete_one(&id_filter("id", vid)).await;
            }
        }
        Ok(())
    }
    async fn list(&self, agent_id: &AgentId) -> CoreResult<Vec<ContextVariable>> {
        let docs = self
            .descriptors
            .find(&id_filter("agent_id", agent_id.as_str()))
            .await
            .map_err(persistence_to_core)?;
        let mut out = Vec::with_capacity(docs.len());
        for v in docs {
            out.push(from_value(v)?);
        }
        Ok(out)
    }
    async fn upsert_value(
        &self,
        var_id: &ContextVariableId,
        key: &str,
        data: CoreJsonValue,
    ) -> CoreResult<ContextVariableValue> {
        let value = ContextVariableValue {
            key: key.to_string(),
            data,
            last_updated: chrono::Utc::now(),
        };
        let doc_id = value_doc_id(var_id, key);
        // Wrap with var_id + composite id so we can list / delete by var_id.
        let mut as_json = to_value(&value)?;
        if let Some(obj) = as_json.as_object_mut() {
            obj.insert("id".into(), json!(doc_id.clone()));
            obj.insert("var_id".into(), json!(var_id.as_str()));
        }
        let f = id_filter("id", &doc_id);
        // Upsert = delete prior + insert.
        self.values.delete_one(&f).await.map_err(persistence_to_core)?;
        self.values
            .insert_one(as_json)
            .await
            .map_err(persistence_to_core)?;
        Ok(value)
    }
    async fn read_value(
        &self,
        var_id: &ContextVariableId,
        key: &str,
    ) -> CoreResult<Option<ContextVariableValue>> {
        let doc_id = value_doc_id(var_id, key);
        let f = id_filter("id", &doc_id);
        match self.values.find_one(&f).await.map_err(persistence_to_core)? {
            Some(mut v) => {
                if let Some(obj) = v.as_object_mut() {
                    obj.remove("id");
                    obj.remove("var_id");
                }
                Ok(Some(from_value(v)?))
            }
            None => Ok(None),
        }
    }
}

// ---------------------------------------------------------------------
// 10. CannedResponseStore
// ---------------------------------------------------------------------

pub struct DocumentBackedCannedResponseStore {
    handle: Arc<dyn DocumentCollectionHandle>,
}

impl DocumentBackedCannedResponseStore {
    pub fn new(handle: Arc<dyn DocumentCollectionHandle>) -> Self {
        Self { handle }
    }
}

#[async_trait]
impl CannedResponseStore for DocumentBackedCannedResponseStore {
    async fn create(&self, c: CannedResponse) -> CoreResult<CannedResponse> {
        self.handle
            .insert_one(to_value(&c)?)
            .await
            .map_err(persistence_to_core)?;
        Ok(c)
    }
    async fn read(&self, id: &CannedResponseId) -> CoreResult<Option<CannedResponse>> {
        let f = id_filter("id", id.as_str());
        match self.handle.find_one(&f).await.map_err(persistence_to_core)? {
            Some(v) => Ok(Some(from_value(v)?)),
            None => Ok(None),
        }
    }
    async fn update(
        &self,
        id: &CannedResponseId,
        p: CannedResponseUpdateParams,
    ) -> CoreResult<CannedResponse> {
        let f = id_filter("id", id.as_str());
        let v = self
            .handle
            .find_one(&f)
            .await
            .map_err(persistence_to_core)?
            .ok_or_else(|| CoreError::NotFound(UniqueId(id.0.clone())))?;
        let mut c: CannedResponse = from_value(v)?;
        if let Some(v) = p.value {
            c.value = v;
        }
        if let Some(m) = p.matchers {
            c.matchers = m;
        }
        self.handle.delete_one(&f).await.map_err(persistence_to_core)?;
        self.handle
            .insert_one(to_value(&c)?)
            .await
            .map_err(persistence_to_core)?;
        Ok(c)
    }
    async fn delete(&self, id: &CannedResponseId) -> CoreResult<()> {
        let f = id_filter("id", id.as_str());
        self.handle.delete_one(&f).await.map_err(persistence_to_core)?;
        Ok(())
    }
    async fn list(&self, agent_id: &AgentId) -> CoreResult<Vec<CannedResponse>> {
        let docs = self
            .handle
            .find(&id_filter("agent_id", agent_id.as_str()))
            .await
            .map_err(persistence_to_core)?;
        let mut out = Vec::with_capacity(docs.len());
        for v in docs {
            out.push(from_value(v)?);
        }
        Ok(out)
    }
}

// ---------------------------------------------------------------------
// 11. CapabilityStore
// ---------------------------------------------------------------------

pub struct DocumentBackedCapabilityStore {
    handle: Arc<dyn DocumentCollectionHandle>,
}

impl DocumentBackedCapabilityStore {
    pub fn new(handle: Arc<dyn DocumentCollectionHandle>) -> Self {
        Self { handle }
    }
}

#[async_trait]
impl CapabilityStore for DocumentBackedCapabilityStore {
    async fn create(&self, c: Capability) -> CoreResult<Capability> {
        self.handle
            .insert_one(to_value(&c)?)
            .await
            .map_err(persistence_to_core)?;
        Ok(c)
    }
    async fn read(&self, id: &CapabilityId) -> CoreResult<Option<Capability>> {
        let f = id_filter("id", id.as_str());
        match self.handle.find_one(&f).await.map_err(persistence_to_core)? {
            Some(v) => Ok(Some(from_value(v)?)),
            None => Ok(None),
        }
    }
    async fn update(&self, id: &CapabilityId, p: CapabilityUpdateParams) -> CoreResult<Capability> {
        let f = id_filter("id", id.as_str());
        let v = self
            .handle
            .find_one(&f)
            .await
            .map_err(persistence_to_core)?
            .ok_or_else(|| CoreError::NotFound(UniqueId(id.0.clone())))?;
        let mut c: Capability = from_value(v)?;
        if let Some(n) = p.name {
            c.name = n;
        }
        if let Some(desc) = p.description {
            c.description = desc;
        }
        self.handle.delete_one(&f).await.map_err(persistence_to_core)?;
        self.handle
            .insert_one(to_value(&c)?)
            .await
            .map_err(persistence_to_core)?;
        Ok(c)
    }
    async fn delete(&self, id: &CapabilityId) -> CoreResult<()> {
        let f = id_filter("id", id.as_str());
        self.handle.delete_one(&f).await.map_err(persistence_to_core)?;
        Ok(())
    }
    async fn list(&self, agent_id: &AgentId) -> CoreResult<Vec<Capability>> {
        let docs = self
            .handle
            .find(&id_filter("agent_id", agent_id.as_str()))
            .await
            .map_err(persistence_to_core)?;
        let mut out = Vec::with_capacity(docs.len());
        for v in docs {
            out.push(from_value(v)?);
        }
        Ok(out)
    }
}

// ---------------------------------------------------------------------
// 12. RetrieverStore
// ---------------------------------------------------------------------

pub struct DocumentBackedRetrieverStore {
    handle: Arc<dyn DocumentCollectionHandle>,
}

impl DocumentBackedRetrieverStore {
    pub fn new(handle: Arc<dyn DocumentCollectionHandle>) -> Self {
        Self { handle }
    }
}

#[async_trait]
impl RetrieverStore for DocumentBackedRetrieverStore {
    async fn create(&self, r: Retriever) -> CoreResult<Retriever> {
        self.handle
            .insert_one(to_value(&r)?)
            .await
            .map_err(persistence_to_core)?;
        Ok(r)
    }
    async fn read(&self, id: &RetrieverId) -> CoreResult<Option<Retriever>> {
        let f = id_filter("id", id.as_str());
        match self.handle.find_one(&f).await.map_err(persistence_to_core)? {
            Some(v) => Ok(Some(from_value(v)?)),
            None => Ok(None),
        }
    }
    async fn delete(&self, id: &RetrieverId) -> CoreResult<()> {
        let f = id_filter("id", id.as_str());
        self.handle.delete_one(&f).await.map_err(persistence_to_core)?;
        Ok(())
    }
    async fn list(&self, agent_id: &AgentId) -> CoreResult<Vec<Retriever>> {
        let docs = self
            .handle
            .find(&id_filter("agent_id", agent_id.as_str()))
            .await
            .map_err(persistence_to_core)?;
        let mut out = Vec::with_capacity(docs.len());
        for v in docs {
            out.push(from_value(v)?);
        }
        Ok(out)
    }
}

// ---------------------------------------------------------------------
// 13. TagStore
// ---------------------------------------------------------------------

pub struct DocumentBackedTagStore {
    handle: Arc<dyn DocumentCollectionHandle>,
}

impl DocumentBackedTagStore {
    pub fn new(handle: Arc<dyn DocumentCollectionHandle>) -> Self {
        Self { handle }
    }
}

#[async_trait]
impl TagStore for DocumentBackedTagStore {
    async fn create(&self, t: Tag) -> CoreResult<Tag> {
        self.handle
            .insert_one(to_value(&t)?)
            .await
            .map_err(persistence_to_core)?;
        Ok(t)
    }
    async fn read(&self, id: &TagId) -> CoreResult<Option<Tag>> {
        let f = id_filter("id", id.as_str());
        match self.handle.find_one(&f).await.map_err(persistence_to_core)? {
            Some(v) => Ok(Some(from_value(v)?)),
            None => Ok(None),
        }
    }
    async fn update(&self, id: &TagId, params: TagUpdateParams) -> CoreResult<Tag> {
        let f = id_filter("id", id.as_str());
        let v = self
            .handle
            .find_one(&f)
            .await
            .map_err(persistence_to_core)?
            .ok_or_else(|| CoreError::NotFound(UniqueId(id.0.clone())))?;
        let mut t: Tag = from_value(v)?;
        if let Some(name) = params.name {
            t.name = name;
        }
        self.handle.delete_one(&f).await.map_err(persistence_to_core)?;
        self.handle
            .insert_one(to_value(&t)?)
            .await
            .map_err(persistence_to_core)?;
        Ok(t)
    }
    async fn list(&self) -> CoreResult<Vec<Tag>> {
        let docs = self
            .handle
            .find(&all_filter())
            .await
            .map_err(persistence_to_core)?;
        let mut out = Vec::with_capacity(docs.len());
        for v in docs {
            out.push(from_value(v)?);
        }
        Ok(out)
    }
    async fn delete(&self, id: &TagId) -> CoreResult<()> {
        let f = id_filter("id", id.as_str());
        self.handle.delete_one(&f).await.map_err(persistence_to_core)?;
        Ok(())
    }
}

// ---------------------------------------------------------------------
// 14. RelationshipStore
// ---------------------------------------------------------------------

pub struct DocumentBackedRelationshipStore {
    handle: Arc<dyn DocumentCollectionHandle>,
}

impl DocumentBackedRelationshipStore {
    pub fn new(handle: Arc<dyn DocumentCollectionHandle>) -> Self {
        Self { handle }
    }
}

#[async_trait]
impl RelationshipStore for DocumentBackedRelationshipStore {
    async fn create(&self, r: Relationship) -> CoreResult<Relationship> {
        self.handle
            .insert_one(to_value(&r)?)
            .await
            .map_err(persistence_to_core)?;
        Ok(r)
    }
    async fn read(&self, id: &RelationshipId) -> CoreResult<Option<Relationship>> {
        let f = id_filter("id", id.as_str());
        match self.handle.find_one(&f).await.map_err(persistence_to_core)? {
            Some(v) => Ok(Some(from_value(v)?)),
            None => Ok(None),
        }
    }
    async fn delete(&self, id: &RelationshipId) -> CoreResult<()> {
        let f = id_filter("id", id.as_str());
        self.handle.delete_one(&f).await.map_err(persistence_to_core)?;
        Ok(())
    }
    async fn list_for(&self, entity: &RelationshipEntity) -> CoreResult<Vec<Relationship>> {
        // Filter at the in-memory level because RelationshipEntity is a
        // nested struct that the simple Eq filter would have to match
        // through `serde_json::Value` equality — relying on that for
        // future backends would couple us to serializer field ordering,
        // so we scan and check in code.
        let docs = self
            .handle
            .find(&all_filter())
            .await
            .map_err(persistence_to_core)?;
        let mut out = Vec::new();
        for v in docs {
            let r: Relationship = from_value(v)?;
            if &r.source == entity || &r.target == entity {
                out.push(r);
            }
        }
        Ok(out)
    }
}

// ---------------------------------------------------------------------
// 15. GuidelineToolAssociationStore
// ---------------------------------------------------------------------

pub struct DocumentBackedGuidelineToolAssociationStore {
    handle: Arc<dyn DocumentCollectionHandle>,
}

impl DocumentBackedGuidelineToolAssociationStore {
    pub fn new(handle: Arc<dyn DocumentCollectionHandle>) -> Self {
        Self { handle }
    }
}

#[async_trait]
impl GuidelineToolAssociationStore for DocumentBackedGuidelineToolAssociationStore {
    async fn create(&self, a: GuidelineToolAssociation) -> CoreResult<GuidelineToolAssociation> {
        self.handle
            .insert_one(to_value(&a)?)
            .await
            .map_err(persistence_to_core)?;
        Ok(a)
    }
    async fn read(
        &self,
        id: &GuidelineToolAssociationId,
    ) -> CoreResult<Option<GuidelineToolAssociation>> {
        let f = id_filter("id", id.as_str());
        match self.handle.find_one(&f).await.map_err(persistence_to_core)? {
            Some(v) => Ok(Some(from_value(v)?)),
            None => Ok(None),
        }
    }
    async fn delete(&self, id: &GuidelineToolAssociationId) -> CoreResult<()> {
        let f = id_filter("id", id.as_str());
        self.handle.delete_one(&f).await.map_err(persistence_to_core)?;
        Ok(())
    }
    async fn list_for_tool(
        &self,
        tool_id: &ToolId,
    ) -> CoreResult<Vec<GuidelineToolAssociation>> {
        let docs = self
            .handle
            .find(&id_filter("tool_id", tool_id.as_str()))
            .await
            .map_err(persistence_to_core)?;
        let mut out = Vec::with_capacity(docs.len());
        for v in docs {
            out.push(from_value(v)?);
        }
        Ok(out)
    }
    async fn list_for_guideline(
        &self,
        guideline_id: &GuidelineId,
    ) -> CoreResult<Vec<GuidelineToolAssociation>> {
        let docs = self
            .handle
            .find(&id_filter("guideline_id", guideline_id.as_str()))
            .await
            .map_err(persistence_to_core)?;
        let mut out = Vec::with_capacity(docs.len());
        for v in docs {
            out.push(from_value(v)?);
        }
        Ok(out)
    }
}

// ---------------------------------------------------------------------
// 16. ShotStore
// ---------------------------------------------------------------------

pub struct DocumentBackedShotStore {
    handle: Arc<dyn DocumentCollectionHandle>,
}

impl DocumentBackedShotStore {
    pub fn new(handle: Arc<dyn DocumentCollectionHandle>) -> Self {
        Self { handle }
    }
}

#[async_trait]
impl ShotStore for DocumentBackedShotStore {
    async fn create(&self, s: Shot) -> CoreResult<Shot> {
        self.handle
            .insert_one(to_value(&s)?)
            .await
            .map_err(persistence_to_core)?;
        Ok(s)
    }
    async fn read(&self, id: &ShotId) -> CoreResult<Option<Shot>> {
        let f = id_filter("id", id.as_str());
        match self.handle.find_one(&f).await.map_err(persistence_to_core)? {
            Some(v) => Ok(Some(from_value(v)?)),
            None => Ok(None),
        }
    }
    async fn delete(&self, id: &ShotId) -> CoreResult<()> {
        let f = id_filter("id", id.as_str());
        self.handle.delete_one(&f).await.map_err(persistence_to_core)?;
        Ok(())
    }
    async fn list(&self, agent_id: &AgentId) -> CoreResult<Vec<Shot>> {
        let docs = self
            .handle
            .find(&id_filter("agent_id", agent_id.as_str()))
            .await
            .map_err(persistence_to_core)?;
        let mut out = Vec::with_capacity(docs.len());
        for v in docs {
            out.push(from_value(v)?);
        }
        Ok(out)
    }
}

// =====================================================================
// Tests — one round-trip per store, using JsonFileDocumentDatabase.
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Agent, CannedResponse, Capability, ContextVariable, ContextVariableUpdateParams,
        Customer, GuidelineContent, JourneyId, JourneyNodeId, Observation, Relationship,
        RelationshipEntity, RelationshipEntityKind, RelationshipKind, Retriever, Session, Shot,
        Tag, Tool, ToolKind, ToolUpdateParams,
    };
    use loon_persistence::backends::json_file::JsonFileDocumentDatabase;
    use std::time::Duration;
    use tempfile::tempdir;

    async fn db_handle() -> (tempfile::TempDir, Arc<dyn DocumentDatabaseHandle>) {
        let dir = tempdir().unwrap();
        let db: Arc<dyn DocumentDatabaseHandle> = Arc::new(
            JsonFileDocumentDatabase::new(dir.path(), Duration::from_millis(50)).unwrap(),
        );
        (dir, db)
    }

    #[tokio::test]
    async fn agent_store_round_trip() {
        let (_dir, db) = db_handle().await;
        let s = DocumentBackedAgentStore::new(db.collection("agents").await.unwrap());
        let a = Agent::new("test", "x");
        let id = a.id.clone();
        s.create(a).await.unwrap();
        let loaded = s.read(&id).await.unwrap().unwrap();
        assert_eq!(loaded.name, "test");

        let updated = s
            .update(
                &id,
                AgentUpdateParams {
                    name: Some("renamed".into()),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(updated.name, "renamed");

        s.delete(&id).await.unwrap();
        assert!(s.read(&id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn session_store_round_trip() {
        let (_dir, db) = db_handle().await;
        let s = DocumentBackedSessionStore::new(
            db.collection("sessions").await.unwrap(),
            db.clone(),
        );
        let agent_id = AgentId::new();
        let sess = Session::new(&agent_id);
        let sid = sess.id.clone();
        s.create(sess).await.unwrap();
        let loaded = s.read(&sid).await.unwrap().unwrap();
        assert_eq!(loaded.agent_id, agent_id);

        let ev = Event {
            id: EventId::new(),
            source: crate::EventSource::Customer,
            kind: crate::EventKind::Message,
            trace_id: "t".into(),
            data: serde_json::json!({"message": "hi"}),
            metadata: None,
            creation_utc: chrono::Utc::now(),
        };
        s.create_event(sid.clone(), ev.clone()).await.unwrap();
        let events = s.read_events(&sid).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id, ev.id);
    }

    #[tokio::test]
    async fn guideline_store_round_trip() {
        let (_dir, db) = db_handle().await;
        let s = DocumentBackedGuidelineStore::new(db.collection("guidelines").await.unwrap());
        let agent_id = AgentId::new();
        let g = Guideline::new(
            GuidelineContent {
                condition: "c".into(),
                action: "a".into(),
                description: None,
            },
            &agent_id,
            true,
            0,
        );
        let id = g.id.clone();
        s.create(g).await.unwrap();
        assert_eq!(s.list(&agent_id, &[]).await.unwrap().len(), 1);

        let updated = s
            .update(
                &id,
                GuidelineUpdateParams {
                    condition: Some("c2".into()),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(updated.content.condition, "c2");
        s.delete(&id).await.unwrap();
        assert!(s.list(&agent_id, &[]).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn journey_store_round_trip() {
        let (_dir, db) = db_handle().await;
        let s = DocumentBackedJourneyStore::new(db.collection("journeys").await.unwrap());
        let agent_id = AgentId::new();
        let j = Journey {
            id: JourneyId::new(),
            agent_id: agent_id.clone(),
            title: "j".into(),
            description: "d".into(),
            root_id: JourneyNodeId::new(),
            tags: vec![],
            creation_utc: chrono::Utc::now(),
        };
        let id = j.id.clone();
        s.create(j).await.unwrap();
        let loaded = s.read(&id).await.unwrap().unwrap();
        assert_eq!(loaded.title, "j");
        assert_eq!(s.list(&agent_id).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn tool_store_round_trip() {
        let (_dir, db) = db_handle().await;
        let s = DocumentBackedToolStore::new(db.collection("tools").await.unwrap());
        let t = Tool {
            id: ToolId::new(),
            name: "t".into(),
            description: "d".into(),
            parameters_schema: serde_json::Value::Null,
            kind: ToolKind::Local,
            creation_utc: chrono::Utc::now(),
        };
        let id = t.id.clone();
        s.create(t).await.unwrap();
        let loaded = s.read(&id).await.unwrap().unwrap();
        assert_eq!(loaded.name, "t");
        let updated = s
            .update(
                &id,
                ToolUpdateParams {
                    name: Some("renamed".into()),
                    description: None,
                    parameters_schema: None,
                },
            )
            .await
            .unwrap();
        assert_eq!(updated.name, "renamed");
    }

    #[tokio::test]
    async fn evaluation_store_round_trip() {
        let (_dir, db) = db_handle().await;
        let s = DocumentBackedEvaluationStore::new(db.collection("evaluations").await.unwrap());
        let agent_id = AgentId::new();
        let o = Observation::new("c", vec![], &agent_id);
        let id = o.id.clone();
        s.create(o).await.unwrap();
        assert_eq!(s.list(&agent_id).await.unwrap().len(), 1);
        assert!(s.read(&id).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn customer_store_round_trip() {
        let (_dir, db) = db_handle().await;
        let s = DocumentBackedCustomerStore::new(db.collection("customers").await.unwrap());
        let c = Customer::new("alice");
        let id = c.id.clone();
        s.create(c).await.unwrap();
        let loaded = s.read(&id).await.unwrap().unwrap();
        assert_eq!(loaded.name, "alice");
        assert_eq!(s.list(&[]).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn glossary_store_round_trip() {
        let (_dir, db) = db_handle().await;
        let s = DocumentBackedGlossaryStore::new(db.collection("glossary").await.unwrap());
        let t = Term::new("foo", "bar");
        let id = t.id.clone();
        s.create_term(t).await.unwrap();
        let loaded = s.read_term(&id).await.unwrap().unwrap();
        assert_eq!(loaded.name, "foo");
        s.delete_term(&id).await.unwrap();
        assert!(s.read_term(&id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn context_variable_store_round_trip() {
        let (_dir, db) = db_handle().await;
        let s = DocumentBackedContextVariableStore::new(
            db.collection("context_variables").await.unwrap(),
            db.collection("context_variable_values").await.unwrap(),
        );
        let agent_id = AgentId::new();
        let v = ContextVariable {
            id: ContextVariableId::new(),
            agent_id: agent_id.clone(),
            key: "balance".into(),
            freshness_rules: vec![],
            tags: vec![],
            creation_utc: chrono::Utc::now(),
        };
        let id = v.id.clone();
        s.create(v).await.unwrap();
        let loaded = s.read(&id).await.unwrap().unwrap();
        assert_eq!(loaded.key, "balance");

        let payload = serde_json::json!({"amount": 42});
        let value = s
            .upsert_value(&id, "user_a", payload.clone())
            .await
            .unwrap();
        assert_eq!(value.data, payload);
        let read_back = s.read_value(&id, "user_a").await.unwrap().unwrap();
        assert_eq!(read_back.data, payload);

        // update with new payload via descriptor's update path
        let new_payload = serde_json::json!({"amount": 100});
        s.update(
            &id,
            ContextVariableUpdateParams {
                key: None,
                data: Some(new_payload.clone()),
            },
        )
        .await
        .unwrap();
        let read_after = s.read_value(&id, "balance").await.unwrap().unwrap();
        assert_eq!(read_after.data, new_payload);
    }

    #[tokio::test]
    async fn canned_response_store_round_trip() {
        let (_dir, db) = db_handle().await;
        let s = DocumentBackedCannedResponseStore::new(
            db.collection("canned_responses").await.unwrap(),
        );
        let agent_id = AgentId::new();
        let c = CannedResponse::new(&agent_id, "hi");
        let id = c.id.clone();
        s.create(c).await.unwrap();
        let loaded = s.read(&id).await.unwrap().unwrap();
        assert_eq!(loaded.value, "hi");
        assert_eq!(s.list(&agent_id).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn capability_store_round_trip() {
        let (_dir, db) = db_handle().await;
        let s = DocumentBackedCapabilityStore::new(db.collection("capabilities").await.unwrap());
        let agent_id = AgentId::new();
        let c = Capability::new(&agent_id, "n", "d");
        let id = c.id.clone();
        s.create(c).await.unwrap();
        let loaded = s.read(&id).await.unwrap().unwrap();
        assert_eq!(loaded.name, "n");
        assert_eq!(s.list(&agent_id).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn retriever_store_round_trip() {
        let (_dir, db) = db_handle().await;
        let s = DocumentBackedRetrieverStore::new(db.collection("retrievers").await.unwrap());
        let agent_id = AgentId::new();
        let r = Retriever::new(&agent_id, "r");
        let id = r.id.clone();
        s.create(r).await.unwrap();
        let loaded = s.read(&id).await.unwrap().unwrap();
        assert_eq!(loaded.name, "r");
        s.delete(&id).await.unwrap();
        assert!(s.read(&id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn tag_store_round_trip() {
        let (_dir, db) = db_handle().await;
        let s = DocumentBackedTagStore::new(db.collection("tags").await.unwrap());
        let t = Tag::new("vip");
        let id = t.id.clone();
        s.create(t).await.unwrap();
        let loaded = s.read(&id).await.unwrap().unwrap();
        assert_eq!(loaded.name, "vip");
        let updated = s
            .update(&id, TagUpdateParams { name: Some("ultra-vip".into()) })
            .await
            .unwrap();
        assert_eq!(updated.name, "ultra-vip");
        assert_eq!(s.list().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn relationship_store_round_trip() {
        let (_dir, db) = db_handle().await;
        let s = DocumentBackedRelationshipStore::new(db.collection("relationships").await.unwrap());
        let a = RelationshipEntity {
            kind: RelationshipEntityKind::Guideline,
            id: "a".into(),
        };
        let b = RelationshipEntity {
            kind: RelationshipEntityKind::Guideline,
            id: "b".into(),
        };
        let r = Relationship::new(a.clone(), b.clone(), RelationshipKind::Excludes);
        let id = r.id.clone();
        s.create(r).await.unwrap();
        let loaded = s.read(&id).await.unwrap().unwrap();
        assert_eq!(loaded.kind, RelationshipKind::Excludes);
        assert_eq!(s.list_for(&a).await.unwrap().len(), 1);
        assert_eq!(s.list_for(&b).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn guideline_tool_association_round_trip() {
        let (_dir, db) = db_handle().await;
        let s = DocumentBackedGuidelineToolAssociationStore::new(
            db.collection("guideline_tool_associations").await.unwrap(),
        );
        let gid = GuidelineId::new();
        let tid = ToolId::new();
        let a = GuidelineToolAssociation::new(&gid, &tid);
        let id = a.id.clone();
        s.create(a).await.unwrap();
        assert!(s.read(&id).await.unwrap().is_some());
        assert_eq!(s.list_for_tool(&tid).await.unwrap().len(), 1);
        assert_eq!(s.list_for_guideline(&gid).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn shot_store_round_trip() {
        let (_dir, db) = db_handle().await;
        let s = DocumentBackedShotStore::new(db.collection("shots").await.unwrap());
        let agent_id = AgentId::new();
        let sh = Shot::new(&agent_id, "c", "a", "in", "out");
        let id = sh.id.clone();
        s.create(sh).await.unwrap();
        let loaded = s.read(&id).await.unwrap().unwrap();
        assert_eq!(loaded.example_input, "in");
        assert_eq!(s.list(&agent_id).await.unwrap().len(), 1);
    }

    /// Data inserted via a `DocumentBackedAgentStore` persists when a
    /// fresh handle is opened on the same on-disk directory. This is
    /// the smoke test for "real persistence".
    #[tokio::test]
    async fn agents_persist_across_handle_reopen() {
        let dir = tempdir().unwrap();
        let path = dir.path().to_path_buf();
        {
            let db: Arc<dyn DocumentDatabaseHandle> = Arc::new(
                JsonFileDocumentDatabase::new(&path, Duration::from_millis(50)).unwrap(),
            );
            let store = DocumentBackedAgentStore::new(db.collection("agents").await.unwrap());
            let a = Agent::new("persist-me", "x");
            store.create(a).await.unwrap();
        }
        // Re-open and verify the agent is still there.
        let db: Arc<dyn DocumentDatabaseHandle> =
            Arc::new(JsonFileDocumentDatabase::new(&path, Duration::from_millis(50)).unwrap());
        let store = DocumentBackedAgentStore::new(db.collection("agents").await.unwrap());
        let all = store.list(&[]).await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].name, "persist-me");
    }
}

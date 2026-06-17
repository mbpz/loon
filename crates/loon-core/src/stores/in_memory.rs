//! In-memory implementations of all 16 Store traits.
//!
//! Backed by `parking_lot::Mutex<HashMap<Id, Entity>>`. Used as the
//! default backing for `EntityQueries` when no persistent store is
//! provided. Tests and quick-start examples can rely on these
//! without any external DB.

use std::collections::HashMap;

use async_trait::async_trait;
use parking_lot::Mutex;

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
    GuidelineUpdateParams, JsonValue, Journey, JourneyId, JourneyUpdateParams, Observation,
    Relationship, RelationshipEntity, RelationshipId, Retriever, RetrieverId, Session, SessionId,
    SessionUpdateParams, Shot, ShotId, Tag, TagId, Term, Tool, ToolId, UniqueId,
};

// ---------------------------------------------------------------------
// 1. AgentStore
// ---------------------------------------------------------------------

pub struct InMemoryAgentStore {
    data: Mutex<HashMap<AgentId, Agent>>,
}

impl InMemoryAgentStore {
    pub fn new() -> Self {
        Self {
            data: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryAgentStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AgentStore for InMemoryAgentStore {
    async fn create(&self, a: Agent) -> CoreResult<Agent> {
        self.data.lock().insert(a.id.clone(), a.clone());
        Ok(a)
    }
    async fn read(&self, id: &AgentId) -> CoreResult<Option<Agent>> {
        Ok(self.data.lock().get(id).cloned())
    }
    async fn update(&self, id: &AgentId, p: AgentUpdateParams) -> CoreResult<Agent> {
        let mut d = self.data.lock();
        let a = d
            .get_mut(id)
            .ok_or_else(|| CoreError::NotFound(UniqueId(id.0.clone())))?;
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
        Ok(a.clone())
    }
    async fn delete(&self, id: &AgentId) -> CoreResult<()> {
        self.data.lock().remove(id);
        Ok(())
    }
    async fn list(&self, _tags: &[TagId]) -> CoreResult<Vec<Agent>> {
        Ok(self.data.lock().values().cloned().collect())
    }
}

// ---------------------------------------------------------------------
// 2. SessionStore (with Event sub-resource)
// ---------------------------------------------------------------------

pub struct InMemorySessionStore {
    sessions: Mutex<HashMap<SessionId, Session>>,
    events: Mutex<HashMap<SessionId, Vec<Event>>>,
}

impl InMemorySessionStore {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            events: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for InMemorySessionStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SessionStore for InMemorySessionStore {
    async fn create(&self, s: Session) -> CoreResult<Session> {
        self.sessions.lock().insert(s.id.clone(), s.clone());
        Ok(s)
    }
    async fn read(&self, id: &SessionId) -> CoreResult<Option<Session>> {
        Ok(self.sessions.lock().get(id).cloned())
    }
    async fn update(&self, id: &SessionId, p: SessionUpdateParams) -> CoreResult<Session> {
        let mut d = self.sessions.lock();
        let s = d
            .get_mut(id)
            .ok_or_else(|| CoreError::NotFound(UniqueId(id.0.clone())))?;
        if let Some(t) = p.title {
            s.title = Some(t);
        }
        if let Some(m) = p.mode {
            s.mode = m;
        }
        if let Some(l) = p.labels {
            s.labels = l;
        }
        Ok(s.clone())
    }
    async fn delete(&self, id: &SessionId) -> CoreResult<()> {
        self.sessions.lock().remove(id);
        self.events.lock().remove(id);
        Ok(())
    }
    async fn list(
        &self,
        agent_id: Option<&AgentId>,
        customer_id: Option<&CustomerId>,
    ) -> CoreResult<Vec<Session>> {
        Ok(self
            .sessions
            .lock()
            .values()
            .filter(|s| agent_id.map(|a| &s.agent_id == a).unwrap_or(true))
            .filter(|s| {
                customer_id
                    .map(|c| s.customer_id.as_ref() == Some(c))
                    .unwrap_or(true)
            })
            .cloned()
            .collect())
    }
    async fn create_event(&self, sid: SessionId, e: Event) -> CoreResult<Event> {
        self.events.lock().entry(sid).or_default().push(e.clone());
        Ok(e)
    }
    async fn update_event(
        &self,
        sid: &SessionId,
        eid: &EventId,
        p: EventUpdateParams,
    ) -> CoreResult<Event> {
        let mut d = self.events.lock();
        let evs = d
            .get_mut(sid)
            .ok_or_else(|| CoreError::NotFound(UniqueId(sid.0.clone())))?;
        let e = evs
            .iter_mut()
            .find(|e| &e.id == eid)
            .ok_or_else(|| CoreError::NotFound(UniqueId(eid.0.clone())))?;
        if let Some(data) = p.data {
            e.data = data;
        }
        if let Some(m) = p.metadata {
            e.metadata = Some(m);
        }
        Ok(e.clone())
    }
    async fn read_events(&self, sid: &SessionId) -> CoreResult<Vec<Event>> {
        Ok(self.events.lock().get(sid).cloned().unwrap_or_default())
    }
    async fn find_events(&self, sid: &SessionId) -> CoreResult<Vec<Event>> {
        self.read_events(sid).await
    }
}

// ---------------------------------------------------------------------
// 3. GuidelineStore
// ---------------------------------------------------------------------

pub struct InMemoryGuidelineStore {
    data: Mutex<HashMap<GuidelineId, Guideline>>,
}

impl InMemoryGuidelineStore {
    pub fn new() -> Self {
        Self {
            data: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryGuidelineStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl GuidelineStore for InMemoryGuidelineStore {
    async fn create(&self, g: Guideline) -> CoreResult<Guideline> {
        self.data.lock().insert(g.id.clone(), g.clone());
        Ok(g)
    }
    async fn read(&self, id: &GuidelineId) -> CoreResult<Option<Guideline>> {
        Ok(self.data.lock().get(id).cloned())
    }
    async fn update(&self, id: &GuidelineId, p: GuidelineUpdateParams) -> CoreResult<Guideline> {
        let mut d = self.data.lock();
        let g = d
            .get_mut(id)
            .ok_or_else(|| CoreError::NotFound(UniqueId(id.0.clone())))?;
        if let Some(c) = p.condition {
            g.content.condition = c;
        }
        if let Some(a) = p.action {
            g.content.action = a;
        }
        if let Some(e) = p.enabled {
            g.enabled = e;
        }
        Ok(g.clone())
    }
    async fn delete(&self, id: &GuidelineId) -> CoreResult<()> {
        self.data.lock().remove(id);
        Ok(())
    }
    async fn list(&self, agent_id: &AgentId, _tags: &[TagId]) -> CoreResult<Vec<Guideline>> {
        Ok(self
            .data
            .lock()
            .values()
            .filter(|g| &g.agent_id == agent_id)
            .cloned()
            .collect())
    }
}

// ---------------------------------------------------------------------
// 4. JourneyStore
// ---------------------------------------------------------------------

pub struct InMemoryJourneyStore {
    data: Mutex<HashMap<JourneyId, Journey>>,
}

impl InMemoryJourneyStore {
    pub fn new() -> Self {
        Self {
            data: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryJourneyStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl JourneyStore for InMemoryJourneyStore {
    async fn create(&self, j: Journey) -> CoreResult<Journey> {
        self.data.lock().insert(j.id.clone(), j.clone());
        Ok(j)
    }
    async fn read(&self, id: &JourneyId) -> CoreResult<Option<Journey>> {
        Ok(self.data.lock().get(id).cloned())
    }
    async fn update(&self, id: &JourneyId, p: JourneyUpdateParams) -> CoreResult<Journey> {
        let mut d = self.data.lock();
        let j = d
            .get_mut(id)
            .ok_or_else(|| CoreError::NotFound(UniqueId(id.0.clone())))?;
        if let Some(t) = p.title {
            j.title = t;
        }
        if let Some(desc) = p.description {
            j.description = desc;
        }
        Ok(j.clone())
    }
    async fn delete(&self, id: &JourneyId) -> CoreResult<()> {
        self.data.lock().remove(id);
        Ok(())
    }
    async fn list(&self, agent_id: &AgentId) -> CoreResult<Vec<Journey>> {
        Ok(self
            .data
            .lock()
            .values()
            .filter(|j| &j.agent_id == agent_id)
            .cloned()
            .collect())
    }
}

// ---------------------------------------------------------------------
// 5. ToolStore
// ---------------------------------------------------------------------

pub struct InMemoryToolStore {
    data: Mutex<HashMap<ToolId, Tool>>,
    by_agent: Mutex<HashMap<AgentId, Vec<ToolId>>>,
}

impl InMemoryToolStore {
    pub fn new() -> Self {
        Self {
            data: Mutex::new(HashMap::new()),
            by_agent: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryToolStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ToolStore for InMemoryToolStore {
    async fn create(&self, t: Tool) -> CoreResult<Tool> {
        self.data.lock().insert(t.id.clone(), t.clone());
        Ok(t)
    }
    async fn read(&self, id: &ToolId) -> CoreResult<Option<Tool>> {
        Ok(self.data.lock().get(id).cloned())
    }
    async fn delete(&self, id: &ToolId) -> CoreResult<()> {
        self.data.lock().remove(id);
        let mut by_agent = self.by_agent.lock();
        for ids in by_agent.values_mut() {
            ids.retain(|x| x != id);
        }
        Ok(())
    }
    async fn list(&self, agent_id: &AgentId) -> CoreResult<Vec<Tool>> {
        let by_agent = self.by_agent.lock();
        let data = self.data.lock();
        Ok(by_agent
            .get(agent_id)
            .map(|ids| ids.iter().filter_map(|i| data.get(i).cloned()).collect())
            .unwrap_or_default())
    }
}

// ---------------------------------------------------------------------
// 6. EvaluationStore (Observation entity)
// ---------------------------------------------------------------------

pub struct InMemoryEvaluationStore {
    data: Mutex<HashMap<EvaluationId, Observation>>,
}

impl InMemoryEvaluationStore {
    pub fn new() -> Self {
        Self {
            data: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryEvaluationStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EvaluationStore for InMemoryEvaluationStore {
    async fn create(&self, e: Observation) -> CoreResult<Observation> {
        self.data.lock().insert(e.id.clone(), e.clone());
        Ok(e)
    }
    async fn read(&self, id: &EvaluationId) -> CoreResult<Option<Observation>> {
        Ok(self.data.lock().get(id).cloned())
    }
    async fn delete(&self, id: &EvaluationId) -> CoreResult<()> {
        self.data.lock().remove(id);
        Ok(())
    }
    async fn list(&self, agent_id: &AgentId) -> CoreResult<Vec<Observation>> {
        Ok(self
            .data
            .lock()
            .values()
            .filter(|o| &o.agent_id == agent_id)
            .cloned()
            .collect())
    }
}

// ---------------------------------------------------------------------
// 7. CustomerStore
// ---------------------------------------------------------------------

pub struct InMemoryCustomerStore {
    data: Mutex<HashMap<CustomerId, Customer>>,
}

impl InMemoryCustomerStore {
    pub fn new() -> Self {
        Self {
            data: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryCustomerStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CustomerStore for InMemoryCustomerStore {
    async fn create(&self, c: Customer) -> CoreResult<Customer> {
        self.data.lock().insert(c.id.clone(), c.clone());
        Ok(c)
    }
    async fn read(&self, id: &CustomerId) -> CoreResult<Option<Customer>> {
        Ok(self.data.lock().get(id).cloned())
    }
    async fn update(&self, id: &CustomerId, p: CustomerUpdateParams) -> CoreResult<Customer> {
        let mut d = self.data.lock();
        let c = d
            .get_mut(id)
            .ok_or_else(|| CoreError::NotFound(UniqueId(id.0.clone())))?;
        if let Some(n) = p.name {
            c.name = n;
        }
        if let Some(m) = p.metadata {
            c.metadata = m;
        }
        Ok(c.clone())
    }
    async fn delete(&self, id: &CustomerId) -> CoreResult<()> {
        self.data.lock().remove(id);
        Ok(())
    }
    async fn list(&self, _tags: &[TagId]) -> CoreResult<Vec<Customer>> {
        Ok(self.data.lock().values().cloned().collect())
    }
}

// ---------------------------------------------------------------------
// 8. GlossaryStore (Term entity)
// ---------------------------------------------------------------------

pub struct InMemoryGlossaryStore {
    data: Mutex<HashMap<GlossaryTermId, Term>>,
    by_agent: Mutex<HashMap<AgentId, Vec<GlossaryTermId>>>,
}

impl InMemoryGlossaryStore {
    pub fn new() -> Self {
        Self {
            data: Mutex::new(HashMap::new()),
            by_agent: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryGlossaryStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl GlossaryStore for InMemoryGlossaryStore {
    async fn create_term(&self, t: Term) -> CoreResult<Term> {
        self.data.lock().insert(t.id.clone(), t.clone());
        Ok(t)
    }
    async fn read_term(&self, id: &GlossaryTermId) -> CoreResult<Option<Term>> {
        Ok(self.data.lock().get(id).cloned())
    }
    async fn update_term(&self, id: &GlossaryTermId, t: Term) -> CoreResult<Term> {
        let mut d = self.data.lock();
        if !d.contains_key(id) {
            return Err(CoreError::NotFound(UniqueId(id.0.clone())));
        }
        d.insert(id.clone(), t.clone());
        Ok(t)
    }
    async fn delete_term(&self, id: &GlossaryTermId) -> CoreResult<()> {
        self.data.lock().remove(id);
        let mut by_agent = self.by_agent.lock();
        for ids in by_agent.values_mut() {
            ids.retain(|x| x != id);
        }
        Ok(())
    }
    async fn list_terms(&self, agent_id: &AgentId) -> CoreResult<Vec<Term>> {
        let by_agent = self.by_agent.lock();
        let data = self.data.lock();
        Ok(by_agent
            .get(agent_id)
            .map(|ids| ids.iter().filter_map(|i| data.get(i).cloned()).collect())
            .unwrap_or_default())
    }
}

// ---------------------------------------------------------------------
// 9. ContextVariableStore (with upsert_value)
// ---------------------------------------------------------------------

pub struct InMemoryContextVariableStore {
    data: Mutex<HashMap<ContextVariableId, ContextVariable>>,
    values: Mutex<HashMap<(ContextVariableId, String), ContextVariableValue>>,
}

impl InMemoryContextVariableStore {
    pub fn new() -> Self {
        Self {
            data: Mutex::new(HashMap::new()),
            values: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryContextVariableStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ContextVariableStore for InMemoryContextVariableStore {
    async fn create(&self, v: ContextVariable) -> CoreResult<ContextVariable> {
        self.data.lock().insert(v.id.clone(), v.clone());
        Ok(v)
    }
    async fn read(&self, id: &ContextVariableId) -> CoreResult<Option<ContextVariable>> {
        Ok(self.data.lock().get(id).cloned())
    }
    async fn update(
        &self,
        id: &ContextVariableId,
        p: ContextVariableUpdateParams,
    ) -> CoreResult<ContextVariable> {
        let mut d = self.data.lock();
        let v = d
            .get_mut(id)
            .ok_or_else(|| CoreError::NotFound(UniqueId(id.0.clone())))?;
        if let Some(k) = p.key {
            v.key = k;
        }
        // `data` lives on `ContextVariableValue`, not the variable
        // descriptor itself — surface it via `upsert_value`.
        let _ = p.data;
        Ok(v.clone())
    }
    async fn delete(&self, id: &ContextVariableId) -> CoreResult<()> {
        self.data.lock().remove(id);
        self.values.lock().retain(|(vid, _), _| vid != id);
        Ok(())
    }
    async fn list(&self, agent_id: &AgentId) -> CoreResult<Vec<ContextVariable>> {
        Ok(self
            .data
            .lock()
            .values()
            .filter(|v| &v.agent_id == agent_id)
            .cloned()
            .collect())
    }
    async fn upsert_value(
        &self,
        var_id: &ContextVariableId,
        key: &str,
        data: JsonValue,
    ) -> CoreResult<ContextVariableValue> {
        let val = ContextVariableValue {
            key: key.to_string(),
            data,
            last_updated: chrono::Utc::now(),
        };
        self.values
            .lock()
            .insert((var_id.clone(), key.to_string()), val.clone());
        Ok(val)
    }
}

// ---------------------------------------------------------------------
// 10. CannedResponseStore
// ---------------------------------------------------------------------

pub struct InMemoryCannedResponseStore {
    data: Mutex<HashMap<CannedResponseId, CannedResponse>>,
}

impl InMemoryCannedResponseStore {
    pub fn new() -> Self {
        Self {
            data: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryCannedResponseStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CannedResponseStore for InMemoryCannedResponseStore {
    async fn create(&self, c: CannedResponse) -> CoreResult<CannedResponse> {
        self.data.lock().insert(c.id.clone(), c.clone());
        Ok(c)
    }
    async fn read(&self, id: &CannedResponseId) -> CoreResult<Option<CannedResponse>> {
        Ok(self.data.lock().get(id).cloned())
    }
    async fn update(
        &self,
        id: &CannedResponseId,
        p: CannedResponseUpdateParams,
    ) -> CoreResult<CannedResponse> {
        let mut d = self.data.lock();
        let c = d
            .get_mut(id)
            .ok_or_else(|| CoreError::NotFound(UniqueId(id.0.clone())))?;
        if let Some(v) = p.value {
            c.value = v;
        }
        if let Some(m) = p.matchers {
            c.matchers = m;
        }
        Ok(c.clone())
    }
    async fn delete(&self, id: &CannedResponseId) -> CoreResult<()> {
        self.data.lock().remove(id);
        Ok(())
    }
    async fn list(&self, agent_id: &AgentId) -> CoreResult<Vec<CannedResponse>> {
        Ok(self
            .data
            .lock()
            .values()
            .filter(|c| &c.agent_id == agent_id)
            .cloned()
            .collect())
    }
}

// ---------------------------------------------------------------------
// 11. CapabilityStore
// ---------------------------------------------------------------------

pub struct InMemoryCapabilityStore {
    data: Mutex<HashMap<CapabilityId, Capability>>,
}

impl InMemoryCapabilityStore {
    pub fn new() -> Self {
        Self {
            data: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryCapabilityStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CapabilityStore for InMemoryCapabilityStore {
    async fn create(&self, c: Capability) -> CoreResult<Capability> {
        self.data.lock().insert(c.id.clone(), c.clone());
        Ok(c)
    }
    async fn read(&self, id: &CapabilityId) -> CoreResult<Option<Capability>> {
        Ok(self.data.lock().get(id).cloned())
    }
    async fn update(&self, id: &CapabilityId, p: CapabilityUpdateParams) -> CoreResult<Capability> {
        let mut d = self.data.lock();
        let c = d
            .get_mut(id)
            .ok_or_else(|| CoreError::NotFound(UniqueId(id.0.clone())))?;
        if let Some(n) = p.name {
            c.name = n;
        }
        if let Some(desc) = p.description {
            c.description = desc;
        }
        Ok(c.clone())
    }
    async fn delete(&self, id: &CapabilityId) -> CoreResult<()> {
        self.data.lock().remove(id);
        Ok(())
    }
    async fn list(&self, agent_id: &AgentId) -> CoreResult<Vec<Capability>> {
        Ok(self
            .data
            .lock()
            .values()
            .filter(|c| &c.agent_id == agent_id)
            .cloned()
            .collect())
    }
}

// ---------------------------------------------------------------------
// 12. RetrieverStore
// ---------------------------------------------------------------------

pub struct InMemoryRetrieverStore {
    data: Mutex<HashMap<RetrieverId, Retriever>>,
}

impl InMemoryRetrieverStore {
    pub fn new() -> Self {
        Self {
            data: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryRetrieverStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RetrieverStore for InMemoryRetrieverStore {
    async fn create(&self, r: Retriever) -> CoreResult<Retriever> {
        self.data.lock().insert(r.id.clone(), r.clone());
        Ok(r)
    }
    async fn read(&self, id: &RetrieverId) -> CoreResult<Option<Retriever>> {
        Ok(self.data.lock().get(id).cloned())
    }
    async fn delete(&self, id: &RetrieverId) -> CoreResult<()> {
        self.data.lock().remove(id);
        Ok(())
    }
    async fn list(&self, agent_id: &AgentId) -> CoreResult<Vec<Retriever>> {
        Ok(self
            .data
            .lock()
            .values()
            .filter(|r| &r.agent_id == agent_id)
            .cloned()
            .collect())
    }
}

// ---------------------------------------------------------------------
// 13. TagStore
// ---------------------------------------------------------------------

pub struct InMemoryTagStore {
    data: Mutex<HashMap<TagId, Tag>>,
}

impl InMemoryTagStore {
    pub fn new() -> Self {
        Self {
            data: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryTagStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TagStore for InMemoryTagStore {
    async fn create(&self, t: Tag) -> CoreResult<Tag> {
        self.data.lock().insert(t.id.clone(), t.clone());
        Ok(t)
    }
    async fn read(&self, id: &TagId) -> CoreResult<Option<Tag>> {
        Ok(self.data.lock().get(id).cloned())
    }
    async fn list(&self) -> CoreResult<Vec<Tag>> {
        Ok(self.data.lock().values().cloned().collect())
    }
    async fn delete(&self, id: &TagId) -> CoreResult<()> {
        self.data.lock().remove(id);
        Ok(())
    }
}

// ---------------------------------------------------------------------
// 14. RelationshipStore (with list_for(entity))
// ---------------------------------------------------------------------

pub struct InMemoryRelationshipStore {
    data: Mutex<HashMap<RelationshipId, Relationship>>,
}

impl InMemoryRelationshipStore {
    pub fn new() -> Self {
        Self {
            data: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryRelationshipStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RelationshipStore for InMemoryRelationshipStore {
    async fn create(&self, r: Relationship) -> CoreResult<Relationship> {
        self.data.lock().insert(r.id.clone(), r.clone());
        Ok(r)
    }
    async fn read(&self, id: &RelationshipId) -> CoreResult<Option<Relationship>> {
        Ok(self.data.lock().get(id).cloned())
    }
    async fn delete(&self, id: &RelationshipId) -> CoreResult<()> {
        self.data.lock().remove(id);
        Ok(())
    }
    async fn list_for(&self, entity: &RelationshipEntity) -> CoreResult<Vec<Relationship>> {
        Ok(self
            .data
            .lock()
            .values()
            .filter(|r| &r.source == entity || &r.target == entity)
            .cloned()
            .collect())
    }
}

// ---------------------------------------------------------------------
// 15. GuidelineToolAssociationStore
// ---------------------------------------------------------------------

pub struct InMemoryGuidelineToolAssociationStore {
    data: Mutex<HashMap<GuidelineToolAssociationId, GuidelineToolAssociation>>,
}

impl InMemoryGuidelineToolAssociationStore {
    pub fn new() -> Self {
        Self {
            data: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryGuidelineToolAssociationStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl GuidelineToolAssociationStore for InMemoryGuidelineToolAssociationStore {
    async fn create(&self, a: GuidelineToolAssociation) -> CoreResult<GuidelineToolAssociation> {
        self.data.lock().insert(a.id.clone(), a.clone());
        Ok(a)
    }
    async fn read(
        &self,
        id: &GuidelineToolAssociationId,
    ) -> CoreResult<Option<GuidelineToolAssociation>> {
        Ok(self.data.lock().get(id).cloned())
    }
    async fn delete(&self, id: &GuidelineToolAssociationId) -> CoreResult<()> {
        self.data.lock().remove(id);
        Ok(())
    }
    async fn list_for_tool(&self, tool_id: &ToolId) -> CoreResult<Vec<GuidelineToolAssociation>> {
        Ok(self
            .data
            .lock()
            .values()
            .filter(|a| &a.tool_id == tool_id)
            .cloned()
            .collect())
    }
    async fn list_for_guideline(
        &self,
        guideline_id: &GuidelineId,
    ) -> CoreResult<Vec<GuidelineToolAssociation>> {
        Ok(self
            .data
            .lock()
            .values()
            .filter(|a| &a.guideline_id == guideline_id)
            .cloned()
            .collect())
    }
}

// ---------------------------------------------------------------------
// 16. ShotStore
// ---------------------------------------------------------------------

pub struct InMemoryShotStore {
    data: Mutex<HashMap<ShotId, Shot>>,
}

impl InMemoryShotStore {
    pub fn new() -> Self {
        Self {
            data: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryShotStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ShotStore for InMemoryShotStore {
    async fn create(&self, s: Shot) -> CoreResult<Shot> {
        self.data.lock().insert(s.id.clone(), s.clone());
        Ok(s)
    }
    async fn read(&self, id: &ShotId) -> CoreResult<Option<Shot>> {
        Ok(self.data.lock().get(id).cloned())
    }
    async fn delete(&self, id: &ShotId) -> CoreResult<()> {
        self.data.lock().remove(id);
        Ok(())
    }
    async fn list(&self, agent_id: &AgentId) -> CoreResult<Vec<Shot>> {
        Ok(self
            .data
            .lock()
            .values()
            .filter(|s| &s.agent_id == agent_id)
            .cloned()
            .collect())
    }
}

// =====================================================================
// Tests — one round-trip per store
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Criticality, GuidelineContent, JourneyNode, JourneyNodeId, RelationshipEntityKind,
        RelationshipKind, ToolKind,
    };

    #[tokio::test]
    async fn agent_store_round_trip() {
        let s = InMemoryAgentStore::new();
        let a = Agent::new("test", "x");
        let id = a.id.clone();
        s.create(a).await.unwrap();
        let loaded = s.read(&id).await.unwrap().unwrap();
        assert_eq!(loaded.name, "test");
    }

    #[tokio::test]
    async fn session_store_round_trip() {
        let s = InMemorySessionStore::new();
        let agent_id = AgentId::new();
        let sess = Session::new(&agent_id);
        let id = sess.id.clone();
        s.create(sess).await.unwrap();
        let loaded = s.read(&id).await.unwrap().unwrap();
        assert_eq!(loaded.agent_id, agent_id);
    }

    #[tokio::test]
    async fn session_store_event_round_trip() {
        let s = InMemorySessionStore::new();
        let agent_id = AgentId::new();
        let sess = Session::new(&agent_id);
        let sid = sess.id.clone();
        s.create(sess).await.unwrap();
        let event = Event {
            id: EventId::new(),
            source: crate::EventSource::Customer,
            kind: crate::EventKind::Message,
            trace_id: "t".into(),
            data: serde_json::json!({"message": "hi"}),
            metadata: None,
            creation_utc: chrono::Utc::now(),
        };
        s.create_event(sid.clone(), event).await.unwrap();
        let events = s.read_events(&sid).await.unwrap();
        assert_eq!(events.len(), 1);
    }

    #[tokio::test]
    async fn guideline_store_round_trip() {
        let s = InMemoryGuidelineStore::new();
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
        let loaded = s.read(&id).await.unwrap().unwrap();
        assert_eq!(loaded.content.condition, "c");
        let list = s.list(&agent_id, &[]).await.unwrap();
        assert_eq!(list.len(), 1);
    }

    #[tokio::test]
    async fn journey_store_round_trip() {
        let s = InMemoryJourneyStore::new();
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
        // exercise unrelated import
        let _ = JourneyNode::initial();
    }

    #[tokio::test]
    async fn tool_store_round_trip() {
        let s = InMemoryToolStore::new();
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
    }

    #[tokio::test]
    async fn evaluation_store_round_trip() {
        let s = InMemoryEvaluationStore::new();
        let agent_id = AgentId::new();
        let o = Observation::new("c", vec![], &agent_id);
        let id = o.id.clone();
        s.create(o).await.unwrap();
        let loaded = s.read(&id).await.unwrap().unwrap();
        assert_eq!(loaded.condition, "c");
    }

    #[tokio::test]
    async fn customer_store_round_trip() {
        let s = InMemoryCustomerStore::new();
        let c = Customer::new("alice");
        let id = c.id.clone();
        s.create(c).await.unwrap();
        let loaded = s.read(&id).await.unwrap().unwrap();
        assert_eq!(loaded.name, "alice");
    }

    #[tokio::test]
    async fn glossary_store_round_trip() {
        let s = InMemoryGlossaryStore::new();
        let t = Term::new("foo", "bar");
        let id = t.id.clone();
        s.create_term(t).await.unwrap();
        let loaded = s.read_term(&id).await.unwrap().unwrap();
        assert_eq!(loaded.name, "foo");
    }

    #[tokio::test]
    async fn context_variable_store_round_trip() {
        let s = InMemoryContextVariableStore::new();
        let agent_id = AgentId::new();
        let v = ContextVariable {
            id: ContextVariableId::new(),
            agent_id: agent_id.clone(),
            key: "k".into(),
            freshness_rules: vec![],
            tags: vec![],
            creation_utc: chrono::Utc::now(),
        };
        let id = v.id.clone();
        s.create(v).await.unwrap();
        let loaded = s.read(&id).await.unwrap().unwrap();
        assert_eq!(loaded.key, "k");
        let value = s
            .upsert_value(&id, "user_a", serde_json::json!(42))
            .await
            .unwrap();
        assert_eq!(value.data, serde_json::json!(42));
    }

    #[tokio::test]
    async fn canned_response_store_round_trip() {
        let s = InMemoryCannedResponseStore::new();
        let agent_id = AgentId::new();
        let c = CannedResponse::new(&agent_id, "hi");
        let id = c.id.clone();
        s.create(c).await.unwrap();
        let loaded = s.read(&id).await.unwrap().unwrap();
        assert_eq!(loaded.value, "hi");
    }

    #[tokio::test]
    async fn capability_store_round_trip() {
        let s = InMemoryCapabilityStore::new();
        let agent_id = AgentId::new();
        let c = Capability::new(&agent_id, "n", "d");
        let id = c.id.clone();
        s.create(c).await.unwrap();
        let loaded = s.read(&id).await.unwrap().unwrap();
        assert_eq!(loaded.name, "n");
    }

    #[tokio::test]
    async fn retriever_store_round_trip() {
        let s = InMemoryRetrieverStore::new();
        let agent_id = AgentId::new();
        let r = Retriever::new(&agent_id, "r");
        let id = r.id.clone();
        s.create(r).await.unwrap();
        let loaded = s.read(&id).await.unwrap().unwrap();
        assert_eq!(loaded.name, "r");
    }

    #[tokio::test]
    async fn tag_store_round_trip() {
        let s = InMemoryTagStore::new();
        let t = Tag::new("vip");
        let id = t.id.clone();
        s.create(t).await.unwrap();
        let loaded = s.read(&id).await.unwrap().unwrap();
        assert_eq!(loaded.name, "vip");
    }

    #[tokio::test]
    async fn relationship_store_round_trip() {
        let s = InMemoryRelationshipStore::new();
        let entity_a = RelationshipEntity {
            kind: RelationshipEntityKind::Guideline,
            id: "a".into(),
        };
        let entity_b = RelationshipEntity {
            kind: RelationshipEntityKind::Guideline,
            id: "b".into(),
        };
        let r = Relationship::new(entity_a.clone(), entity_b.clone(), RelationshipKind::Excludes);
        let id = r.id.clone();
        s.create(r).await.unwrap();
        let loaded = s.read(&id).await.unwrap().unwrap();
        assert_eq!(loaded.kind, RelationshipKind::Excludes);
        let listed = s.list_for(&entity_a).await.unwrap();
        assert_eq!(listed.len(), 1);
    }

    #[tokio::test]
    async fn guideline_tool_association_store_round_trip() {
        let s = InMemoryGuidelineToolAssociationStore::new();
        let gid = GuidelineId::new();
        let tid = ToolId::new();
        let a = GuidelineToolAssociation::new(&gid, &tid);
        let id = a.id.clone();
        s.create(a).await.unwrap();
        let loaded = s.read(&id).await.unwrap().unwrap();
        assert_eq!(loaded.guideline_id, gid);
        let by_tool = s.list_for_tool(&tid).await.unwrap();
        assert_eq!(by_tool.len(), 1);
        let by_g = s.list_for_guideline(&gid).await.unwrap();
        assert_eq!(by_g.len(), 1);
    }

    #[tokio::test]
    async fn shot_store_round_trip() {
        let s = InMemoryShotStore::new();
        let agent_id = AgentId::new();
        let sh = Shot::new(&agent_id, "c", "a", "in", "out");
        let id = sh.id.clone();
        s.create(sh).await.unwrap();
        let loaded = s.read(&id).await.unwrap().unwrap();
        assert_eq!(loaded.example_input, "in");
        // touch unused crate-internal reference
        let _ = Criticality::Low;
    }
}

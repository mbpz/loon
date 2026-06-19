//! In-memory implementations of all 16 Store traits.
//!
//! Backed by `parking_lot::Mutex<HashMap<Id, Entity>>`. Used as the
//! default backing for `EntityQueries` when no persistent store is
//! provided. Tests and quick-start examples can rely on these
//! without any external DB.

use std::collections::HashMap;

use async_trait::async_trait;
use parking_lot::Mutex;

/// Parlcant-style AND-semantics tag filter: empty `requested` is
/// "no filter"; non-empty `requested` matches when `owned` contains
/// every requested tag. Shared by every `InMemory*Store::list`
/// implementation that takes a `tags: &[TagId]` argument.
fn matches_all_tags(requested: &[TagId], owned: &[TagId]) -> bool {
    requested.is_empty() || requested.iter().all(|t| owned.contains(t))
}

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
    GuidelineUpdateParams, Journey, JourneyId, JourneyUpdateParams, JsonValue, Observation,
    ObservationUpdateParams, Relationship, RelationshipEntity, RelationshipId, Retriever, RetrieverId, Session, SessionId,
    SessionUpdateParams, Shot, ShotId, Tag, TagId, TagUpdateParams, Term, Tool, ToolId,
    ToolUpdateParams, UniqueId,
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
    async fn list(&self, tags: &[TagId]) -> CoreResult<Vec<Agent>> {
        let all = self.data.lock();
        Ok(all
            .values()
            .filter(|a| matches_all_tags(tags, &a.tags))
            .cloned()
            .collect())
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
    async fn list(&self, agent_id: &AgentId, tags: &[TagId]) -> CoreResult<Vec<Guideline>> {
        let all = self.data.lock();
        Ok(all
            .values()
            .filter(|g| &g.agent_id == agent_id)
            .filter(|g| matches_all_tags(tags, &g.tags))
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
    async fn update(&self, id: &ToolId, params: ToolUpdateParams) -> CoreResult<Tool> {
        let mut d = self.data.lock();
        let t = d
            .get_mut(id)
            .ok_or_else(|| CoreError::NotFound(UniqueId(id.0.clone())))?;
        if let Some(name) = params.name {
            t.name = name;
        }
        if let Some(description) = params.description {
            t.description = description;
        }
        if let Some(parameters_schema) = params.parameters_schema {
            t.parameters_schema = parameters_schema;
        }
        Ok(t.clone())
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
    async fn update(
        &self,
        id: &EvaluationId,
        params: ObservationUpdateParams,
    ) -> CoreResult<Observation> {
        let mut d = self.data.lock();
        let o = d
            .get_mut(id)
            .ok_or_else(|| CoreError::NotFound(UniqueId(id.0.clone())))?;
        if let Some(c) = params.condition {
            o.condition = c;
        }
        if let Some(t) = params.tools {
            o.tools = t;
        }
        if let Some(e) = params.enabled {
            o.enabled = e;
        }
        Ok(o.clone())
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
    async fn list(&self, tags: &[TagId]) -> CoreResult<Vec<Customer>> {
        let all = self.data.lock();
        Ok(all
            .values()
            .filter(|c| matches_all_tags(tags, &c.tags))
            .cloned()
            .collect())
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
        // Phase 1: capture the (id, key) we need from the descriptor
        // while holding the data lock. Drop the lock before any
        // `await` to avoid borrow-checker issues and to keep the
        // critical section tight.
        let (var_id, value_key) = {
            let mut d = self.data.lock();
            let v = d
                .get_mut(id)
                .ok_or_else(|| CoreError::NotFound(UniqueId(id.0.clone())))?;
            if let Some(k) = p.key {
                v.key = k;
            }
            (v.id.clone(), v.key.clone())
        };

        // Phase 2: if the caller passed a data payload, persist it to
        // the value store keyed by the variable's own key. This makes
        // `update` the high-level "modify anything" entry point
        // while keeping `upsert_value` available for low-level
        // access by an explicit value key.
        if let Some(payload) = p.data {
            self.upsert_value(&var_id, &value_key, payload).await?;
        }

        // Phase 3: re-read and return the (now-mutated) descriptor.
        Ok(self
            .data
            .lock()
            .get(&var_id)
            .cloned()
            .expect("descriptor must still exist after update"))
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
    async fn read_value(
        &self,
        var_id: &ContextVariableId,
        key: &str,
    ) -> CoreResult<Option<ContextVariableValue>> {
        Ok(self
            .values
            .lock()
            .get(&(var_id.clone(), key.to_string()))
            .cloned())
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
    async fn update(&self, id: &TagId, params: TagUpdateParams) -> CoreResult<Tag> {
        let mut d = self.data.lock();
        let t = d
            .get_mut(id)
            .ok_or_else(|| CoreError::NotFound(UniqueId(id.0.clone())))?;
        if let Some(name) = params.name {
            t.name = name;
        }
        Ok(t.clone())
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
        ContextVariable, ContextVariableId, ContextVariableUpdateParams, Criticality,
        GuidelineContent, JourneyNode, JourneyNodeId, RelationshipEntityKind, RelationshipKind,
        TagUpdateParams, ToolKind, ToolUpdateParams,
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
        let r = Relationship::new(
            entity_a.clone(),
            entity_b.clone(),
            RelationshipKind::Excludes,
        );
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

    /// `InMemoryAgentStore::list` must apply its `tags` filter rather
    /// than return every stored agent. The trait contract is:
    /// empty `tags` ⇒ no filter; non-empty `tags` ⇒ return only
    /// agents whose `tags` vector contains **all** of the requested
    /// ids (parlcant semantics: `if tags and not all(t in self.tags
    /// for t in tags): continue`).
    #[tokio::test]
    async fn in_memory_agent_store_list_filters_by_tags() {
        let s = InMemoryAgentStore::new();
        let tagged = TagId::new();
        let other = TagId::new();

        // Agent carrying only `tagged`.
        let mut a1 = Agent::new("a1", "first");
        a1.tags = vec![tagged.clone()];
        s.create(a1.clone()).await.unwrap();

        // Agent carrying both `tagged` and `other`.
        let mut a2 = Agent::new("a2", "second");
        a2.tags = vec![tagged.clone(), other.clone()];
        s.create(a2.clone()).await.unwrap();

        // Untagged agent.
        let a3 = Agent::new("a3", "third");
        s.create(a3.clone()).await.unwrap();

        // Empty tag list ⇒ no filter, all three returned.
        let all = s.list(&[]).await.unwrap();
        assert_eq!(all.len(), 3, "empty tag filter must return every agent");

        // Single matching tag ⇒ both tagged agents, in insertion
        // order (HashMap iteration is non-deterministic so we sort
        // by name for the assertion).
        let mut by_tagged = s.list(std::slice::from_ref(&tagged)).await.unwrap();
        by_tagged.sort_by(|x, y| x.name.cmp(&y.name));
        let names: Vec<&str> = by_tagged.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["a1", "a2"],
            "tag filter must drop untagged agent and keep every agent that carries the tag"
        );

        // Two required tags ⇒ only the agent that carries **both**.
        let mut by_both = s.list(&[tagged.clone(), other.clone()]).await.unwrap();
        by_both.sort_by(|x, y| x.name.cmp(&y.name));
        let names: Vec<&str> = by_both.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["a2"],
            "AND-semantics: agent must carry every requested tag"
        );

        // Tag no agent carries ⇒ empty result.
        let lonely = TagId::new();
        let none = s.list(&[lonely]).await.unwrap();
        assert!(
            none.is_empty(),
            "filter for a tag nobody carries must be empty"
        );
    }

    /// `InMemoryGuidelineStore::list` must apply **both** the
    /// `agent_id` and the `tags` filter rather than collapse to
    /// agent-only. Tag filter follows the same parlcant
    /// AND-semantics used by [`in_memory_agent_store_list_filters_by_tags`]:
    /// empty `tags` ⇒ no tag filter; non-empty ⇒ guideline must
    /// carry every requested tag.
    #[tokio::test]
    async fn in_memory_guideline_store_list_filters_by_agent_and_tags() {
        let s = InMemoryGuidelineStore::new();
        let agent_x = AgentId::new();
        let agent_y = AgentId::new();
        let tag_t = TagId::new();
        let tag_other = TagId::new();
        let tag_lonely = TagId::new();

        // X, tag T
        let mut g1 = Guideline::new(
            GuidelineContent {
                condition: "c1".into(),
                action: "a".into(),
                description: None,
            },
            &agent_x,
            true,
            0,
        );
        g1.tags = vec![tag_t.clone()];
        let id1 = g1.id.clone();
        s.create(g1).await.unwrap();

        // X, untagged
        let g2 = Guideline::new(
            GuidelineContent {
                condition: "c2".into(),
                action: "a".into(),
                description: None,
            },
            &agent_x,
            true,
            0,
        );
        let id2 = g2.id.clone();
        s.create(g2).await.unwrap();

        // X, tag T + tag_other
        let mut g3 = Guideline::new(
            GuidelineContent {
                condition: "c3".into(),
                action: "a".into(),
                description: None,
            },
            &agent_x,
            true,
            0,
        );
        g3.tags = vec![tag_t.clone(), tag_other.clone()];
        let id3 = g3.id.clone();
        s.create(g3).await.unwrap();

        // Y, tag T (must not leak into agent-X queries)
        let mut g4 = Guideline::new(
            GuidelineContent {
                condition: "c4".into(),
                action: "a".into(),
                description: None,
            },
            &agent_y,
            true,
            0,
        );
        g4.tags = vec![tag_t.clone()];
        let id4 = g4.id.clone();
        s.create(g4).await.unwrap();

        // Empty tag filter ⇒ all agent-X guidelines (Y must be excluded).
        let mut x_all = s.list(&agent_x, &[]).await.unwrap();
        x_all.sort_by(|a, b| a.content.condition.cmp(&b.content.condition));
        let x_all_ids: std::collections::HashSet<_> = x_all.iter().map(|g| g.id.clone()).collect();
        assert_eq!(
            x_all_ids,
            [id1.clone(), id2.clone(), id3.clone()]
                .into_iter()
                .collect(),
            "agent filter alone must drop agent-Y guideline"
        );

        // Single tag T on agent X ⇒ g1, g3 (g2 is untagged, g4 is other agent).
        let mut x_t = s
            .list(&agent_x, std::slice::from_ref(&tag_t))
            .await
            .unwrap();
        x_t.sort_by(|a, b| a.content.condition.cmp(&b.content.condition));
        let x_t_ids: std::collections::HashSet<_> = x_t.iter().map(|g| g.id.clone()).collect();
        assert_eq!(
            x_t_ids,
            [id1.clone(), id3.clone()].into_iter().collect(),
            "tag filter on agent X must drop untagged g2 and cross-agent g4"
        );

        // AND-semantics: tag T + tag_other on agent X ⇒ only g3.
        let x_both = s
            .list(&agent_x, &[tag_t.clone(), tag_other.clone()])
            .await
            .unwrap();
        assert_eq!(
            x_both.iter().map(|g| g.id.clone()).collect::<Vec<_>>(),
            vec![id3.clone()],
            "AND-semantics: guideline must carry every requested tag"
        );

        // Tag no guideline carries ⇒ empty.
        assert!(s
            .list(&agent_x, std::slice::from_ref(&tag_lonely))
            .await
            .unwrap()
            .is_empty());

        // Same tag T on agent Y ⇒ only g4 (proves agent_id filter is
        // applied first / in combination, not bypassed by tag).
        let y_t = s
            .list(&agent_y, std::slice::from_ref(&tag_t))
            .await
            .unwrap();
        assert_eq!(
            y_t.iter().map(|g| g.id.clone()).collect::<Vec<_>>(),
            vec![id4.clone()],
            "agent-Y tag filter must not see agent-X guidelines even when their tags match"
        );
    }

    /// `InMemoryCustomerStore::list` must apply its `tags` filter
    /// rather than return every stored customer. Same parlcant
    /// AND-semantics as the agent and guideline stores: empty
    /// `tags` ⇒ no filter; non-empty ⇒ customer must carry every
    /// requested tag.
    #[tokio::test]
    async fn in_memory_customer_store_list_filters_by_tags() {
        let s = InMemoryCustomerStore::new();
        let tag_t = TagId::new();
        let tag_other = TagId::new();
        let tag_lonely = TagId::new();

        // Customer carrying only `tag_t`.
        let mut c1 = Customer::new("c1");
        c1.tags = vec![tag_t.clone()];
        let id1 = c1.id.clone();
        s.create(c1).await.unwrap();

        // Customer carrying `tag_t` + `tag_other`.
        let mut c2 = Customer::new("c2");
        c2.tags = vec![tag_t.clone(), tag_other.clone()];
        let id2 = c2.id.clone();
        s.create(c2).await.unwrap();

        // Untagged customer.
        let c3 = Customer::new("c3");
        let id3 = c3.id.clone();
        s.create(c3).await.unwrap();

        // Empty tag filter ⇒ all three.
        let all_ids: std::collections::HashSet<_> = s
            .list(&[])
            .await
            .unwrap()
            .iter()
            .map(|c| c.id.clone())
            .collect();
        assert_eq!(
            all_ids,
            [id1.clone(), id2.clone(), id3.clone()]
                .into_iter()
                .collect(),
            "empty tag filter must return every customer"
        );

        // Single tag T ⇒ c1, c2 (c3 untagged is dropped).
        let mut by_t = s.list(std::slice::from_ref(&tag_t)).await.unwrap();
        by_t.sort_by(|a, b| a.name.cmp(&b.name));
        let t_ids: Vec<_> = by_t.iter().map(|c| c.id.clone()).collect();
        assert_eq!(
            t_ids,
            vec![id1.clone(), id2.clone()],
            "tag filter must drop the untagged customer"
        );

        // AND-semantics: tag T + tag_other ⇒ only c2.
        let by_both = s.list(&[tag_t.clone(), tag_other.clone()]).await.unwrap();
        let both_ids: Vec<_> = by_both.iter().map(|c| c.id.clone()).collect();
        assert_eq!(
            both_ids,
            vec![id2.clone()],
            "AND-semantics: customer must carry every requested tag"
        );

        // Tag nobody carries ⇒ empty.
        assert!(s.list(&[tag_lonely]).await.unwrap().is_empty());
    }

    /// Regression: `InMemoryTagStore` must implement the `update`
    /// method declared on the `TagStore` trait. Without it the
    /// workspace fails to compile (`E0046: not all trait items
    /// implemented`). The behaviour is the same partial-update
    /// pattern used by the other in-memory stores: a missing field
    /// in the params leaves the corresponding attribute untouched.
    #[tokio::test]
    async fn in_memory_tag_store_update_renames_tag() {
        let s = InMemoryTagStore::new();
        let original = Tag::new("old-name");
        let id = original.id.clone();
        s.create(original.clone()).await.unwrap();

        // Partial update: only name is changed.
        let updated = s
            .update(
                &id,
                TagUpdateParams {
                    name: Some("new-name".into()),
                },
            )
            .await
            .unwrap();
        assert_eq!(updated.name, "new-name");
        assert_eq!(updated.id, id, "update must preserve identity");

        // Persisted state reflects the rename.
        let loaded = s.read(&id).await.unwrap().expect("tag should still exist");
        assert_eq!(loaded.name, "new-name");
    }

    /// Regression: \`InMemoryToolStore\` must implement the
    /// \`update\` method declared on the \`ToolStore\` trait.
    /// Without it the workspace fails to compile (\`E0046: not all
    /// trait items implemented\`). Mirrors the partial-update
    /// pattern used by the other in-memory stores.
    #[tokio::test]
    async fn in_memory_tool_store_update_renames_tool() {
        let s = InMemoryToolStore::new();
        let original = Tool {
            id: ToolId::new(),
            name: "orig".into(),
            description: "d".into(),
            parameters_schema: serde_json::json!({"type": "object"}),
            kind: ToolKind::Local,
            creation_utc: chrono::Utc::now(),
        };
        let id = original.id.clone();
        s.create(original.clone()).await.unwrap();

        let updated = s
            .update(
                &id,
                ToolUpdateParams {
                    name: Some("renamed".into()),
                    description: Some("new desc".into()),
                    parameters_schema: None,
                },
            )
            .await
            .unwrap();

        assert_eq!(updated.name, "renamed");
        assert_eq!(updated.description, "new desc");
        // Unset fields stay untouched.
        assert_eq!(updated.kind, ToolKind::Local);
        assert_eq!(updated.id, id, "update must preserve identity");

        // Persisted state reflects the change.
        let loaded = s.read(&id).await.unwrap().expect("tool should still exist");
        assert_eq!(loaded.name, "renamed");
    }

    /// Regression: \`ContextVariableStore::update\` must persist the
    /// \`data\` field carried by \`ContextVariableUpdateParams\`.
    /// The current implementation silently drops it (\`let _ = p.data\`).
    /// The contract is: if a caller passes \`data: Some(payload)\`, that
    /// payload should end up in the value store keyed by the variable's
    /// own key. Reading the value back via \`upsert_value\` or any
    /// higher-level read should return the new payload.
    #[tokio::test]
    async fn context_variable_update_persists_data_field() {
        let s = InMemoryContextVariableStore::new();
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

        // Update with a new data payload.
        let payload = serde_json::json!({"amount": 42, "currency": "USD"});
        s.update(
            &id,
            ContextVariableUpdateParams {
                key: None,
                data: Some(payload.clone()),
            },
        )
        .await
        .expect("update with data should succeed");

        // The payload must be readable via read_value keyed by the
        // variable's own key.
        let value = s
            .read_value(&id, "balance")
            .await
            .unwrap()
            .expect("update() should have persisted a value");
        assert_eq!(
            value.data, payload,
            "update() must persist the data field on the value store"
        );
    }
}

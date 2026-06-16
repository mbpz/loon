//! Application-level wrapper around `SessionStore`.

use std::sync::Arc;

use loon_core::stores::SessionStore;
use loon_core::{
    AgentId, CoreResult, CustomerId, Event, Session, SessionId, SessionMode, SessionUpdateParams,
};

#[derive(Debug, Clone)]
pub struct SessionCreateParams {
    pub agent_id: AgentId,
    pub customer_id: Option<CustomerId>,
    pub title: Option<String>,
    pub mode: Option<SessionMode>,
}

pub struct SessionAppModule {
    pub store: Arc<dyn SessionStore>,
}

impl SessionAppModule {
    pub fn new(store: Arc<dyn SessionStore>) -> Self {
        Self { store }
    }

    pub async fn create_session(&self, params: SessionCreateParams) -> CoreResult<Session> {
        let mut s = Session::new(&params.agent_id);
        s.customer_id = params.customer_id;
        s.title = params.title;
        if let Some(m) = params.mode {
            s.mode = m;
        }
        self.store.create(s).await
    }

    pub async fn read_session(&self, id: &SessionId) -> CoreResult<Option<Session>> {
        self.store.read(id).await
    }

    pub async fn update_session(
        &self,
        id: &SessionId,
        params: SessionUpdateParams,
    ) -> CoreResult<Session> {
        self.store.update(id, params).await
    }

    pub async fn delete_session(&self, id: &SessionId) -> CoreResult<()> {
        self.store.delete(id).await
    }

    pub async fn list_sessions(
        &self,
        agent_id: Option<&AgentId>,
        customer_id: Option<&CustomerId>,
    ) -> CoreResult<Vec<Session>> {
        self.store.list(agent_id, customer_id).await
    }

    pub async fn append_event(&self, session_id: SessionId, event: Event) -> CoreResult<Event> {
        self.store.create_event(session_id, event).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use loon_core::EventUpdateParams;
    use parking_lot::Mutex;
    use std::collections::HashMap;

    pub struct FakeSessionStore {
        pub sessions: Mutex<HashMap<SessionId, Session>>,
        pub events: Mutex<HashMap<SessionId, Vec<Event>>>,
    }
    impl FakeSessionStore {
        pub fn new() -> Self {
            Self {
                sessions: Mutex::new(HashMap::new()),
                events: Mutex::new(HashMap::new()),
            }
        }
    }

    #[async_trait]
    impl SessionStore for FakeSessionStore {
        async fn create(&self, s: Session) -> CoreResult<Session> {
            let id = s.id.clone();
            self.sessions.lock().insert(id, s.clone());
            Ok(s)
        }
        async fn read(&self, id: &SessionId) -> CoreResult<Option<Session>> {
            Ok(self.sessions.lock().get(id).cloned())
        }
        async fn update(
            &self,
            id: &SessionId,
            p: SessionUpdateParams,
        ) -> CoreResult<Session> {
            let mut g = self.sessions.lock();
            let s = g.get_mut(id).unwrap();
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
            Ok(())
        }
        async fn list(
            &self,
            _agent_id: Option<&AgentId>,
            _customer_id: Option<&CustomerId>,
        ) -> CoreResult<Vec<Session>> {
            Ok(self.sessions.lock().values().cloned().collect())
        }
        async fn create_event(
            &self,
            session_id: SessionId,
            event: Event,
        ) -> CoreResult<Event> {
            self.events
                .lock()
                .entry(session_id)
                .or_default()
                .push(event.clone());
            Ok(event)
        }
        async fn update_event(
            &self,
            _session_id: &SessionId,
            _event_id: &loon_core::EventId,
            _p: EventUpdateParams,
        ) -> CoreResult<Event> {
            unimplemented!()
        }
        async fn read_events(&self, session_id: &SessionId) -> CoreResult<Vec<Event>> {
            Ok(self
                .events
                .lock()
                .get(session_id)
                .cloned()
                .unwrap_or_default())
        }
        async fn find_events(&self, session_id: &SessionId) -> CoreResult<Vec<Event>> {
            self.read_events(session_id).await
        }
    }

    fn fake_event(source: loon_core::EventSource) -> Event {
        Event {
            id: loon_core::EventId::new(),
            source,
            kind: loon_core::EventKind::Message,
            trace_id: "t".into(),
            data: serde_json::json!({"text": "hi"}),
            metadata: None,
            creation_utc: chrono::Utc::now(),
        }
    }

    #[tokio::test]
    async fn session_create_and_read() {
        let store: Arc<dyn SessionStore> = Arc::new(FakeSessionStore::new());
        let module = SessionAppModule::new(store);
        let s = module
            .create_session(SessionCreateParams {
                agent_id: AgentId::new(),
                customer_id: None,
                title: Some("t".into()),
                mode: None,
            })
            .await
            .unwrap();
        let loaded = module.read_session(&s.id).await.unwrap().unwrap();
        assert_eq!(loaded.title.as_deref(), Some("t"));
    }

    #[tokio::test]
    async fn session_append_event() {
        let store: Arc<dyn SessionStore> = Arc::new(FakeSessionStore::new());
        let module = SessionAppModule::new(store);
        let s = module
            .create_session(SessionCreateParams {
                agent_id: AgentId::new(),
                customer_id: None,
                title: None,
                mode: None,
            })
            .await
            .unwrap();
        let ev = module
            .append_event(s.id.clone(), fake_event(loon_core::EventSource::AiAgent))
            .await
            .unwrap();
        assert_eq!(ev.source, loon_core::EventSource::AiAgent);
    }
}

use async_trait::async_trait;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

use crate::tool_service::ToolService;
use crate::CoreResult;

/// Registry mapping service name to a concrete service implementation.
/// The phase-1 contract is `ToolService`; later stages may add
/// additional service kinds.
#[async_trait]
pub trait ServiceRegistry: Send + Sync {
    async fn read_tool_service(&self, name: &str) -> CoreResult<Arc<dyn ToolService>>;
    async fn register(&self, name: &str, service: Arc<dyn ToolService>) -> CoreResult<()>;
    async fn list_services(&self) -> CoreResult<Vec<String>>;
}

/// In-memory service registry. Suitable for tests and single-process deployments.
pub struct InMemoryServiceRegistry {
    services: RwLock<HashMap<String, Arc<dyn ToolService>>>,
}

impl InMemoryServiceRegistry {
    pub fn new() -> Self {
        Self {
            services: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl ServiceRegistry for InMemoryServiceRegistry {
    async fn read_tool_service(&self, name: &str) -> CoreResult<Arc<dyn ToolService>> {
        self.services
            .read()
            .get(name)
            .cloned()
            .ok_or_else(|| crate::CoreError::NotFound(crate::UniqueId(name.into())))
    }

    async fn register(&self, name: &str, service: Arc<dyn ToolService>) -> CoreResult<()> {
        self.services.write().insert(name.to_string(), service);
        Ok(())
    }

    async fn list_services(&self) -> CoreResult<Vec<String>> {
        Ok(self.services.read().keys().cloned().collect())
    }
}

impl Default for InMemoryServiceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool_service::LocalToolService;
    use crate::Tool;
    use crate::ToolId;
    use crate::ToolKind;
    use chrono::Utc;

    fn empty_service() -> Arc<dyn ToolService> {
        Arc::new(LocalToolService::new(vec![Tool {
            id: ToolId::new(),
            name: "t".into(),
            description: "".into(),
            parameters_schema: serde_json::Value::Null,
            kind: ToolKind::Local,
            creation_utc: Utc::now(),
        }]))
    }

    #[tokio::test]
    async fn register_then_read_returns_service() {
        let reg = InMemoryServiceRegistry::new();
        reg.register("svc", empty_service()).await.unwrap();
        let svc = reg.read_tool_service("svc").await.unwrap();
        let tools = svc.list_tools().await.unwrap();
        assert_eq!(tools.len(), 1);
    }
}

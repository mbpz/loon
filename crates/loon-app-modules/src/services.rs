//! Application-level wrapper around `ServiceRegistry`. In phase 1 the
//! registry only stores tool services, but the trait leaves room for
//! future service kinds.

use std::sync::Arc;

use loon_core::{CoreResult, ServiceRegistry, ToolService};

pub struct ServicesAppModule {
    pub registry: Arc<dyn ServiceRegistry>,
}

impl ServicesAppModule {
    pub fn new(registry: Arc<dyn ServiceRegistry>) -> Self {
        Self { registry }
    }

    pub async fn register_service(
        &self,
        name: &str,
        service: Arc<dyn ToolService>,
    ) -> CoreResult<()> {
        self.registry.register(name, service).await
    }

    pub async fn read_tool_service(&self, name: &str) -> CoreResult<Arc<dyn ToolService>> {
        self.registry.read_tool_service(name).await
    }

    pub async fn list_services(&self) -> CoreResult<Vec<String>> {
        self.registry.list_services().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use loon_core::{InMemoryServiceRegistry, JsonValue, LocalToolService, Tool, ToolId, ToolKind};

    fn fake_service() -> Arc<dyn ToolService> {
        Arc::new(LocalToolService::new(vec![Tool {
            id: ToolId::new(),
            name: "echo".into(),
            description: "".into(),
            parameters_schema: JsonValue::Null,
            kind: ToolKind::Local,
            creation_utc: chrono::Utc::now(),
        }]))
    }

    #[tokio::test]
    async fn services_register_read_list() {
        let registry: Arc<dyn ServiceRegistry> = Arc::new(InMemoryServiceRegistry::new());
        let module = ServicesAppModule::new(registry);
        module
            .register_service("svc", fake_service())
            .await
            .unwrap();
        let svc = module.read_tool_service("svc").await.unwrap();
        let tools = svc.list_tools().await.unwrap();
        assert_eq!(tools.len(), 1);
        let names = module.list_services().await.unwrap();
        assert_eq!(names, vec!["svc".to_string()]);
    }
}

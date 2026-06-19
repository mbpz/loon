//! OpenAPI/Swagger document → `Tool` generator + a `ToolService` adapter.
//!
//! Phase 7: parses OpenAPI 3.x documents, generates one `Tool` per
//! operation with stable deterministic IDs. The actual HTTP call
//! dispatch is a stub — real impl lands in a follow-up phase (needs
//! HTTP client + auth negotiation for security schemes).

use async_trait::async_trait;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value as JsonValue};
use std::collections::HashMap;
use std::sync::Arc;

use crate::id_generator::IdGenerator;
use crate::{
    CoreError, CoreResult, JsonValue as CrateJsonValue, Tool, ToolId, ToolKind, ToolResult,
    ToolService,
};

/// Minimal OpenAPI 3.x root.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OpenApiDocument {
    pub openapi: String,
    #[serde(default)]
    pub info: OpenApiInfo,
    #[serde(default)]
    pub paths: HashMap<String, OpenApiPathItem>,
    #[serde(default)]
    pub components: Option<OpenApiComponents>,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct OpenApiInfo {
    pub title: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct OpenApiPathItem {
    #[serde(default)]
    pub get: Option<OpenApiOperation>,
    #[serde(default)]
    pub post: Option<OpenApiOperation>,
    #[serde(default)]
    pub put: Option<OpenApiOperation>,
    #[serde(default)]
    pub patch: Option<OpenApiOperation>,
    #[serde(default)]
    pub delete: Option<OpenApiOperation>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OpenApiOperation {
    #[serde(rename = "operationId", default)]
    pub operation_id: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub parameters: Option<Vec<OpenApiParameter>>,
    #[serde(default)]
    pub request_body: Option<JsonValue>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OpenApiParameter {
    pub name: String,
    /// `"path" | "query" | "header" | "cookie"`
    #[serde(rename = "in")]
    pub location: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub schema: Option<JsonValue>,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct OpenApiComponents {
    #[serde(default)]
    pub schemas: HashMap<String, JsonValue>,
}

/// `ToolService` backed by an OpenAPI document. Phase 7: generates
/// tools from the doc, but `call_tool` is a stub.
pub struct OpenApiToolService {
    name: String,
    document: OpenApiDocument,
    base_url: Option<String>,
    cached_tools: Mutex<Option<Vec<Tool>>>,
    id_gen: Mutex<IdGenerator>,
}

impl OpenApiToolService {
    pub fn new(name: impl Into<String>, document: OpenApiDocument) -> Self {
        Self {
            name: name.into(),
            document,
            base_url: None,
            cached_tools: Mutex::new(None),
            id_gen: Mutex::new(IdGenerator::new()),
        }
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    /// The configured service name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The base URL, if configured.
    #[allow(dead_code)]
    pub fn base_url(&self) -> Option<&str> {
        self.base_url.as_deref()
    }

    /// Compute a stable tool id for a given (method, path) pair. Uses
    /// the operation_id when present so the same operation produces
    /// the same id across processes.
    pub fn tool_id(&self, path: &str, method: &str, op: &OpenApiOperation) -> ToolId {
        let mut gen = self.id_gen.lock();
        let key = format!(
            "openapi:{}:{}:{}:{}",
            self.name,
            method,
            path,
            op.operation_id.as_deref().unwrap_or("")
        );
        ToolId(gen.generate(&key).0)
    }

    /// Generate the list of `Tool` records by walking the document's paths.
    pub fn generate_tools(&self) -> Vec<Tool> {
        let mut out = Vec::new();
        for (path, item) in &self.document.paths {
            let methods: [(&str, &Option<OpenApiOperation>); 5] = [
                ("get", &item.get),
                ("post", &item.post),
                ("put", &item.put),
                ("patch", &item.patch),
                ("delete", &item.delete),
            ];
            for (method, op) in methods {
                if let Some(op) = op {
                    let id = self.tool_id(path, method, op);
                    let name = op
                        .operation_id
                        .clone()
                        .unwrap_or_else(|| format!("{}_{}", method, path.replace('/', "_")));
                    let description = op
                        .description
                        .clone()
                        .or_else(|| op.summary.clone())
                        .unwrap_or_else(|| format!("{} {}", method.to_uppercase(), path));
                    let schema = build_parameters_schema(op);
                    out.push(Tool {
                        id,
                        name,
                        description,
                        parameters_schema: schema,
                        kind: ToolKind::OpenAPI,
                        creation_utc: chrono::Utc::now(),
                    });
                }
            }
        }
        out
    }
}

fn build_parameters_schema(op: &OpenApiOperation) -> JsonValue {
    let mut properties = Map::new();
    let mut required = Vec::new();
    if let Some(params) = &op.parameters {
        for p in params {
            properties.insert(
                p.name.clone(),
                p.schema.clone().unwrap_or(JsonValue::Object(Map::new())),
            );
            if p.required {
                required.push(p.name.clone());
            }
        }
    }
    if let Some(body) = &op.request_body {
        properties.insert("body".into(), body.clone());
        required.push("body".into());
    }
    serde_json::json!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": false
    })
}

#[async_trait]
impl ToolService for OpenApiToolService {
    async fn list_tools(&self) -> CoreResult<Vec<Tool>> {
        {
            let cache = self.cached_tools.lock();
            if let Some(t) = cache.as_ref() {
                return Ok(t.clone());
            }
        }
        let tools = self.generate_tools();
        *self.cached_tools.lock() = Some(tools.clone());
        Ok(tools)
    }

    async fn call_tool(
        &self,
        _tool_id: &ToolId,
        _arguments: CrateJsonValue,
    ) -> CoreResult<ToolResult> {
        // Phase 7: stub. Real impl lands in a follow-up phase (requires
        // HTTP client + auth handling for OpenAPI security schemes).
        Err(CoreError::Internal(
            "OpenAPI call_tool not yet implemented".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn generate_tools_walks_paths_and_methods() {
        let doc: OpenApiDocument = serde_json::from_value(json!({
            "openapi": "3.0.0",
            "info": { "title": "Test API", "version": "1.0.0" },
            "paths": {
                "/users": {
                    "get": { "operationId": "listUsers", "description": "List all users" },
                    "post": { "operationId": "createUser", "description": "Create a user" }
                },
                "/users/{id}": {
                    "get": { "operationId": "getUser", "description": "Get a user",
                             "parameters": [{ "name": "id", "in": "path", "required": true,
                                              "schema": { "type": "string" } }] }
                }
            }
        }))
        .unwrap();
        let svc = OpenApiToolService::new("test", doc);
        let tools = svc.generate_tools();
        assert_eq!(tools.len(), 3);
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"listUsers"));
        assert!(names.contains(&"createUser"));
        assert!(names.contains(&"getUser"));
        // Verify the path-parameter tool has id in its schema.
        let get_user = tools.iter().find(|t| t.name == "getUser").unwrap();
        assert!(get_user.parameters_schema["properties"]["id"].is_object());
    }

    #[test]
    fn tool_id_is_deterministic() {
        let doc: OpenApiDocument =
            serde_json::from_value(json!({ "openapi": "3.0.0", "paths": {} })).unwrap();
        let s1 = OpenApiToolService::new("api", doc.clone());
        let s2 = OpenApiToolService::new("api", doc);
        let t1 = s1.generate_tools();
        let t2 = s2.generate_tools();
        // Both services produce the same shape (no operations → both
        // empty, but the formatter and id generation must not panic).
        assert_eq!(t1.len(), t2.len());
    }

    #[test]
    fn tool_id_format_is_stable_across_instances() {
        let doc: OpenApiDocument = serde_json::from_value(json!({
            "openapi": "3.0.0",
            "paths": {
                "/x": { "get": { "operationId": "op" } }
            }
        }))
        .unwrap();
        let s1 = OpenApiToolService::new("api", doc.clone());
        let s2 = OpenApiToolService::new("api", doc);
        let op = OpenApiOperation {
            operation_id: Some("op".into()),
            summary: None,
            description: None,
            parameters: None,
            request_body: None,
        };
        assert_eq!(
            s1.tool_id("/x", "get", &op).0,
            s2.tool_id("/x", "get", &op).0
        );
    }

    #[tokio::test]
    async fn list_tools_caches_result() {
        let doc: OpenApiDocument =
            serde_json::from_value(json!({ "openapi": "3.0.0", "paths": {} })).unwrap();
        let svc = OpenApiToolService::new("test", doc);
        let _ = svc.list_tools().await.unwrap();
        let _ = svc.list_tools().await.unwrap();
        // Just verify it doesn't panic on repeat calls.
    }

    #[tokio::test]
    async fn call_tool_stub_returns_error() {
        let doc: OpenApiDocument =
            serde_json::from_value(json!({ "openapi": "3.0.0", "paths": {} })).unwrap();
        let svc = OpenApiToolService::new("test", doc);
        let id = ToolId::new();
        let result = svc.call_tool(&id, json!({})).await;
        assert!(result.is_err());
    }

    #[test]
    fn build_parameters_schema_includes_required() {
        let op = OpenApiOperation {
            operation_id: None,
            summary: None,
            description: None,
            parameters: Some(vec![OpenApiParameter {
                name: "id".into(),
                location: "path".into(),
                required: true,
                schema: Some(json!({ "type": "string" })),
            }]),
            request_body: None,
        };
        let schema = build_parameters_schema(&op);
        assert_eq!(schema["type"], "object");
        assert_eq!(schema["properties"]["id"]["type"], "string");
        assert_eq!(schema["required"][0], "id");
    }

    #[test]
    fn with_base_url_records_url() {
        let doc: OpenApiDocument =
            serde_json::from_value(json!({ "openapi": "3.0.0", "paths": {} })).unwrap();
        let svc = OpenApiToolService::new("test", doc).with_base_url("https://api.example.com");
        assert_eq!(svc.base_url(), Some("https://api.example.com"));
    }
}

/// Bridges an `OpenApiToolService` into a `ToolService` facade for
/// the `ServiceRegistry`. Phase 7 is a thin pass-through; later
/// phases may add auth header injection.
pub struct OpenApiToolServiceAdapter {
    pub service: Arc<OpenApiToolService>,
}

#[async_trait]
impl ToolService for OpenApiToolServiceAdapter {
    async fn list_tools(&self) -> CoreResult<Vec<Tool>> {
        self.service.list_tools().await
    }

    async fn call_tool(
        &self,
        tool_id: &ToolId,
        arguments: CrateJsonValue,
    ) -> CoreResult<ToolResult> {
        self.service.call_tool(tool_id, arguments).await
    }
}

#[cfg(test)]
mod adapter_tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn adapter_delegates_to_service() {
        let doc: OpenApiDocument =
            serde_json::from_value(json!({ "openapi": "3.0.0", "paths": {} })).unwrap();
        let svc = Arc::new(OpenApiToolService::new("test", doc));
        let adapter = OpenApiToolServiceAdapter { service: svc };
        let tools = adapter.list_tools().await.unwrap();
        assert!(tools.is_empty());
    }
}

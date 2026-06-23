# Loon 补强 2 — Implementation Plan

**Spec:** `docs/superpowers/specs/2026-06-23-loon-strengthening-spec.md`

## Stage 1: P1.1 — Document-backed EntityQueries

### Task 1.1: `DocumentDatabaseHandle` + `DocumentCollectionHandle` traits

**Files:** `crates/loon-persistence/src/document_handle.rs` (new)

Define type-erased traits:

```rust
use async_trait::async_trait;
use std::sync::Arc;
use crate::{BaseDocument, DocumentFilter, DocumentUpdate, UpdateResult, DeleteResult, PersistenceResult};

#[async_trait]
pub trait DocumentDatabaseHandle: Send + Sync {
    async fn collection(&self, name: &str) -> PersistenceResult<Arc<dyn DocumentCollectionHandle>>;
}

#[async_trait]
pub trait DocumentCollectionHandle: Send + Sync {
    async fn insert_one(&self, doc: BaseDocument) -> PersistenceResult<()>;
    async fn find_one(&self, filter: &DocumentFilter) -> PersistenceResult<Option<BaseDocument>>;
    async fn find(&self, filter: &DocumentFilter) -> PersistenceResult<Vec<BaseDocument>>;
    async fn update_one(&self, filter: &DocumentFilter, update: DocumentUpdate) -> PersistenceResult<UpdateResult>;
    async fn delete_one(&self, filter: &DocumentFilter) -> PersistenceResult<DeleteResult>;
}
```

### Task 1.2: Implement DocumentDatabaseHandle on JsonFileDocumentDatabase

**Files:** `crates/loon-persistence/src/backends/json_file.rs`

Wrap the typed collection via JSON round-trip:

```rust
#[async_trait]
impl DocumentDatabaseHandle for JsonFileDocumentDatabase {
    async fn collection(&self, name: &str) -> PersistenceResult<Arc<dyn DocumentCollectionHandle>> {
        let dir = self.root.join(name);
        std::fs::create_dir_all(&dir)?;
        Ok(Arc::new(JsonFileCollectionHandle { dir, ... }))
    }
}

pub struct JsonFileCollectionHandle { dir: PathBuf, cache: Arc<RwLock<HashMap<String, JsonValue>>> }
```

The handle internally caches `serde_json::Value` (= `BaseDocument`). Tests:
1. `handle_round_trip` — insert + find round-trips a JSON object
2. `handle_filter_eq` — find_one with `DocumentFilter::Eq`

### Task 1.3: Same for MongoDocumentDatabase

**Files:** `crates/loon-persistence/src/backends/mongodb.rs`

Same pattern but BSON-backed.

### Task 1.4: DocumentBacked stores

**Files:** `crates/loon-core/src/stores/document_backed.rs` (new)

One per entity (16 total). Pattern:

```rust
pub struct DocumentBackedAgentStore {
    collection: Arc<dyn DocumentCollectionHandle>,
}

#[async_trait]
impl AgentStore for DocumentBackedAgentStore {
    async fn create(&self, a: Agent) -> CoreResult<Agent> {
        let doc = serde_json::to_value(&a).map_err(|e| CoreError::Internal(e.to_string()))?;
        self.collection.insert_one(doc).await.map_err(|e| CoreError::Internal(e.to_string()))?;
        Ok(a)
    }
    // ... read/update/delete/list similarly
}
```

For `update`: read the doc, apply update params on the JSON, write back. For `list`: enumerate all docs, filter by JSON fields.

Tests for each (16 stores × 1 round-trip test = 16 new tests).

### Task 1.5: EntityQueries::from_document_database factory

**Files:** `crates/loon-core/src/entity_cq.rs`

```rust
impl EntityQueries {
    pub async fn from_document_database(handle: Arc<dyn DocumentDatabaseHandle>) -> PersistenceResult<Arc<Self>> {
        let agent_store = Arc::new(DocumentBackedAgentStore { collection: handle.collection("agents").await? });
        // ... 15 more
        Ok(Arc::new(Self { agent_store, ..., journey_guideline_projection }))
    }
}
```

Test: 1 round-trip test that creates an agent + reads it back through `EntityQueries`.

### Task 1.6: Wire SDK::ServerBuilder to use document_db_handle when provided

**Files:** `crates/loon-sdk/src/server.rs`

Add `document_db_handle: Option<Arc<dyn DocumentDatabaseHandle>>` field. Update `build()` to:

```rust
let queries = if let Some(handle) = self.document_db_handle {
    EntityQueries::from_document_database(handle).await?
} else if let Some(q) = self.entity_queries {
    q
} else {
    EntityQueries::in_memory()
};
```

Replace the broken `with_document_db<DB: DocumentDatabase>` with `with_document_db_handle(Arc<dyn DocumentDatabaseHandle>)`.

Test: build with `JsonFileDocumentDatabase` handle, verify queries actually persists.

### Task 1.7: Wire loon-server::run to construct handle

**Files:** `crates/loon-server/src/lib.rs`

For both JsonFile and Mongo branches, get the handle:

```rust
let handle: Arc<dyn DocumentDatabaseHandle> = match &config.persistence.backend {
    JsonFile { root, flush_interval_ms } => Arc::new(JsonFileDocumentDatabase::new(...)?),
    Mongo { uri, database } => Arc::new(MongoDocumentDatabase::connect(uri, database).await?),
};
let server = loon_sdk::Server::builder().with_document_db_handle(handle).with_nlp_service(nlp).build().await?;
```

End-to-end test: create agent via HTTP, restart server, verify agent persists.

**Commit per task. Final commit:** `feat(loon-*): wire document-backed EntityQueries end-to-end`

## Stage 2: P1.2 — SingleToolBatch

### Task 2.1: Implement SingleToolBatch in batcher.rs

**Files:** `crates/loon-engine/src/tool_calling/batcher.rs`

```rust
define_schematic! {
    pub struct ToolArgsOutput { pub arguments_json: String }
}

#[async_trait]
impl ToolCaller for DefaultToolCallBatcher {
    async fn generate_insights(&self, ctx: &EngineContext, guidelines: &[GuidelineMatch]) -> EngineResult<ToolInsights> {
        let mut evaluations = HashMap::new();
        for m in guidelines {
            let associations = self.queries.guideline_tool_association_store
                .list_for_guideline(&m.guideline.id).await?;
            for a in associations {
                evaluations.insert(a.tool_id, ToolCallEvaluation::NeedsToRun);
            }
        }
        Ok(ToolInsights { evaluations })
    }

    async fn call_tools(&self, ctx: &EngineContext, insights: &ToolInsights) -> EngineResult<Vec<ToolExecutionResult>> {
        let mut out = Vec::new();
        for (tool_id, eval) in &insights.evaluations {
            if !matches!(eval, ToolCallEvaluation::NeedsToRun) { continue; }
            let tool = self.queries.tool_store.read(tool_id).await?;
            if tool.is_none() { continue; }
            let tool = tool.unwrap();
            let args = self.generate_tool_args(&tool, ctx).await?;
            let service = self.registry.read_tool_service(&self.find_service_name(&tool)).await?;
            let result = service.call_tool(tool_id, args).await?;
            out.push(ToolExecutionResult { tool_id: tool_id.clone(), result });
        }
        Ok(out)
    }
}
```

`generate_tool_args` builds a prompt from the tool's `parameters_schema` + the interaction, calls `nlp.schematic_generator::<ToolArgsOutput>`, parses `arguments_json` as `JsonValue`.

`DefaultToolCallBatcher` needs new fields: `queries: Arc<EntityQueries>`. Update `AlphaEngine` wiring to pass `queries.clone()`.

Tests:
- `generate_insights_from_associations`: 1 guideline → 2 associated tools → 2 NeedsToRun evaluations
- `call_tools_invokes_service`: fake tool service, verify call_tool called with correct args
- `call_tools_skips_data_already_in_context`: evaluation is `DataAlreadyInContext` → no call

### Task 2.2: Wire DefaultToolCallBatcher with queries reference

**Files:** `crates/loon-sdk/src/server.rs`

In `build()`:

```rust
let tool_caller: Arc<dyn ToolCaller> = Arc::new(DefaultToolCallBatcher {
    nlp: nlp.clone(),
    registry: registry.clone(),
    queries: queries.clone(),
});
```

### Task 2.3: Integration test — full preparation loop calls a tool

**Files:** `crates/loon-engine/src/alpha_engine.rs` test module

Build an engine with a real `LocalToolService` containing one tool. Pre-create a guideline + a guideline_tool_association. Call `process()`. Verify the LocalToolService's handler is invoked.

**Commit per task. Final commit:** `feat(loon-engine): SingleToolBatch + DefaultToolCallBatcher real tool invocation`

## Stage 3: P1.3 — Streaming WS

### Task 3.1: StreamingEventEmitter

**Files:** `crates/loon-server/src/routes/chat.rs`

```rust
pub struct StreamingEventEmitter {
    tx: tokio::sync::mpsc::Sender<OutgoingFrame>,
}

#[async_trait]
impl EventEmitter for StreamingEventEmitter {
    async fn emit_message_event(&self, trace_id: &str, data: MessageEmitData, _: Option<HashMap<String, JsonValue>>) -> EmissionResult<MessageEventHandle> {
        let text = match &data {
            MessageEmitData::Simple(s) => s.clone(),
            MessageEmitData::Structured(m) => m.message.clone(),
        };
        let _ = self.tx.send(OutgoingFrame::AgentMessage(text)).await;
        // Return a synthesized handle whose update is a no-op
        ...
    }
    // emit_status_event, emit_tool_event, emit_custom_event: drop frames (or forward as a different OutgoingFrame variant)
}
```

Test:
- `streaming_emitter_forwards_message_delta`: send 1 message via emit_message_event, verify channel receives it
- `streaming_emitter_handles_buffer_full`: bounded channel, verify backpressure semantics

### Task 3.2: Refactor chat_ws to use mpsc

**Files:** `crates/loon-server/src/routes/chat.rs`

```rust
async fn handle_socket(socket: WebSocket, state: Arc<AppState>, session_id: SessionId) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = tokio::sync::mpsc::channel(64);

    tokio::spawn(async move {
        while let Some(frame) = rx.recv().await {
            let _ = sender.send(Message::Text(frame.to_wire_json().into())).await;
        }
    });

    while let Some(Ok(msg)) = receiver.next().await {
        if let Message::Text(text) = msg {
            if let Ok(parsed) = serde_json::from_str::<JsonValue>(&text) {
                if parsed["type"] == "user_message" {
                    let content = parsed["content"].as_str().unwrap_or("").to_string();
                    let tx = tx.clone();
                    let state = state.clone();
                    let sid = session_id.clone();
                    tokio::spawn(async move {
                        let emitter = StreamingEventEmitter { tx: tx.clone() };
                        let _ = drive_engine(&state, &sid, &content, &emitter).await;
                        let _ = tx.send(OutgoingFrame::Done).await;
                    });
                }
            }
        }
    }
}
```

Test:
- `ws_streams_message_delta`: connect WS, send `user_message`, verify `agent_message` frames arrive before `done`

### Task 3.3: MessageGenerator::generate_streaming actually streams

**Files:** `crates/loon-engine/src/message_generator.rs`

Currently returns a `Stream` with single chunk. Refactor to yield chunks as the LLM emits them (via `StreamingTextGenerator::generate_streaming` if available, else chunk-by-words as a fallback).

**Commit per task. Final commit:** `feat(loon-server,loon-engine): streaming WS chat with EventEmitter → mpsc bridge`

## Stage 4: P2 — SDK Type Completeness + EntityCommands

### Task 4.1: shot_store in EntityQueries

**Files:** `crates/loon-core/src/entity_cq.rs`, `crates/loon-core/src/stores/in_memory.rs`

```rust
pub struct EntityQueries {
    // ... existing fields
    pub shot_store: Arc<dyn ShotStore>,
}

impl EntityQueries {
    pub async fn find_shots_for_agent(&self, agent_id: &AgentId) -> CoreResult<Vec<Shot>> {
        self.shot_store.list(agent_id).await
    }
}
```

Test: round-trip a shot.

### Task 4.2: AnyOf / AllOf in SDK

**Files:** `crates/loon-sdk/src/lib.rs`

```rust
#[derive(Debug, Clone)]
pub struct AnyOf<T>(pub Vec<T>);

#[derive(Debug, Clone)]
pub struct AllOf<T>(pub Vec<T>);

impl<T: Clone> AnyOf<T> {
    pub fn new(items: impl IntoIterator<Item = T>) -> Self { Self(items.into_iter().collect()) }
    pub fn as_slice(&self) -> &[T] { &self.0 }
}
// same for AllOf
```

Test: round-trip construction + iteration.

### Task 4.3: ToolContext + Variable

**Files:** `crates/loon-sdk/src/tool_context.rs` (new), `crates/loon-sdk/src/variable.rs` (new)

```rust
// tool_context.rs
pub struct ToolContext {
    pub agent_id: AgentId,
    pub session_id: SessionId,
    pub customer_id: Option<CustomerId>,
    pub commands: Arc<EntityCommands>,
}

// variable.rs (distinct from ContextVariable)
pub struct Variable<T> {
    pub name: String,
    pub value: T,
}
```

`ToolContext` is passed to local tool handlers. Add `LocalToolService::register_handler_with_context(id, |args, ctx| async {...})`. Test: a handler that uses `ToolContext::commands.upsert_session_labels` to mutate session state.

### Task 4.4: EntityCommands wired into Server + routes

**Files:** `crates/loon-sdk/src/server.rs`, `crates/loon-server/src/routes/sessions.rs`

```rust
pub struct Server {
    pub engine: Arc<dyn Engine>,
    pub queries: Arc<EntityQueries>,
    pub commands: Arc<EntityCommands>,
}
```

In `routes/sessions.rs::update_session`, route writes through `s.server.commands.update_session(...)` instead of `s.server.queries.session_store.update(...)`.

Test: route lifecycle test verifies the change.

**Commit per task. Final commit:** `feat(loon-*): complete SDK type surface + EntityCommands write path`

## Verification (after all stages)

- `cargo test --workspace`: 330+ tests (305 current + ~25 new)
- `cargo clippy --workspace --all-targets -- -D warnings`: clean
- `cargo build --workspace`: clean
- Manual smoke test: `cargo run -p loon-server` with `LOON_CONFIG=/tmp/loon.toml`, create agent via curl, kill server, restart, verify agent persists

## Risk / Rollback

Each stage is independent; if a stage fails, prior stages remain functional. Each commit is reverted with `git revert`.

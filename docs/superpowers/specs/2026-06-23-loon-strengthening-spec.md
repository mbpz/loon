# Loon 补强 2 — 实施 Spec

**Date:** 2026-06-23
**Based on:** `docs/reviews/2026-06-19-loon-final-review.md`
**Scope:** Address P1 critical + P2 easy fixes from final review

## 0. Background

Final review identified 4 critical gaps + several easy wins. 305 tests + clippy clean is the baseline. This spec defines the work that moves loon from "structure-complete" to "production-runnable with real persistence + real tool calls + real streaming."

## 1. Goals

1. **Real persistence:** make `ServerBuilder::with_document_db` actually wire `DocumentDatabase` through to all 16 stores. Currently it's a diagnostics-only no-op.
2. **Real tool calls:** make `DefaultToolCallBatcher` invoke `ToolService::call_tool` with LLM-generated arguments. Currently it returns empty `ToolInsights`.
3. **Real WS streaming:** stream each `agent_message` delta as the engine emits it, not all at once after `process()` returns.
4. **SDK type completeness:** add `AnyOf`, `AllOf`, `ToolContext`, `Variable` (spec §10.2 calls for them).
5. **Plumbing fixes:** add `shot_store` to `EntityQueries`; route writes through `EntityCommands`.

## 2. Non-Goals

- HealthReporter live impl (separate concern, low impact)
- Vector DB integration into AlphaEngine (no caller wants it yet)
- Token budget truncation in PromptBuilder (warning is sufficient for now)
- Anthropic / Gemini live LLM testing (mock-only fine)
- `loon-chat-ui` integration tests (out of Rust scope)

## 3. Architecture Decisions

### 3.1 DocumentDatabase dyn-compatibility

The current `DocumentDatabase` trait:
```rust
async fn get_or_create_collection<TDocument: Document>(&self, ...) -> ...
```
has a generic method, so `Arc<dyn DocumentDatabase>` doesn't compile.

**Decision:** introduce a `DocumentDatabaseHandle` trait that erases the generic by accepting an `&dyn DocumentLoaderAny` (or `Box<dyn Fn(BaseDocument) -> Option<BaseDocument>>`). Each `DocumentBacked*Store` wraps a `Arc<dyn DocumentCollectionHandle>` returning `BaseDocument` (= `serde_json::Value`), and the store deserializes into the typed entity. This is the standard "type-erased collection" pattern.

```rust
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

`JsonFileDocumentDatabase` and `MongoDocumentDatabase` both implement `DocumentDatabaseHandle`. `EntityQueries::from_document_database(handle)` constructs 16 `DocumentBacked*Store` instances and wraps them in `Arc<dyn ...Store>`.

### 3.2 SingleToolBatch

Mirror parlcant's `tool_caller.py::SingleToolBatch`:

1. For each matched guideline, collect associated tools via `guideline_tool_association_store.list_for_guideline(guideline_id)`.
2. For each tool, call `nlp.schematic_generator::<ToolArgs>` with a prompt that includes the tool's `parameters_schema` and the current interaction.
3. Call `tool_service.call_tool(tool_id, args)`.
4. Return `ToolExecutionResult`s.

`ToolInsights::evaluations` is populated with `ToolCallEvaluation::NeedsToRun` for every associated tool. The engine's preparation loop then runs these.

### 3.3 Streaming WS

Replace synchronous `Vec<OutgoingFrame>` return with a `tokio::sync::mpsc::channel`:

```rust
pub async fn process_user_message_streaming(
    state: &AppState,
    session_id: &SessionId,
    content: &str,
    sender: tokio::sync::mpsc::Sender<OutgoingFrame>,
) -> EngineResult<()> {
    let emitter = StreamingEventEmitter::new(sender.clone());
    state.server.engine.process(&ctx, &emitter).await?;
    let _ = sender.send(OutgoingFrame::Done).await;
    Ok(())
}
```

`StreamingEventEmitter` implements `EventEmitter` and on each `emit_message_event` sends a `OutgoingFrame::AgentMessage(delta)` through the channel. WS handler reads from the channel and writes to the socket as messages arrive.

### 3.4 SDK type additions

- `AnyOf<T>` — at-least-one-tag-matches filter
- `AllOf<T>` — all-tags-match filter
- `ToolContext` — passed to tools, exposes session, agent, customer, and a write-back to `EntityCommands`
- `Variable` (distinct from `ContextVariable`) — short-lived per-tool state

These are new types in `loon-sdk/src/lib.rs` (not `loon-core` re-exports). `AnyOf` / `AllOf` are zero-cost wrappers over `Vec<TagId>` with a marker.

### 3.5 ShotStore in EntityQueries

Add `pub shot_store: Arc<dyn ShotStore>` field. Wire in `in_memory()` and `from_document_database()`. Add `find_shots_for_agent(agent_id) -> Vec<Shot>` query method.

### 3.6 EntityCommands wiring

Route `update_session` + `update_context_variable_value` + `upsert_session_labels` calls in `loon-server` through `s.server.commands` instead of `s.server.queries.session_store.update`. Add `commands` to `Server` struct alongside `queries`.

## 4. Test Strategy

- Each `DocumentBacked*Store` gets a round-trip test using `tempfile::tempdir()` + `JsonFileDocumentDatabase`.
- `EntityQueries::from_document_database` gets a round-trip integration test.
- `SingleToolBatch` gets a fake-`ToolService` test verifying argument prompting + invocation.
- `StreamingEventEmitter` gets a unit test verifying mpsc delivery order.
- WS streaming gets an end-to-end test through `axum::Router` + `tokio_tungstenite`.
- SDK type additions get round-trip serialization tests.

## 5. Out-of-Scope Risks

- `DocumentBacked*Store::update` requires a Set/Inc primitive on `DocumentCollection` (already exists) but needs careful field-name mapping per entity. Mitigation: each store has a helper `apply_update_params(&mut JsonValue, params)` that mirrors the InMemory version.
- The `Document` trait requires a `const VERSION: &'static str` and `type Id`. For the 16 entities, we need to either (a) implement `Document` on each entity, or (b) wrap each entity in an `XxxDoc { id: String, version: String, payload: T }`. **Decision: (a)** — implement `Document` directly on `Agent`, `Session`, etc. The `Id` type is the entity's typed ID (e.g. `AgentId`).
- Streaming WS may interact poorly with axum's WebSocket sink if multiple writes happen concurrently. Mitigation: serialize writes through the WS sender side.

## 6. Out-of-Scope Items (Reaffirmed)

- HealthReporter live methods
- Token budget truncation
- Anthropic + Gemini live integration tests
- Preamble message generation (the `should_emit_preamble` path stays as today)

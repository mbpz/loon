# Loon Final Code Review

**Date:** 2026-06-23
**Reviewer:** Claude (self-review, full repo scan)
**Scope:** All 9 Rust crates + e2e tests
**Method:** Spec § cross-reference + grep-based architectural survey

## Executive Summary

loon is in a **production-shaped but not production-ready** state. 305 tests pass, clippy clean, 0 build errors, 68 first-parent commits. All 15 entities + 16 stores + complete CRUD routes + real OpenAI/Anthropic/Gemini providers + MongoDB/JSON persistence + Chroma/Qdrant vector backends + MCP/OpenAPI tool sources + Plugin system + auth + rate limiting are in place.

**The largest gap** is that `ServerBuilder::with_document_db` is a diagnostics-only no-op — `DocumentDatabase` is not dyn-compatible (the `get_or_create_collection<T>` generic method blocks `Arc<dyn DocumentDatabase>`), so every server defaults to `EntityQueries::in_memory()` regardless of what the user passes. The persistence is entirely in-process memory; data does not survive restart unless callers manually wire a database-backed `EntityQueries` and pass it via `with_entity_queries`. **No such factory exists yet.**

Other significant gaps: SDK spec calls for 22 public types (Agent, Guideline, …); core defines them but SDK only re-exports via `pub use loon_core::*` (the **6 entity-handle aliases** `agent_handle`, `journey_handle`, etc. add no new ergonomics). `AnyOf`, `AllOf`, `ToolContext`, `Variable` are spec'd but **not implemented**. `ShotStore` is **missing from `EntityQueries`**.

The engine sub-component impls (RelationalResolver, PromptBuilder, MessageGenerator, indexing) are real but light — they work but have not been validated against parlcant fixtures.

## Spec Coverage Matrix

| Spec section | Impl location | Status |
|---|---|---|
| §3.1 Common types (Version, UniqueId, etc.) | `loon-core/src/common.rs` | ✓ |
| §3.2 ID types (15+) | `loon-core/src/ids.rs` | ✓ |
| §3.3 IdGenerator (xxh3) | `loon-core/src/id_generator.rs` | ✓ |
| §3.4 15 entities + 2 enums | `loon-core/src/{agent,guideline,journey,...}.rs` | ✓ |
| §3.5 15 Store traits | `loon-core/src/stores/*.rs` | ✓ (16 stores including ShotStore) |
| §3.6 EntityQueries / EntityCommands | `loon-core/src/entity_cq.rs:18-200` | ⚠ ShotStore missing from EntityQueries struct |
| §3.7 JourneyGuidelineProjection | `loon-core/src/journey_guideline_projection.rs` | ✓ (BFS implemented) |
| §3.8 Logger / Tracer / Meter / async utils | `loon-core/src/{logger,tracer,meter,async_utils}.rs` | ✓ |
| §3.9 Shot + ShotStore | `loon-core/src/shot.rs`, `stores/shot.rs` | ✓ but EntityQueries doesn't include shot_store |
| §3.10 ServiceRegistry + ToolService + LocalToolService | `loon-core/src/{service_registry,tool_service}.rs` | ✓ |
| §4.1-4.4 EventEmitter / EventBuffer / EventPublisher / MessageEventHandle | `loon-emission/src/*` | ✓ |
| §5.1 13 AppModules | `loon-app-modules/src/*` | ✓ |
| §6.1-6.4 DocumentDatabase + JsonFileDocumentDatabase | `loon-persistence/src/document.rs`, `backends/json_file.rs` | ⚠ DocumentDatabase NOT dyn-compatible (generic method) |
| §6.4 VectorDatabase trait | `loon-persistence/src/vector.rs` | ✓ + Chroma + Qdrant impls |
| §6.5 DocumentStoreMigrationHelper | `loon-persistence/src/migration.rs` | ✓ (real chain resolver) |
| §7.1-7.5 NlpService + Schematic + OpenAI provider | `loon-nlp/src/{service,schematic,providers/openai}.rs` | ✓ + Anthropic + Gemini |
| §8.1-8.4 Engine trait + EngineContext + EntityContext + EngineHooks | `loon-engine/src/{engine,engine_context,entity_context,hooks}.rs` | ✓ |
| §8.5 Guideline matching | `loon-engine/src/guideline_matching/*` | ✓ |
| §8.6 Tool calling | `loon-engine/src/tool_calling/*` | ⚠ DefaultToolCallBatcher is stub (returns empty) |
| §8.7 RelationalResolver | `loon-engine/src/relational_resolver.rs` | ✓ (real graph traversal) |
| §8.8 PromptBuilder | `loon-engine/src/prompt_builder.rs` | ✓ (10-section template) |
| §8.9 CannedResponseGenerator | `loon-engine/src/canned_response_generator.rs` | ✓ (LLM-driven selection) |
| §8.10 MessageGenerator | `loon-engine/src/message_generator.rs` | ✓ (calls LLM) |
| §8.11 ToolEventGenerator + MessageEventComposer | `loon-engine/src/{tool_event_generator,message_event_composer}.rs` | ✓ |
| §8.12 Planner + OptimizationPolicy + PerceivedPerformancePolicy | `loon-engine/src/{planner,optimization_policy,perceived_performance_policy}.rs` | ✓ all wired |
| §8.13 AlphaEngine 4-stage pipeline | `loon-engine/src/alpha_engine.rs:46-235` | ✓ with 8 hooks |
| §9 Indexing (9 traits) | `loon-engine/src/indexing/*` | ✓ all 9 have real impls |
| §10 SDK public types | `loon-sdk/src/*` | ⚠ AnyOf/AllOf/ToolContext/Variable MISSING |
| §11 HTTP/WS service (42 endpoints) | `loon-server/src/routes/*` | ⚠ 15 modules, ~50 endpoints; PATCH/DELETE complete |
| §12 CLI | `loon/src/{main,repl}.rs` | ✓ |
| §13 ApiError + ApiResponse | `loon-server/src/api/common.rs` | ✓ |
| §13 (sic) HealthReporter | `loon-engine/src/health/health_reporter.rs` | ⚠ stub methods (test-only fake impl is the only one) |
| §15 E2E test | `tests/e2e_agent_loop.rs` | ⚠ only 2 tests; no PATCH/WS path tested |
| §16 CI | `.github/workflows/ci.yml` | ✓ |

## Critical Issues (Must Fix Before "Done")

### 1. **`with_document_db` is a no-op** — `crates/loon-sdk/src/server.rs:80-110`

```rust
pub fn with_document_db<DB: loon_persistence::DocumentDatabase + 'static>(
    mut self,
    _db: Arc<DB>,
) -> Self {
    self.document_db_label = Some(std::any::type_name::<DB>().to_string());
    self  // db is recorded by type name only; the Arc is dropped
}
```

The user's `Arc<JsonFileDocumentDatabase>` or `Arc<MongoDocumentDatabase>` is **discarded**. The actual data path uses `self.entity_queries.unwrap_or_else(EntityQueries::in_memory)`. So:

- `cargo run -p loon-server` with `loon.toml` configured for Mongo: **silently uses in-memory storage**
- Restarting the server: **loses all data**
- `loon-chat-ui` against a running server: **agents/sessions disappear**

This is a fundamental capability gap. The `DocumentDatabase` trait has `async fn get_or_create_collection<TDocument: Document>(...)` — generics on object-safety violators block `Arc<dyn>`. **Fix:** add a `DocumentDatabaseHandle` indirection (already exists in `loon-persistence::migration`) that boxes per-collection accessors, then build a database-backed `EntityQueries::from_document_database(handle)` factory. **Effort:** 2-3 days.

### 2. **`DefaultToolCallBatcher` returns empty** — `crates/loon-engine/src/tool_calling/batcher.rs:80-95`

```rust
async fn generate_insights(...) -> EngineResult<ToolInsights> {
    Ok(ToolInsights::default())  // always empty
}
async fn call_tools(...) -> EngineResult<Vec<ToolExecutionResult>> {
    Ok(vec![])  // never calls anything
}
```

The engine's preparation loop will **never trigger a tool call** in production. This means MCP, OpenAPI, and LocalToolService integrations are wired but unreachable from chat. **Fix:** at minimum, port `SingleToolBatch` from parlcant's `tool_caller.py` — given matched guidelines with associated tools, prompt the LLM for arguments and invoke them. **Effort:** 1 week.

### 3. **`HealthReporter` is unwired and stubbed** — `crates/loon-engine/src/health/health_reporter.rs:80-180`

```rust
pub async fn check(&self) -> EngineResult<HealthStatus> {
    unimplemented!()
}
```

Six stub methods. There is no `/health/engine`, `/health/nlp`, `/health/event-loop` endpoint anywhere. The server's `/health` route returns a hardcoded `{"status":"ok"}`. **Fix:** implement `check()` to ping NLP + count session-store reads, mount under `/health/detailed`. **Effort:** half a day.

### 4. **SDK is missing 4 types from spec §10** — `crates/loon-sdk/src/server.rs`

Spec §10.2 calls for: Agent, Guideline, GuidelineMatch, Observation, Journey, Tool, **ToolContext**, Capability, Retriever, Term, **Variable**, Customer, Session, Tag, **AnyOf**, **AllOf**, Relationship, CompositionMode, MessageOutputMode.

Missing: `AnyOf`, `AllOf`, `ToolContext`, `Variable` (the variable type, distinct from `ContextVariable`). The spec models these as ergonomic SDK wrappers; current SDK only re-exports the raw core types via `pub use loon_core::*`. **Effort:** half a day.

## Spec vs Impl Mismatches

### 1. `EntityQueries` missing `shot_store` field — `crates/loon-core/src/entity_cq.rs:18`

Spec §3.6 says EntityQueries is the read-side of all entity stores. There are 16 InMemory stores (including `InMemoryShotStore`), but `EntityQueries` only has 15 store fields. Shot data is unreachable via the standard query path. **Fix:** add `pub shot_store: Arc<dyn ShotStore>` + wire in `EntityQueries::in_memory()` + add `find_shots_for_agent` query method.

### 2. `EntityCommands` is held but unused — `crates/loon-engine/src/alpha_engine.rs:32`

```rust
pub commands: Arc<EntityCommands>,  // field declared
```

The struct exists, has 3 methods (`update_session`, `update_context_variable_value`, `upsert_session_labels`), but `AlphaEngine::process` **never calls `self.commands`**. Routes also bypass it (they call `queries.session_store.update` directly). This means context variable values set by tool calls **don't propagate**. **Fix:** route writes to context_variable values + session label updates through `EntityCommands`. **Effort:** half a day.

### 3. `MessageEventHandle::update` always panics in EventPublisher — `crates/loon-emission/src/publisher.rs:228+`

```rust
update: Arc::new(|_| Box::pin(async { unreachable!() })),
```

The streaming-delta API is documented but unusable in the persistence path. Spec §4.1 says "allows in-place update of message content" — currently the only update path is via EventBuffer (in-memory). **Effort:** medium; needs session_store.update_event wiring through the closure.

### 4. WS chat doesn't stream — `crates/loon-server/src/routes/chat.rs`

`process_user_message` returns `Vec<OutgoingFrame>` (all frames at once). Real streaming requires sending each `AgentMessage` delta as the engine emits message events, not after `process()` returns. **Fix:** wire `EventEmitter` to the WS sink. **Effort:** medium-high (also needs `MessageGenerator::generate_streaming` to work — currently it returns a single chunk).

## Stub/Noop Inventory

### Production stubs (real concerns)

| File:line | What | Impact |
|---|---|---|
| `loon-engine/tool_calling/batcher.rs:80-95` | `DefaultToolCallBatcher` returns empty | **No tool calls in production** |
| `loon-engine/health/health_reporter.rs:80-180` | 6 health methods `unimplemented!()` | `/health/detailed` would panic |
| `loon-engine/message_generator.rs` | `generate_streaming` returns single chunk | WS streaming impossible |
| `loon-emission/publisher.rs:228+` | `MessageEventHandle::update` panics | In-place message updates broken |
| `loon-nlp/test_utils.rs:75` | `config()` panics — but only in test FakeNlp | Acceptable (test-only) |

### Test stubs (acceptable)

| File | Stubs | Why OK |
|---|---|---|
| `loon-engine/alpha_engine.rs` tests | ~25 `unimplemented!()` in `Dummy*Store` fake impls | Each fake only needs the methods the test exercises |
| `loon-app-modules/src/sessions.rs:136` | Test session-store fake | Acceptable |
| `loon-engine/message_generator.rs:145-163` | DummyNlp test mock | Acceptable |

### Noop policies (intentional defaults)

- `NoopIndexer`, `NoopBehavioralChangeEvaluation`, etc. — alongside real `Keyword*` impls, used as "no indexing" fallback. ✓ Acceptable.
- `NoopPlanner::plan` returns `Plan::Done` — equivalent to no planning. ✓ Acceptable.

## Parlcant Alignment Gaps

### 1. AlphaEngine pipeline shape

Parlcant's `engines/alpha/engine.py::AlphaEngine.process` has these phases:
```
acknowledging → preamble (optional) → preparing → preparation loop → generating_messages → emitted
```

loon has all 8 hook points wired (✓), but **never emits a preamble** — parlcant uses preamble as the "buying time" output while tools run. The `PerceivedPerformancePolicy::should_emit_preamble` exists and is called, but only emits an empty `"thinking..."` status — not a real preamble message. Parlcant emits a real LLM-generated preamble message. **Fix:** when `should_emit_preamble` is true, call `MessageGenerator::generate_preamble_message` (not yet implemented) and emit it as a message event.

### 2. Guideline matching is single-strategy

`LlmGuidelineMatcher::match_guidelines` (`loon-engine/src/guideline_matching/llm_matcher.rs`) returns a single match per LLM call. Parlcant supports custom strategies per guideline (e.g. always-match, regex-match) via `GenericGuidelineMatchingStrategyResolver`. **Current state:** the resolver exists (`strategy_resolver.rs`) but always returns `None`, so all guidelines fall through to the LLM. **Fix:** make the resolver actually route by `guideline.metadata.matcher_kind` (e.g. for `MATCH_ALWAYS`, skip LLM and return confidence 1.0).

### 3. Token budget enforcement is just a warning

`PromptBuilder::build_prompt` (`loon-engine/src/prompt_builder.rs:111-115`) logs a warning if the prompt exceeds `max_tokens` but doesn't actually truncate. Parlcant truncates the conversation history first, then drops lower-priority sections. **Fix:** add a `truncate_to_budget` pass.

### 4. `define_schematic!` macro uses snake_case JSON keys but the LLM is told to emit camelCase

Anthropic's `tool_use` response uses the field names as declared. The `define_schematic!` macro in `loon-nlp/src/macros.rs` uses `stringify!($field)` which gives snake_case (`canned_response_index`), so this matches as long as we instruct the LLM via the schema. **Probably OK** but should verify by hand with a live Anthropic call.

### 5. SessionStore lacks `find_events_after_offset` for incremental fetch

Parlcant pages session events for long sessions. loon's `SessionStore::find_events` always returns the full list. For high-volume agents this means the `Interaction` ballons. **Fix:** add a `find_events_after_offset(session_id, offset, limit)` method.

## Code Quality

### Good

- Consistent error handling (`thiserror` everywhere, no `unwrap()` outside tests)
- Cargo workspace dependency hygiene (all versions pinned in root `Cargo.toml`)
- Clippy clean with `-D warnings`
- 305 tests across 21 suites — solid baseline
- Documentation comments on most public types
- TDD discipline visible in commit history

### Bad

#### 1. `Default::default()` defeats deterministic ID generation

`loon-core/src/id_generator.rs::IdGenerator::default()` calls `Self::new()` which seeds from `nanoid`. Anywhere code uses `IdGenerator::default()` instead of explicitly seeding, you get non-reproducible IDs. Search hits:
```
crates/loon-core/src/entity_cq.rs:202: id_gen: IdGenerator::default()
crates/loon-app-modules/src/guidelines.rs:30: id_generator: Arc::new(Mutex::new(IdGenerator::default()))
```
**Fix:** require explicit seed.

#### 2. Duplicated entity-id mapping in routes

Every route module has the same pattern:
```rust
.map_err(|e| match e {
    loon_core::CoreError::NotFound(uid) => ApiError::NotFound(format!("foo {}", uid.0), "FOO_NOT_FOUND".into()),
    other => ApiError::Internal(other.to_string()),
})
```
14 instances. **Fix:** extract a helper `into_api_error(entity_name: &str, code: &str)` on `CoreError`.

#### 3. `OutgoingFrame::to_wire_json` hand-builds JSON

```rust
format!(r#"{{"type":"agent_message","delta":{}}}"#, serde_json::Value::String(delta.clone()))
```
Defensive against injection but ugly. **Fix:** define a `#[derive(Serialize)] enum OutgoingFrame` with `#[serde(tag = "type", content = "delta")]`.

#### 4. `EntityCommands` has only 3 methods

Spec §3.6 says EntityCommands holds the "write side" of the CQ separation. Currently it has `update_session`, `update_context_variable_value`, `upsert_session_labels`. **Most writes go directly through `EntityQueries.<store>.create/update/delete`** — the CQ separation is largely cosmetic.

#### 5. ~25 `unimplemented!()` in alpha_engine test stubs

Every test that needs a stub store gets a per-test struct with 5+ `unimplemented!()` methods. **Fix:** extract a single `MinimalStores` mock module under `loon-engine/src/test_utils.rs`.

## Test Coverage Gaps

| Path | Covered? | Notes |
|---|---|---|
| `process_message` end-to-end with real LLM | ✗ | Only tested with FakeNlpService |
| WS chat with streaming | ✗ | Only frame serialization tested |
| Routes: update/delete after create | ✓ | Each route module has a lifecycle test |
| Routes: under auth | ⚠ | `auth_middleware` has 3 isolated unit tests but no integration with route handlers |
| EngineHooks: complete pipeline | ⚠ | Only `on_acknowledging` and `on_messages_emitted` bail tested |
| RelationalResolver: cycle in dependencies | ✓ | Has a transitive-deps test |
| AlphaEngine with real MCP server | ✗ | No integration test |
| Migration helper with real version mismatch | ⚠ | `JsonFileMigrator` has a directory test; the full DocumentStoreMigrationHelper.enter() doesn't have an e2e test |
| `loon` CLI commands | ✗ | 10 tests in `crates/loon/`, all of `Cli::try_parse_from` shape; no integration |
| `loon-chat-ui` | ✗ | No tests at all (TypeScript files exist, not exercised) |

## Recommendations (Ordered by Impact)

### 1. **Implement `EntityQueries::from_document_database` factory** — *Critical*

Without this, persistence is fake. Build it on top of the existing `JsonFileDocumentDatabase`. Pattern:

```rust
pub async fn from_document_database<D: DocumentDatabase>(db: Arc<D>) -> Result<Arc<Self>> {
    let agents = db.get_or_create_collection::<AgentDoc>("agents", json!({}), agent_loader()).await?;
    // ... 15 more collections
    let agent_store: Arc<dyn AgentStore> = Arc::new(DocumentBackedAgentStore { collection: agents });
    // ... etc
    Ok(Arc::new(EntityQueries { agent_store, ... }))
}
```

Each `DocumentBacked*Store` is a thin wrapper that serializes the entity to a document, calls the collection, and deserializes. **Effort:** 2-3 days. **Impact:** unblocks data persistence end-to-end.

### 2. **Implement `SingleToolBatch` in `DefaultToolCallBatcher`** — *Critical*

The engine architecture is sound but the tool-call path is empty. Port parlcant's `tool_caller.py::SingleToolBatch`. **Effort:** 1 week.

### 3. **Add `shot_store` to `EntityQueries`** — *Easy*

5-minute fix: add the field, wire in `in_memory()`, add a `find_shots_for_agent` query method. **Effort:** half a day.

### 4. **Fix `MessageEventHandle::update` in `EventPublisher`** — *Medium*

Capture the `Arc<dyn SessionStore>` + `event_id` in the closure and call `session_store.update_event`. **Effort:** half a day.

### 5. **Stream WS chat from EventEmitter** — *Medium-High*

Replace `Vec<OutgoingFrame>` with a `tokio::sync::mpsc::channel<OutgoingFrame>` and wire `EventEmitter::emit_message_event` to push into it. **Effort:** 2-3 days.

### 6. **Add `AnyOf`, `AllOf`, `ToolContext`, `Variable` to SDK** — *Easy*

Spec calls for these but they're missing. **Effort:** half a day.

### 7. **Implement preamble path in MessageGenerator** — *Medium*

Currently `should_emit_preamble` triggers a stub status. Generate a real preamble message via LLM. **Effort:** 1-2 days.

### 8. **Add integration test through axum router with auth + rate limit** — *Easy*

`tower::ServiceExt::oneshot` + `Router::layer(middleware)` — verify a request flows through both layers. **Effort:** half a day.

### 9. **Implement HealthReporter** — *Easy*

Real `check()` that pings NLP + counts a session_store list. Mount at `/health/detailed`. **Effort:** half a day.

### 10. **Token budget truncation in PromptBuilder** — *Medium*

Truncate conversation history first when over `max_tokens`. **Effort:** 1 day.

## Summary

Loon's **structure is correct and comprehensive**. All 11 phases of the parlcant Rust reimplementation are in place at the API + module level. The gaps are in **depth**, not breadth — specifically the persistence integration, tool-calling path, and WS streaming. Fixing items 1, 2, and 5 above would move this from "demonstrably works for trivial cases" to "production-ready for real users."

The codebase is clean, idiomatic Rust, well-tested at the unit level, and CI-clean. It's a strong foundation that a future contributor could land item-by-item against.

# loon Phase 1 设计文档

**日期：** 2026-06-15
**作者：** doug（与 Claude 协作完成）
**状态：** 待用户审阅

---

## 0. 背景与目标

`loon` 是 `parlant`（<https://github.com/emcie-co/parlant>）的 Rust 复刻项目，目标是 1:1 对齐其域模型与语义，输出一个生产可用的 Rust crate 生态。

参考材料：`docs/reference/parlant-overview.md`（包含对 parlcant 仓库的逐项调研笔记）。

本仓库采用 12 phase 分解，**本 spec 仅覆盖 Phase 1+2 合并后的范围**（核心域 + 文档数据库抽象 + OpenAI provider + AlphaEngine 最小闭环 + axum HTTP/WS API）。后续 phase 在各自 spec 中定义。

---

## 1. Phase 1 范围与非目标

### 1.1 In Scope

- 完整核心域模型（15 个实体，与 parlcant 1:1）
- 文档数据库抽象（`DocumentDatabase` trait）+ JSON 文件后端
- NLP 抽象（`StreamingTextGenerator`、`SchematicGenerator[T]`）+ OpenAI 一个 provider
- AlphaEngine 4 阶段流水线（策略模式）
- SDK（Rust crate `loon-sdk`）
- axum HTTP/WS 服务（`loon-server` 二进制）
- 客户端 CLI（`loon` 二进制）
- 集成测试：`tests/e2e_agent_loop.rs`
- 单元测试覆盖每个 crate
- 文档：`docs/reference/parlant-overview.md`（已写）+ 本 spec

### 1.2 Out of Scope（后续 phase 处理）

- 其它 LLM provider（Anthropic / Gemini / Vertex / Ollama / LiteLLM / Bedrock / Together / Cerebras / DeepSeek）
- MongoDB / Chroma / Qdrant 后端
- 向量数据库（仅留 trait 接口，无实现）
- MCP、OpenAPI、Plugin 服务
- 文档迁移系统（`bin/prepare_migration.py` 等价物）
- TS 前端
- OpenTelemetry / 指标 / 分布式追踪
- 限速、配额、token 计费
- OAuth / API Key 授权（仅留 placeholder trait）

---

## 2. 全局项目结构

### 2.1 Workspace

仓库根：`/Users/doug/ai/system/loon/`，使用 Cargo workspace。

```
loon/
├── Cargo.toml                       # workspace 声明
├── docs/
│   ├── reference/
│   │   └── parlant-overview.md      # ✅ parlcant 调研笔记
│   └── superpowers/
│       └── specs/
│           └── 2026-06-15-loon-phase1-design.md  # 本文档
├── crates/
│   ├── loon-core/                   # 域实体 + 抽象
│   ├── loon-persistence/            # 文档 / 向量数据库抽象 + JSON 后端
│   ├── loon-nlp/                    # NLP 抽象 + OpenAI 实现
│   ├── loon-engine/                 # AlphaEngine + 4 个 strategy
│   ├── loon-sdk/                    # 公开 SDK
│   ├── loon-server/                 # axum HTTP/WS 服务
│   └── loon/                        # CLI 客户端二进制
└── tests/
    └── e2e_agent_loop.rs            # workspace 级集成测试
```

### 2.2 依赖方向（强制，CI 检查）

```text
loon-server ──▶ loon-sdk ──▶ loon-engine ──┬──▶ loon-core
                                            ├──▶ loon-nlp
                                            └──▶ loon-persistence
loon (CLI) ──▶ loon-sdk
```

- `loon-core` 不依赖任何其它 loon crate。
- `loon-nlp` / `loon-persistence` 仅依赖 `loon-core`。
- `loon-engine` 同时依赖 `loon-core` / `loon-nlp` / `loon-persistence`。
- `loon-sdk` 依赖 `loon-engine` / `loon-nlp` / `loon-persistence`。
- `loon-server` / `loon`（CLI）仅依赖 `loon-sdk`。

依赖方向违反由 `cargo-deny` 或自定义 build script 在 CI 中检查。

### 2.3 crate 命名空间

所有公开类型在 crate 根模块导出，例如：

```rust
use loon_core::{Agent, AgentId, Guideline, Session, SessionId};
use loon_nlp::{SchematicGenerator, StreamingTextGenerator, OpenAIProvider, NlpService};
use loon_persistence::{DocumentDatabase, JsonFileDocumentDatabase};
use loon_engine::{AlphaEngine, Engine, GuidelineMatcher, ToolCaller, Planner, MessageComposer};
use loon_sdk::{Server, ServerBuilder};
```

---

## 3. 核心域（`loon-core`）

### 3.1 ID 类型

每个实体一个 newtype ID，避免跨实体混用：

```rust
// crates/loon-core/src/ids.rs
define_id!(AgentId);
define_id!(GuidelineId);
define_id!(JourneyId);
define_id!(JourneyStateId);
define_id!(ToolId);
define_id!(SessionId);
define_id!(CustomerId);
define_id!(TagId);
define_id!(RelationshipId);
define_id!(CannedResponseId);
define_id!(CapabilityId);
define_id!(ContextVariableId);
define_id!(RetrieverId);
define_id!(GlossaryTermId);
define_id!(EventId);
define_id!(MessageId);
```

`define_id!` 宏产出：

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(pub String);

impl AgentId {
    pub fn new() -> Self { Self(nanoid::nanoid!()) }
    pub fn as_str(&self) -> &str { &self.0 }
}
```

### 3.2 实体清单

15 个实体 + 2 个枚举 + 1 个辅助：

| 实体 | 文件 | 关键字段 |
|------|------|----------|
| `Agent` | `agent.rs` | `id`, `name`, `description`, `composition_mode`（`Fluid`/`Strict`）, `message_output_mode`, `tags` |
| `Guideline` | `guideline.rs` | `id`, `agent_id`, `condition: GuidelineContent`, `action: GuidelineContent`, `enabled`, `tags`, `metadata` |
| `GuidelineContent` | `guideline.rs` | `condition: String`, `action: String` |
| `Journey` | `journey.rs` | `id`, `agent_id`, `title`, `description`, `states: Vec<JourneyState>`, `transitions: Vec<JourneyTransition>`, `tags` |
| `JourneyState` | `journey.rs` | enum: `Initial`, `Tool`, `Chat`, `Fork` |
| `JourneyTransition` | `journey.rs` | `from`, `to`, `condition`, `action` |
| `Observation` | `observation.rs` | `id`, `agent_id`, `condition`, `tools: Vec<ToolId>`, `enabled` |
| `Tool` | `tool.rs` | `id`, `name`, `description`, `parameters_schema: serde_json::Value`, `kind: ToolKind`（`Local`/`OpenAPI`/`MCP`） |
| `ToolCall` / `ToolResult` | `tool.rs` | 调用 / 结果事件 |
| `Session` | `session.rs` | `id`, `agent_id`, `customer_id`, `title`, `mode`, `events: Vec<Event>` |
| `SessionMode` | `session.rs` | enum: `Auto`, `Manual` |
| `Event` | `session.rs` | enum: `Status`, `Message`, `ToolCall`, `ToolResult` |
| `Message` | `session.rs` | `kind: User`/`Agent`/`System`, `content`, `timestamp` |
| `Customer` | `customer.rs` | `id`, `name`, `metadata: serde_json::Value`, `tags` |
| `Glossary` / `Term` | `glossary.rs` | 术语列表 |
| `Variable` / `ContextVariable` | `variable.rs` | 键值变量；`ContextVariable` 带 `freshness_rules` |
| `CannedResponse` | `canned_response.rs` | `id`, `agent_id`, `value: String`, `tags`, `matchers` |
| `Capability` / `Retriever` | `capability.rs` | 知识检索能力 |
| `Tag` | `tag.rs` | `id`, `name` |
| `Relationship` | `relationship.rs` | `source_kind`, `source_id`, `target_kind`, `target_id`, `kind`（如 `entails`/`excludes`） |
| `CompositionMode` | `agent.rs` | `Fluid`, `Strict` |
| `MessageOutputMode` | `agent.rs` | `Fluid`, `Canned` |
| `GuidelineMatchingContext` | `guideline.rs` | `guidelines: Vec<Guideline>`, `messages: Vec<Message>`（供 matcher 使用） |

### 3.3 Store trait

每个实体对应一个 Store trait，定义 CRUD 接口：

```rust
// crates/loon-core/src/agent.rs
#[async_trait]
pub trait AgentStore: Send + Sync {
    async fn create(&self, agent: Agent) -> Result<Agent>;
    async fn read(&self, id: AgentId) -> Result<Option<Agent>>;
    async fn update(&self, id: AgentId, params: AgentUpdateParams) -> Result<Agent>;
    async fn delete(&self, id: AgentId) -> Result<()>;
    async fn list(&self, tags: &[TagId]) -> Result<Vec<Agent>>;
}

#[derive(Default)]
pub struct AgentUpdateParams {
    pub name: Option<String>,
    pub description: Option<String>,
    pub composition_mode: Option<CompositionMode>,
    pub message_output_mode: Option<MessageOutputMode>,
    pub tags: Option<Vec<TagId>>,
}
```

15 个实体 × 1 个 Store trait = 15 个 trait，集中放在 `loon-core/src/stores/` 子模块。

### 3.4 服务层 trait

```rust
// crates/loon-core/src/services.rs
#[async_trait]
pub trait AgentService: Send + Sync {
    async fn create_agent(&self, params: AgentCreateParams) -> Result<Agent>;
    async fn update_agent(&self, id: AgentId, params: AgentUpdateParams) -> Result<Agent>;
    async fn delete_agent(&self, id: AgentId) -> Result<()>;
    // ... 全部业务方法
}
```

服务层封装 store，对外暴露业务语义。每个实体一个 Service。

---

## 4. 持久化（`loon-persistence`）

### 4.1 文档数据库抽象

```rust
// crates/loon-persistence/src/document.rs
#[async_trait]
pub trait DocumentDatabase: Send + Sync {
    async fn collection<TDocument: Document>(&self, name: &str) -> Result<Box<dyn DocumentCollection<TDocument>>>;
}

#[async_trait]
pub trait DocumentCollection<TDocument: Document>: Send + Sync {
    async fn insert(&self, doc: TDocument) -> Result<InsertResult>;
    async fn find_one(&self, filter: &DocumentFilter) -> Result<Option<TDocument>>;
    async fn find(&self, filter: &DocumentFilter) -> Result<Vec<TDocument>>;
    async fn update_one(&self, filter: &DocumentFilter, update: DocumentUpdate) -> Result<UpdateResult>;
    async fn delete_one(&self, filter: &DocumentFilter) -> Result<DeleteResult>;
    async fn count(&self, filter: &DocumentFilter) -> Result<u64>;
}

pub trait Document: Serialize + DeserializeOwned + Send + Sync + 'static {
    const VERSION: u32;             // 用于迁移
    type Id: Serialize + DeserializeOwned + Send + Sync;
    fn id(&self) -> &Self::Id;
}
```

### 4.2 JSON 文件后端（`backends/json_file.rs`）

- 每个 collection 一个目录，目录下文件命名 `<doc_id>.json`。
- 写入用临时文件 + 原子 rename，保证崩溃一致。
- 启动时全量加载到 `Arc<RwLock<HashMap<...>>>`，所有操作走内存，最后由后台任务定期 flush（默认 5 秒）。
- filter 表达式 JSON 子集：`{"field": value}` / `{"field": {"$in": [...]}}` / `{"$and": [...]}`。
- 后续 phase 用 `fjall`/`sled` 或真 Mongo 替换，保持 trait 不变。

### 4.3 向量数据库抽象（占位）

```rust
// crates/loon-persistence/src/vector.rs
#[async_trait]
pub trait VectorDatabase: Send + Sync {
    async fn upsert(&self, collection: &str, id: &str, vector: Vec<f32>, metadata: serde_json::Value) -> Result<()>;
    async fn search(&self, collection: &str, query: Vec<f32>, top_k: usize) -> Result<Vec<VectorHit>>;
}
```

Phase 1 不实现任何向量后端，仅留 trait。`OpenAIProvider::embed` 可写入日志但不调用任何 VectorDatabase。

### 4.4 配置

`DocumentDatabase` 在启动时通过路径加载：

```rust
pub struct JsonFileDocumentDatabase {
    root_path: PathBuf,
    flush_interval: Duration,        // 默认 5s
}
```

---

## 5. NLP（`loon-nlp`）

### 5.1 核心抽象

```rust
// crates/loon-nlp/src/service.rs
pub struct NlpConfig {
    pub provider: NlpProviderKind,         // 仅 OpenAI
    pub model: String,                     // 默认 "gpt-4o-mini"
    pub endpoint: Option<String>,          // 自定义代理
    pub api_key: SecretString,             // 从 env OPENAI_API_KEY
    pub max_retries: u32,                  // 默认 3
    pub timeout: Duration,                 // 默认 60s
    pub temperature: f32,                  // 默认 0.2
}

#[async_trait]
pub trait NlpService: Send + Sync {
    fn config(&self) -> &NlpConfig;
    async fn text_generator(&self) -> Result<Box<dyn StreamingTextGenerator>>;
    async fn schematic_generator<T: Schematic + Send + Sync + 'static>(
        &self,
    ) -> Result<Box<dyn SchematicGenerator<T>>>;
    async fn embedder(&self) -> Result<Box<dyn Embedder>>;
    async fn tokenizer(&self) -> Result<Box<dyn Tokenizer>>;
    async fn moderater(&self) -> Result<Box<dyn Moderater>>;
}
```

### 5.2 Schematic 与生成器

```rust
// crates/loon-nlp/src/generator.rs
#[async_trait]
pub trait StreamingTextGenerator: Send + Sync {
    async fn generate(
        &self,
        prompt: String,
        options: TextGenerationOptions,
    ) -> Result<StreamingTextGenerationResult>;
}

#[async_trait]
pub trait SchematicGenerator<T: Schematic>: Send + Sync {
    async fn generate(
        &self,
        prompt: String,
        options: SchematicGenerationOptions,
    ) -> Result<SchematicGenerationResult<T>>;

    async fn generate_streaming(
        &self,
        prompt: String,
        options: SchematicGenerationOptions,
    ) -> Result<StreamingSchematicGenerationResult<T>>;
}

pub trait Schematic: Serialize + DeserializeOwned + Sized + Send + Sync + 'static {
    fn schema() -> serde_json::Value;       // JSON Schema
}
```

工具宏：

```rust
loon_nlp::schematic! {
    pub struct GuidelineMatch {
        pub guideline_id: String,
        pub confidence: f32,
        pub rationale: String,
    }
}
```

宏展开为：

```rust
impl loon_nlp::Schematic for GuidelineMatch {
    fn schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "guideline_id": { "type": "string" },
                "confidence": { "type": "number" },
                "rationale": { "type": "string" },
            },
            "required": ["guideline_id", "confidence", "rationale"],
        })
    }
}
```

### 5.3 OpenAI provider

```rust
// crates/loon-nlp/src/providers/openai.rs
pub struct OpenAiProvider { config: Arc<NlpConfig>, http: reqwest::Client }

#[async_trait]
impl NlpService for OpenAiProvider { /* ... */ }

struct OpenAiSchematicGenerator<T> { ... }

#[async_trait]
impl<T: Schematic> SchematicGenerator<T> for OpenAiSchematicGenerator<T> {
    async fn generate(&self, prompt: String, opts: SchematicGenerationOptions)
        -> Result<SchematicGenerationResult<T>>
    {
        // 1. POST /v1/chat/completions with response_format=json_schema
        // 2. 解析返回（带 retry，max_retries 来自 config）
        // 3. 用 jsonschema crate 校验 T::schema()
        // 4. 返回 SchematicGenerationResult { value: T, info: GenerationInfo }
    }
}
```

- 流式：SSE 解析用 `futures::StreamExt` + `reqwest` `bytes_stream`。
- 重试：指数退避，仅对 429 / 5xx 重试。
- 错误：`NlpError::RateLimited` / `NlpError::InvalidSchema` / `NlpError::Upstream` / `NlpError::Timeout`。

### 5.4 Fallback

```rust
pub struct FallbackSchematicGenerator<T> {
    primary: Box<dyn SchematicGenerator<T>>,
    fallbacks: Vec<Box<dyn SchematicGenerator<T>>>,
}
```

Phase 1 不强制使用，留作 Phase 3 多 provider 时启用。

---

## 6. 引擎（`loon-engine`）

### 6.1 Engine trait

```rust
// crates/loon-engine/src/engine.rs
#[async_trait]
pub trait Engine: Send + Sync {
    async fn process_turn(
        &self,
        session_id: SessionId,
        user_message: Message,
    ) -> Result<Vec<Event>>;
}

pub struct AlphaEngine {
    matcher: Arc<dyn GuidelineMatcher>,
    tool_caller: Arc<dyn ToolCaller>,
    planner: Arc<dyn Planner>,
    composer: Arc<dyn MessageComposer>,
    agent_service: Arc<dyn AgentService>,
    session_service: Arc<dyn SessionService>,
    guideline_service: Arc<dyn GuidelineService>,
    journey_service: Arc<dyn JourneyService>,
    tool_service: Arc<dyn ToolService>,
    nlp: Arc<dyn NlpService>,
}
```

### 6.2 4 个策略 trait

```rust
// crates/loon-engine/src/strategies/guideline_matcher.rs
#[async_trait]
pub trait GuidelineMatcher: Send + Sync {
    async fn match_guidelines(
        &self,
        ctx: GuidelineMatchingContext,
    ) -> Result<Vec<GuidelineMatch>>;
}

// crates/loon-engine/src/strategies/tool_caller.rs
#[async_trait]
pub trait ToolCaller: Send + Sync {
    async fn call(
        &self,
        ctx: ToolCallingContext,
    ) -> Result<Vec<ToolExecutionResult>>;
}

// crates/loon-engine/src/strategies/planner.rs
#[async_trait]
pub trait Planner: Send + Sync {
    async fn plan(
        &self,
        ctx: PlanningContext,
    ) -> Result<Plan>;
}

// crates/loon-engine/src/strategies/message_composer.rs
#[async_trait]
pub trait MessageComposer: Send + Sync {
    async fn compose(
        &self,
        ctx: MessageCompositionContext,
    ) -> Result<MessageCompositionResult>;
}
```

### 6.3 默认实现

| 策略 | Phase 1 默认实现 |
|------|------------------|
| `GuidelineMatcher` | `LlmGuidelineMatcher`：用 `SchematicGenerator<Vec<GuidelineMatch>>` 调一次 LLM，prompt 包含所有候选 guideline + 当前消息 + 历史 |
| `ToolCaller` | `DefaultToolCaller`：并发执行匹配到的 tool，按 `tools/single_tool_batch.rs` 与 `tools/overlapping_tools_batch.rs` 思路分批 |
| `Planner` | `NoopPlanner`：Phase 1 无规划，直接进入消息合成 |
| `MessageComposer` | `LlmMessageComposer`：用 `StreamingTextGenerator` 或 `SchematicGenerator<MessageEvent>` 调 LLM，prompt 来自 `prompt_builder.rs` |

### 6.4 EngineContext

```rust
pub struct EngineContext {
    pub session: Session,
    pub agent: Agent,
    pub matched_guidelines: Vec<Guideline>,
    pub resolved_journey_state: Option<JourneyState>,
    pub tool_results: Vec<ToolExecutionResult>,
    pub intermediate_events: Vec<Event>,
}

pub struct TurnInput {
    pub session_id: SessionId,
    pub user_message: Message,
}
```

### 6.5 流水线

`AlphaEngine::process_turn`：

```text
1. 加载 Session + Agent
2. GuidelineMatcher.match_guidelines(ctx)        → matched_guidelines
3. JourneyService.resolve_state(session, ctx)    → journey_state（若有 journey）
4. ToolCaller.call(ctx with matched + tools)     → tool_results
5. Planner.plan(ctx)                              → Plan（Phase 1: 直接转下一步）
6. MessageComposer.compose(ctx with all of above) → message_event(s)
7. SessionService.append_events(...)              → 持久化
8. 返回 events
```

每步都把中间状态写入 `EngineContext`，便于单步测试。

---

## 7. SDK（`loon-sdk`）

### 7.1 形态

完全对齐 parlcant 的声明式 API：

```rust
use loon_sdk as p;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    p::Server::builder()
        .with_document_db(JsonFileDocumentDatabase::new("./data")?)
        .with_nlp_service(OpenAiProvider::new(NlpConfig::from_env()?))
        .build()
        .await?
        .run(|server| async move {
            let agent = server.create_agent(p::AgentCreateParams {
                name: "Customer Support".into(),
                description: "Handles customer inquiries for an airline".into(),
                ..Default::default()
            }).await?;

            let expert = agent.create_observation(p::ObservationCreateParams {
                condition: "customer uses financial terminology".into(),
                tools: vec!["research_deep_answer".into()],
                ..Default::default()
            }).await?;

            agent.create_guideline(p::GuidelineCreateParams {
                condition: "always".into(),
                action: "respond with technical depth".into(),
                dependencies: vec![expert.id().clone()],
                ..Default::default()
            }).await?;

            Ok(())
        }).await?;
    Ok(())
}
```

### 7.2 公开类型

- `Server` / `ServerBuilder`
- `Agent` / `Guideline` / `Journey` / `JourneyState` / `JourneyTransition` / `Observation`
- `Tool` / `ToolCall`
- `Customer` / `Session` / `SessionMode`
- `Capability` / `Retriever` / `Term` / `Variable`
- `Tag` / `AnyOf` / `AllOf` / `Relationship`
- `CompositionMode` / `MessageOutputMode`

所有类型 re-export 自 `loon-core`，SDK 仅做 ergonomic 包装（builder、async 链式调用、生命周期管理）。

### 7.3 错误

```rust
#[derive(Debug, thiserror::Error)]
pub enum SdkError {
    #[error("agent not found: {0}")] AgentNotFound(AgentId),
    #[error("guideline not found: {0}")] GuidelineNotFound(GuidelineId),
    #[error("NLP config error: {0}")] NlpConfig(#[from] NlpConfigError),
    #[error("persistence error: {0}")] Persistence(#[from] PersistenceError),
    #[error("engine error: {0}")] Engine(#[from] EngineError),
    #[error(transparent)] Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}
```

---

## 8. HTTP/WS 服务（`loon-server`）

### 8.1 端点

| Method | Path | 说明 |
|--------|------|------|
| GET | `/health` | 健康检查 |
| GET | `/version` | 服务版本 |
| GET | `/v1/agents` | 列出 agents |
| POST | `/v1/agents` | 创建 agent |
| GET | `/v1/agents/{id}` | 读 agent |
| PATCH | `/v1/agents/{id}` | 更新 agent |
| DELETE | `/v1/agents/{id}` | 删除 agent |
| GET/POST/PATCH/DELETE | `/v1/agents/{id}/guidelines` | guideline CRUD |
| GET/POST/PATCH/DELETE | `/v1/agents/{id}/journeys` | journey CRUD |
| GET/POST/PATCH/DELETE | `/v1/agents/{id}/tools` | tool CRUD |
| GET/POST/PATCH/DELETE | `/v1/agents/{id}/observations` | observation CRUD |
| GET | `/v1/sessions` | 列出会话 |
| POST | `/v1/sessions` | 创建会话 |
| POST | `/v1/sessions/{id}/events` | 推送消息事件 |
| GET | `/v1/sessions/{id}/events` | 流式拉取事件 |
| WS | `/v1/sessions/{id}/chat` | WS 双向聊天 |
| GET | `/v1/customers` | customer CRUD |
| GET | `/v1/tags` | 标签 |
| GET | `/v1/glossary` | 术语 |

所有 endpoint 与 parlcant 的 FastAPI 路径对齐（便于后续直接对接前端）。

### 8.2 状态机

```rust
pub struct AppState {
    pub server: Arc<loon_sdk::Server>,
    pub auth: Arc<dyn AuthProvider>,    // Phase 1 仅占位 trait，noop 实现
    pub meter: Arc<Meter>,             // 简单内存计数
}
```

### 8.3 WS 聊天

```rust
async fn chat_ws(ws: WebSocketUpgrade, State(state): State<AppState>, Path(session_id): Path<SessionId>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| async move {
        let (mut sender, mut receiver) = socket.split();
        // 1. 收到 {"type": "user_message", "content": "..."}
        // 2. 调 engine.process_turn(session_id, msg)
        // 3. 流式回写 {"type": "agent_message", "delta": "..."} / {"type": "tool_call", ...}
    })
}
```

事件类型与 parlcant WS 一致。

### 8.4 配置

通过环境变量 + `config.toml`：

```toml
[server]
bind = "0.0.0.0:8800"

[persistence]
kind = "json_file"
root = "./data"
flush_interval_ms = 5000

[nlp]
provider = "openai"
model = "gpt-4o-mini"
api_key_env = "OPENAI_API_KEY"
max_retries = 3
timeout_ms = 60000

[auth]
kind = "noop"
```

---

## 9. CLI 客户端（`loon` 二进制）

clap 子命令，与 parlcant CLI 对齐：

```text
loon server start    [--port 8800] [--config ./loon.toml]
loon server stop
loon agent list
loon agent create --name "..." --description "..."
loon agent update <id> [--name "..."] [--description "..."]
loon agent delete <id>
loon guideline create --agent <id> --condition "..." --action "..."
loon session create --agent <id>
loon session chat <id>                  # 启动 REPL，调 /v1/sessions/{id}/chat WS
loon journey create --agent <id> --title "..."
loon tool create --agent <id> --name "..." --script ./tool.py
```

交互层用 `dialoguer` + `console`。

---

## 10. 错误处理

- 每个 crate 定义自己的 `Error` enum + `Result<T>` 别名。
- 所有 `Error` 实现 `std::error::Error`（通过 `thiserror`）。
- 服务层把内部错误用 `From` 转为 `SdkError` / `EngineError` / `NlpError` / `PersistenceError`。
- HTTP 层把领域错误映射到状态码：
  - `NotFound` → 404
  - `InvalidArgument` → 400
  - `Conflict` → 409
  - `RateLimited` → 429
  - `Upstream` → 502
  - `Internal` → 500

---

## 11. 测试策略

### 11.1 单元测试

每个 crate 内部 `#[cfg(test)]` 模块：
- `loon-core`：实体构造、ID 唯一性、JSON 序列化往返。
- `loon-persistence`：JSON 后端 CRUD、filter 表达式、并发读写。
- `loon-nlp`：用 `mockito` 或 `wiremock` mock OpenAI，验证 prompt 拼接、retry、SSE 解析、schema 校验。
- `loon-engine`：每个 strategy trait 写 fake 实现，验证 `AlphaEngine` 流水线编排。

### 11.2 集成测试

`tests/e2e_agent_loop.rs`：
- 启动 in-memory `JsonFileDocumentDatabase`（tmp 目录）。
- 启动 `AlphaEngine`。
- 创建一个 agent + 2 个 guideline + 1 个 tool（用 stub 实现）。
- 发起一个会话，跑 2 轮，断言：
  - guideline 正确匹配
  - tool 正确调用
  - 响应消息非空
  - 事件正确持久化

### 11.3 文档测试

每个公共 trait / 函数带 doc example，cargo test --doc 校验。

### 11.4 覆盖率门槛

CI 用 `cargo-llvm-cov` 检查，新代码 line coverage ≥ 80%。

---

## 12. CI 与质量门

- GitHub Actions：
  - `cargo fmt --check`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo test --workspace`
  - `cargo doc --no-deps --all-features`
  - `cargo deny check`
- MSRV：1.78（最新 stable）。
- 平台：`x86_64-unknown-linux-gnu` + `aarch64-apple-darwin`（用户当前平台）。

---

## 13. 文档结构

```text
docs/
├── reference/
│   └── parlant-overview.md         ✅ 已写
└── superpowers/
    └── specs/
        └── 2026-06-15-loon-phase1-design.md  ← 本文档
```

后续每个 phase 一个 spec：`2026-06-XX-loon-phase3-design.md` 等。

README.md 在 Phase 1 末尾写，包含：项目简介、快速开始、API 链接、贡献指南。

---

## 14. 风险与缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| OpenAI 限速导致集成测试不稳定 | 高 | 测试默认不调真实 OpenAI；用 mock；CI 跑 mock 集成测试，真实 OpenAI 测试在 nightly job 跑 |
| AlphaEngine 内部逻辑复杂 | 中 | 每个 strategy 单独测试；EngineContext 透明化 |
| 文档数据库 JSON 性能 | 低（Phase 1） | 内存 cache + 周期 flush；后续 phase 用真后端 |
| SDK 与 core 边界模糊 | 中 | SDK 仅做 ergonomic 包装；core 提供基础类型；`cargo doc` 双重标注 |
| 与 parlcant 语义漂移 | 高 | 每个 phase spec 都附 parlcant 对应文件路径引用；CI 加 fixture 一致性测试 |

---

## 15. 待用户确认的关键点（写 spec 前已确认）

- [x] Phase 1+2 合并（含 axum HTTP/WS）
- [x] 持久化：文档抽象 + JSON 文件后端
- [x] NLP：自研 trait + reqwest，仅 OpenAI
- [x] 工程结构：Cargo workspace，提前划分
- [x] DI：手工（无框架）
- [x] 引擎编排：策略模式
- [x] 交付：SDK + server + CLI + 集成测试

---

## 16. 后续 phase 简表（仅占位，详细 spec 后续写）

- **Phase 3**：Anthropic + Gemini provider
- **Phase 4**：MongoDB 后端
- **Phase 5**：Chroma / Qdrant 向量后端
- **Phase 6**：MCP 服务
- **Phase 7**：OpenAPI 服务
- **Phase 8**：Plugin 系统
- **Phase 9**：文档迁移系统
- **Phase 10**：OpenTelemetry / 限速 / 鉴权
- **Phase 11**：TS 前端（独立仓库 `loon-chat-ui`）

---

## 17. 参考

- <https://github.com/emcie-co/parlant>
- `docs/reference/parlant-overview.md`

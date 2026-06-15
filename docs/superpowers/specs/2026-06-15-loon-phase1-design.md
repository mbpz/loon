# loon Phase 1 设计文档（修订版 v2）

**日期：** 2026-06-15（初版）/ 修订 2026-06-16（v2）/ 修订 2026-06-16（v3）
**作者：** doug（与 Claude 协作完成）
**状态：** 已通过全量代码审查修订（v3：补齐 5 个审查缺口）

---

## 0. 背景与目标

`loon` 是 `parlant`（<https://github.com/emcie-co/parlant>）的 Rust 复刻项目，目标是 **1:1 对齐其域模型与语义**，输出一个生产可用的 Rust crate 生态。

参考材料：`docs/reference/parlant-overview.md`（包含对 parlcant 仓库的逐项调研笔记）。

本仓库采用 **12 phase** 分解，**本 spec 覆盖 Phase 1+2 范围**。修订版 v2 基于 parlcant `develop` 分支全量源码审查，补齐了初版遗漏的 7 个核心模块（EntityCQ / AppModules / EventEmission / JourneyGuidelineProjection / GuidelineToolAssociations / Indexing / Health），并大幅扩展了引擎组件的接口设计。

> 本文档中每个模块均标注对应 Parlant 源文件路径，引用格式为 `src/parlant/core/<path>:<key concept>`。

---

## 1. Phase 1 范围与非目标

### 1.1 In Scope（修订后）

#### 核心域（loon-core）
- 完整 15 个实体 + 2 个枚举 + `EntityQueries` / `EntityCommands`（CQ 分离）
- 所有实体的 Store trait（15 个）
- `IdGenerator` / `Version` / `Criticality` / `UniqueId` 等公共工具
- `Relationship`（含 `entails`/`excludes`/`dependency`/`reevaluation` 四种关系类型）
- `GuidelineToolAssociation` + `JourneyGuidelineProjection`
- `Shot`（Few-Shot 示例）+ `ShotStore`
- `ServiceRegistry` + `ToolService` trait（工具服务注册与调用）
- `Logger` trait + `Tracer` trait（Phase 1 简单实现）

#### 事件发射（loon-emission）
- `EventEmitter` trait + `EventEmitterFactory` trait
- `EventBuffer`（内存实现，供测试/引擎内部使用）
- `EventPublisher`（持久化实现，直接写 SessionStore）
- `MessageEventHandle`（允许流式追加更新的消息句柄）
- `EmittedEvent` 类型（未持久化的事件表示）

#### AppModules 层（loon-app-modules）
- 13 个模块：`agents` / `guidelines` / `journeys` / `sessions` / `customers` / `glossary` / `tags` / `canned_responses` / `capabilities` / `context_variables` / `evaluations` / `relationships` / `services`
- 每个模块封装对应 Store 的业务语义，提供 create/read/update/delete/list 方法

#### 持久化（loon-persistence）
- `DocumentDatabase` trait + `DocumentCollection<TDocument>` trait
- `VectorDatabase` trait（仅声明，Phase 1 不实现后端）
- JSON 文件后端 `JsonFileDocumentDatabase`
- `DocumentStoreMigrationHelper`（迁移占位，支持 `Version` 标注但不执行迁移逻辑）
- `DataCollection` 辅助（分页、排序的默认实现）

#### NLP（loon-nlp）
- `NlpService` trait（工厂方法：text_generator / schematic_generator / embedder / tokenizer / moderater）
- `StreamingTextGenerator` + `SchematicGenerator<T>` trait
- `Schematic` derive macro
- OpenAI provider（`OpenAiProvider`）：chat completions + SSE 流式 + JSON schema 结构化输出
- `FallbackSchematicGenerator`（声明，Phase 1 仅单 provider 不启用 chain 逻辑）
- `NlpConfig` + `GenerationInfo` + `NlpError`

#### 引擎（loon-engine）
- `Engine` trait（`process` + `utter` 两个方法，对齐 parlcant `engines/types.py`）
- `AlphaEngine` 主类，含完整 4 阶段流水线 + 迭代
- **Guideline 匹配子系统**（`guideline_matching/`）：
  - `GuidelineMatcher` trait
  - `GuidelineMatch` 数据结构
  - `GuidelineMatchingContext`
  - `LlmGuidelineMatcher`（默认实现，用 SchematicGenerator 调 LLM 匹配）
  - `CustomGuidelineMatchingStrategy` trait（可扩展）
  - `GenericGuidelineMatchingStrategyResolver`
- **Tool 调用子系统**（`tool_calling/`）：
  - `ToolCaller` trait
  - `ToolCallBatch` trait（`SingleToolBatch` + `OverlappingToolsBatch`）
  - `DefaultToolCallBatcher`
  - `ToolInsights` + `ToolCallEvaluation`
- **关系解析器**（`RelationalResolver`）：处理 guideline 之间的 dependency/exclusion 传播
- **Canned Response 生成器**（`CannedResponseGenerator`）：严格输出模式下的模板匹配
- **Message 生成器**（`MessageGenerator`）：流体输出模式下的自由生成
- **Prompt 构建器**（`PromptBuilder`）：组装最终 LLM prompt 的核心逻辑
- **Tool Event 生成器**（`ToolEventGenerator`）：工具调用事件的格式化
- **Message Event 组合器**（`MessageEventComposer`）
- **Planner**（`Planner` trait + `NoopPlanner`）
- **Engine Hooks**（`EngineHooks`）：引擎生命周期的 15 个钩子点
- **优化策略**（`OptimizationPolicy` + `PerceivedPerformancePolicy`）
- `EngineContext`（含 `Interaction` + `ResponseState` + `IterationState`）
- `EntityContext`（ctxvar 传递当前 agent/customer/session）

#### 索引服务（loon-indexing）
- `Indexer` trait
- `BehavioralChangeEvaluation`
- `GuidelineActionProposer` / `GuidelineAgentIntentionProposer` / `GuidelineContinuousProposer`
- `JourneyReachableNodesEvaluation`
- `ToolRunningActionDetector` / `CustomerDependentActionDetector`
- `RelativeActionProposer`
- Phase 1 仅声明 trait，默认实现为 noop / stub

#### SDK（loon-sdk）
- 与 parlcant `sdk.py`（5918 行）功能对齐的声明式 Rust API
- 公开类型：`Server` / `ServerBuilder` / `Agent` / `Guideline` / `GuidelineMatch` / `Observation` / `Journey` / `Tool` / `ToolContext` / `Capability` / `Retriever` / `Term` / `Variable` / `Customer` / `Session` / `Tag` / `AnyOf` / `AllOf` / `Relationship` / `CompositionMode` / `MessageOutputMode`
- 内置常量：`MATCH_ALWAYS`
- `Server` 生命周期：builder → build → run (async closure)

#### HTTP/WS 服务（loon-server）
- axum HTTP + WebSocket
- 约 30 个 REST 端点（对齐 parlcant FastAPI 路由）
- WS 聊天端点（`/v1/sessions/{id}/chat`）
- 配置：`loon.toml` + 环境变量
- 健康检查（`/health` + `/version`）
- `ApiError` 枚举 + `ApiResponse<T>` / `ApiListResponse<T>` 统一响应格式

#### CLI（loon 二进制）
- clap 子命令（对齐 parlcant CLI）
- REPL 聊天模式

#### 测试
- 单元测试覆盖每个 crate
- `tests/e2e_agent_loop.rs` 集成测试
- 文档测试

### 1.2 Out of Scope（后续 phase 处理）

- 其他 LLM provider（Anthropic / Gemini / Vertex / Ollama / LiteLLM / Bedrock / Together / Cerebras / DeepSeek）
- MongoDB / Chroma / Qdrant 后端
- 向量数据库实现（仅保留 trait）
- MCP / OpenAPI / Plugin 服务
- 文档迁移执行逻辑
- TS 前端（`loon-chat-ui`）
- OpenTelemetry / 指标 / 分布式追踪
- 限速 / 配额 / token 计费
- OAuth / API Key 授权（仅保留 placeholder trait）
- 增量索引与行为变更评估的完整实现

---

## 2. 全局项目结构

### 2.1 Workspace

仓库根使用 Cargo workspace。

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
│   ├── loon-core/                   # 域实体 + 公共抽象 + CQ
│   ├── loon-emission/               # 事件发射 / 缓冲 / 发布
│   ├── loon-app-modules/            # 应用模块层（封装 Store 业务语义）
│   ├── loon-persistence/            # 文档 / 向量数据库抽象 + JSON 后端
│   ├── loon-nlp/                    # NLP 抽象 + OpenAI 实现
│   ├── loon-engine/                 # AlphaEngine + 全部 strategy + indexing
│   ├── loon-sdk/                    # 公开 SDK
│   ├── loon-server/                 # axum HTTP/WS 服务
│   └── loon/                        # CLI 客户端二进制
└── tests/
    └── e2e_agent_loop.rs            # workspace 级集成测试
```

### 2.2 依赖方向（强制，CI 检查）

```text
loon-server ──▶ loon-sdk ──▶ loon-engine ──┬──▶ loon-app-modules ──▶ loon-emission ──▶ loon-core
                                            ├──▶ loon-nlp
                                            └──▶ loon-persistence
loon (CLI) ──▶ loon-sdk
```

- `loon-core` 不依赖任何其他 loon crate。
- `loon-emission` 仅依赖 `loon-core`。
- `loon-app-modules` 依赖 `loon-core` + `loon-emission`。
- `loon-nlp` / `loon-persistence` 仅依赖 `loon-core`。
- `loon-engine` 依赖 `loon-core` / `loon-emission` / `loon-app-modules` / `loon-nlp` / `loon-persistence`。
- `loon-sdk` 依赖 `loon-engine` / `loon-nlp` / `loon-persistence` / `loon-app-modules`。
- `loon-server` / `loon` 仅依赖 `loon-sdk`。

依赖方向违反由 `cargo-deny` 在 CI 中强制检查。

---

## 3. 核心域（`loon-core`）

> 对应 Parlant: `src/parlant/core/agents.py`, `guidelines.py`, `journeys.py`, `tools.py`, `sessions.py`, `customers.py`, `tags.py`, `glossary.py`, `context_variables.py`, `canned_responses.py`, `capabilities.py`, `relationships.py`, `evaluations.py`, `common.py`, `entity_cq.py`, `journey_guideline_projection.py`, `guideline_tool_associations.py`, `shots.py`, `async_utils.py`, `background_tasks.py`, `event_loop_monitor.py`, `meter.py`

### 3.1 公共类型（`common.rs`）

```rust
// crates/loon-core/src/common.rs

/// 语义版本
pub struct Version { pub major: u32, pub minor: u32, pub patch: u32 }
// JSON 可序列化值的类型别名
pub type JsonValue = serde_json::Value;
/// 唯一 ID 的 newtype
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UniqueId(pub String);
/// Guideline 关键程度
pub enum Criticality { Low, Medium, High }
/// 分页
pub struct Pagination { pub offset: usize, pub limit: usize }
```

### 3.2 ID 类型（`ids.rs`）

每个实体一个 newtype ID，通过 `define_id!` 宏生成，附带 `nanoid` 随机生成 + 确定性 ID（基于 `xxh3` checksum 的 `IdGenerator`）。

```rust
// crates/loon-core/src/ids.rs
define_id!(AgentId);
define_id!(GuidelineId);
define_id!(JourneyId);
define_id!(JourneyNodeId);       // ← 新增：Journey 节点独立 ID
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
define_id!(GuidelineToolAssociationId);  // ← 新增
define_id!(JourneyEdgeId);               // ← 新增
```

### 3.3 IdGenerator（`id_generator.rs`）

```rust
// crates/loon-core/src/id_generator.rs
pub struct IdGenerator { /* 内部维护 checksum → 计数器映射 */ }

impl IdGenerator {
    pub fn new() -> Self;
    /// 基于 content_checksum 生成确定性 ID（xxh3 → base64 → 截断 10 字符）
    pub fn generate(&mut self, content_checksum: &str) -> UniqueId;
    /// 随机 nanoid（10 字符）
    pub fn generate_random() -> UniqueId;
}
```

### 3.4 实体清单（17 个实体 + 3 个枚举 + 辅助类型）

| 实体 | 文件 | Parlant 源 | 关键字段 |
|------|------|-----------|----------|
| `Agent` | `agent.rs` | `core/agents.py` | `id`, `name`, `description`, `composition_mode`, `message_output_mode`, `tags` |
| `Guideline` | `guideline.rs` | `core/guidelines.py` | `id`, `content: GuidelineContent`, `criticality`, `enabled`, `tags`, `metadata`, `composition_mode` |
| `GuidelineContent` | `guideline.rs` | 同上 | `condition: String`, `action: String`, `description: Option<String>` |
| `GuidelineMatch` | `guideline.rs` | `engines/alpha/guideline_matching/guideline_match.py` | `guideline: Guideline`, `confidence: f32`, `rationale: String` |
| `Journey` | `journey.rs` | `core/journeys.py` | `id`, `title`, `description`, `root_id: JourneyNodeId`, `tags`, `triggers` |
| `JourneyNode` | `journey.rs` | 同上 | `id`, `kind: NodeKind`, `action`, `description`, `tools`, `labels`, `metadata`, `composition_mode` |
| `NodeKind` | `journey.rs` | 同上 | `Initial` / `Tool` / `Chat` / `Fork` |
| `JourneyEdge` | `journey.rs` | 同上 | `id`, `source: JourneyNodeId`, `target: JourneyNodeId`, `condition`, `metadata` |
| `Observation` | `observation.rs` | `core/evaluations.py` | `id`, `agent_id`, `condition`, `tools: Vec<ToolId>`, `enabled` |
| `Tool` | `tool.rs` | `core/tools.py` | `id`, `name`, `description`, `parameters_schema`, `kind: ToolKind` |
| `ToolKind` | `tool.rs` | 同上 | `Local` / `OpenAPI` / `MCP` |
| `ToolCall` / `ToolResult` | `tool.rs` | 同上 | 调用事件 / 结果事件，`ToolResult` 含 `data` + `metadata` + `control` + `canned_responses` + `canned_response_fields` |
| `Session` | `session.rs` | `core/sessions.py` | `id`, `agent_id`, `customer_id`, `title`, `mode: SessionMode`, `labels` |
| `SessionMode` | `session.rs` | 同上 | `Auto` / `Manual` |
| `Event` | `session.rs` | 同上 | `id`, `source: EventSource`, `kind: EventKind`, `trace_id`, `data`, `metadata`, `creation_utc` |
| `EventSource` | `session.rs` | 同上 | `Customer` / `AiAgent` / `System` |
| `EventKind` | `session.rs` | 同上 | `Status` / `Message` / `Tool` / `Custom` |
| `EventUpdateParams` | `session.rs` | 同上 | 允许部分更新 event 的 `data` 和 `metadata` |
| `Message` / `MessageEventData` | `session.rs` | 同上 | `message: String`, `participant: Participant` |
| `Participant` | `session.rs` | 同上 | `id: String`, `display_name: String` |
| `StatusEventData` | `session.rs` | 同上 | 状态事件数据 |
| `ToolEventData` | `session.rs` | 同上 | `tool_calls: Vec<ToolCallData>` |
| `Customer` | `customer.rs` | `core/customers.py` | `id`, `name`, `metadata`, `tags` |
| `Glossary` / `Term` | `glossary.rs` | `core/glossary.py` | `id`, `name`, `description`, `synonyms`, `tags` |
| `Variable` / `ContextVariable` | `variable.rs` | `core/context_variables.py` | `id`, `key`, `value`, `freshness_rules`, `tags` |
| `CannedResponse` | `canned_response.rs` | `core/canned_responses.py` | `id`, `value: String`, `tags`, `matchers` |
| `Capability` / `Retriever` | `capability.rs` | `core/capabilities.py` | `id`, `name`, `description`, `tags` |
| `Tag` | `tag.rs` | `core/tags.py` | `id`, `name` — 并有 `for_agent_id()`, `for_guideline_id()`, `for_journey_id()` 等工厂方法 |
| `Relationship` | `relationship.rs` | `core/relationships.py` | `id`, `source: RelationshipEntity`, `target: RelationshipEntity`, `kind: RelationshipKind`, `indirect: bool` |
| `RelationshipEntity` | 同上 | 同上 | `kind: RelationshipEntityKind`, `id: UniqueId` |
| `RelationshipKind` | 同上 | 同上 | `Entails` / `Excludes` / `Dependency` / `Reevaluation` |
| `RelationshipEntityKind` | 同上 | 同上 | `Guideline` / `Tag` / `Tool` / `Journey` 等 |
| `GuidelineToolAssociation` | `guideline_tool.rs` | `core/guideline_tool_associations.py` | `id`, `guideline_id`, `tool_id` |
| `CompositionMode` | `agent.rs` | `core/agents.py` | `Fluid` / `Strict` |
| `MessageOutputMode` | `agent.rs` | 同上 | `Fluid` / `Canned` |

### 3.5 Store trait（15 个，`stores/` 子模块）

每个实体对应一个 Store trait。以 `AgentStore` 为例：

```rust
// crates/loon-core/src/stores/agent.rs
#[async_trait]
pub trait AgentStore: Send + Sync {
    async fn create(&self, agent: Agent) -> Result<Agent>;
    async fn read(&self, id: &AgentId) -> Result<Option<Agent>>;
    async fn update(&self, id: &AgentId, params: AgentUpdateParams) -> Result<Agent>;
    async fn delete(&self, id: &AgentId) -> Result<()>;
    async fn list(&self, tags: &[TagId]) -> Result<Vec<Agent>>;
}
```

同理：`GuidelineStore` / `JourneyStore` / `SessionStore` / `CustomerStore` / `TagStore` / `GlossaryStore` / `ContextVariableStore` / `CannedResponseStore` / `CapabilityStore` / `RelationshipStore` / `GuidelineToolAssociationStore` / `ToolStore` / `EvaluationStore` / `RetrieverStore`。

### 3.6 EntityCQ（Command-Query 分离）

> 对应 Parlant: `src/parlant/core/entity_cq.py`

```rust
// crates/loon-core/src/entity_cq.rs

/// 只读查询入口，封装所有 Store 的读路径
pub struct EntityQueries {
    agent_store: Arc<dyn AgentStore>,
    session_store: Arc<dyn SessionStore>,
    guideline_store: Arc<dyn GuidelineStore>,
    customer_store: Arc<dyn CustomerStore>,
    context_variable_store: Arc<dyn ContextVariableStore>,
    relationship_store: Arc<dyn RelationshipStore>,
    guideline_tool_association_store: Arc<dyn GuidelineToolAssociationStore>,
    glossary_store: Arc<dyn GlossaryStore>,
    journey_store: Arc<dyn JourneyStore>,
    canned_response_store: Arc<dyn CannedResponseStore>,
    capability_store: Arc<dyn CapabilityStore>,
    journey_guideline_projection: Arc<JourneyGuidelineProjection>,
}

impl EntityQueries {
    // —— 基础读 ——
    pub async fn read_agent(&self, id: &AgentId) -> Result<Agent>;
    pub async fn read_session(&self, id: &SessionId) -> Result<Session>;
    pub async fn read_customer(&self, id: &CustomerId) -> Result<Customer>;
    pub async fn find_events(&self, session_id: &SessionId) -> Result<Vec<Event>>;

    // —— 上下文装填（引擎核心依赖） ——
    pub async fn find_guidelines_for_context(
        &self, agent_id: &AgentId, journeys: &[Journey]
    ) -> Result<Vec<Guideline>>;
    // 逻辑：agent guidelines + global + agent-tag guidelines + journey guidelines + projected

    pub async fn find_context_variables_for_context(
        &self, agent_id: &AgentId
    ) -> Result<Vec<ContextVariable>>;

    pub async fn find_capabilities_for_agent(
        &self, agent_id: &AgentId, query: &str, max_count: usize
    ) -> Result<Vec<Capability>>;

    pub async fn find_glossary_terms_for_context(
        &self, agent_id: &AgentId, query: &str
    ) -> Result<Vec<Term>>;

    pub async fn find_journeys_for_context(
        &self, agent_id: &AgentId
    ) -> Result<Vec<Journey>>;

    pub async fn find_canned_responses_for_context(
        &self, agent: &Agent, journeys: &[Journey], guidelines: &[Guideline]
    ) -> Result<Vec<CannedResponse>>;

    pub async fn find_guidelines_that_need_reevaluation(
        &self, available_guidelines: &HashMap<GuidelineId, Guideline>,
        active_journeys: &[Journey], tool_insights: &ToolInsights,
    ) -> Result<Vec<Guideline>>;

    // —— 关系遍历 ——
    pub async fn find_journey_related_guidelines(
        &self, journey: &Journey
    ) -> Result<Vec<GuidelineId>>;
}

/// 写操作入口
pub struct EntityCommands {
    session_store: Arc<dyn SessionStore>,
    context_variable_store: Arc<dyn ContextVariableStore>,
}

impl EntityCommands {
    pub async fn update_session(&self, session_id: &SessionId, params: SessionUpdateParams) -> Result<()>;
    pub async fn update_context_variable_value(
        &self, variable_id: &ContextVariableId, key: &str, data: JsonValue
    ) -> Result<ContextVariableValue>;
    pub async fn upsert_session_labels(&self, session_id: &SessionId, labels: HashSet<String>) -> Result<Session>;
}
```

### 3.7 JourneyGuidelineProjection（Journey → Guideline 投影）

> 对应 Parlant: `src/parlant/core/journey_guideline_projection.py`

```rust
// crates/loon-core/src/journey_guideline_projection.rs
pub struct JourneyGuidelineProjection {
    journey_store: Arc<dyn JourneyStore>,
    guideline_store: Arc<dyn GuidelineStore>,
}

impl JourneyGuidelineProjection {
    /// 将 Journey 的节点/边图 BFS 展平为 Guideline 列表
    /// 每个 (node, edge) 组合生成一个合成的 Guideline：
    ///   - id = "journey_node:{node_id}:{edge_id}"
    ///   - condition = edge.condition
    ///   - action = node.action
    ///   - metadata 含 journey_node 嵌套结构（follow_ups, index, journey_id, labels, tool_ids）
    pub async fn project_journey_to_guidelines(
        &self, journey_id: &JourneyId
    ) -> Result<Vec<Guideline>>;
}

/// 从 journey_node:xxx 格式的 GuidelineId 提取 JourneyNodeId
pub fn extract_node_id_from_journey_node_guideline_id(id: &GuidelineId) -> Option<JourneyNodeId>;
```

### 3.8 通用工具（`async_utils.rs` / `background_tasks.rs` / `event_loop_monitor.rs` / `meter.rs`）

> 对应 Parlant: `src/parlant/core/async_utils.py`, `background_tasks.py`, `event_loop_monitor.py`, `meter.py`

```rust
// crates/loon-core/src/async_utils.rs
pub async fn safe_gather<T>(futures: Vec<impl Future<Output = Result<T>>>) -> Vec<Result<T>>;
pub struct Stopwatch { /* 高精度计时器 */ }
pub struct ReaderWriterLock<T> { /* tokio::sync::RwLock 的包装 */ }

// crates/loon-core/src/meter.rs
pub struct Meter { /* 简单内存计数器：请求数 / 延迟 / 错误数 */ }

// crates/loon-core/src/logger.rs
/// 日志抽象（Phase 1: 基于 tracing/log crate 的简单实现）
#[async_trait]
pub trait Logger: Send + Sync {
    fn info(&self, msg: &str, context: &HashMap<&str, JsonValue>);
    fn warning(&self, msg: &str, context: &HashMap<&str, JsonValue>);
    fn error(&self, msg: &str, context: &HashMap<&str, JsonValue>);
    fn debug(&self, msg: &str, context: &HashMap<&str, JsonValue>);
    fn is_enabled_for(&self, level: LogLevel) -> bool;
}

pub enum LogLevel { Debug, Info, Warning, Error }

// crates/loon-core/src/tracer.rs
/// 追踪抽象（Phase 1: 简单 UUID trace_id + HashMap 属性存储；Phase 10 接入 OpenTelemetry）
pub trait Tracer: Send + Sync {
    fn trace_id(&self) -> &str;
    fn set_property(&self, key: &str, value: JsonValue);
    fn get_property(&self, key: &str) -> Option<JsonValue>;
    /// 创建子 span，执行完成后自动关闭
    async fn span<F, Fut, T>(&self, name: &str, f: F) -> T
    where F: FnOnce() -> Fut, Fut: Future<Output = T>;
}
```

### 3.9 Shots（Few-Shot 示例管理）

> 对应 Parlant: `src/parlant/core/shots.py` (2715 bytes)

Parlant 允许为 agent 定义 few-shot 示例，prompt_builder 在组装 prompt 时注入这些示例以提高生成质量。

```rust
// crates/loon-core/src/shots.rs

/// 一个 few-shot 示例
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shot {
    pub id: ShotId,
    pub agent_id: AgentId,
    /// 触发此示例的条件描述
    pub condition: String,
    /// 期望 agent 执行的动作
    pub action: String,
    /// 示例用户输入
    pub example_input: String,
    /// 示例理想输出
    pub example_output: String,
    pub creation_utc: DateTime<Utc>,
}

define_id!(ShotId);

#[async_trait]
pub trait ShotStore: Send + Sync {
    async fn create(&self, shot: Shot) -> Result<Shot>;
    async fn read(&self, id: &ShotId) -> Result<Option<Shot>>;
    async fn delete(&self, id: &ShotId) -> Result<()>;
    async fn list(&self, agent_id: &AgentId) -> Result<Vec<Shot>>;
}
```

### 3.10 ServiceRegistry + ToolService（工具服务注册与调用）

> 对应 Parlant: `src/parlant/core/services/tools/service_registry.py` (14495 bytes)

`EntityQueries` 通过 `ServiceRegistry` 解析工具服务名称，引擎的 `ToolCaller` 通过 `ToolService` 执行工具。

```rust
// crates/loon-core/src/tool_service.rs

/// 工具执行服务——知道如何调用一种类型的工具（Local / OpenAPI / MCP）
#[async_trait]
pub trait ToolService: Send + Sync {
    /// 列出此服务提供的所有工具
    async fn list_tools(&self) -> Result<Vec<Tool>>;
    /// 调用指定工具
    async fn call_tool(&self, tool_id: &ToolId, arguments: JsonValue) -> Result<ToolResult>;
}

// crates/loon-core/src/service_registry.rs

/// 服务注册表——按名称查找 ToolService
#[async_trait]
pub trait ServiceRegistry: Send + Sync {
    /// 按服务名读取 ToolService
    async fn read_tool_service(&self, service_name: &str) -> Result<Box<dyn ToolService>>;
    /// 注册一个 ToolService（用于启动时装配）
    async fn register(&self, name: &str, service: Box<dyn ToolService>) -> Result<()>;
    /// 列出所有已注册的服务名
    async fn list_services(&self) -> Result<Vec<String>>;
}

/// Phase 1 默认实现：内存 HashMap
pub struct InMemoryServiceRegistry {
    services: RwLock<HashMap<String, Box<dyn ToolService>>>,
}

impl InMemoryServiceRegistry {
    pub fn new() -> Self;
}

#[async_trait]
impl ServiceRegistry for InMemoryServiceRegistry { /* ... */ }

/// Phase 1 默认的本地工具执行服务
pub struct LocalToolService {
    tools: Vec<Tool>,
    /// 工具 ID → 闭包映射（用户注册的本地函数）
    handlers: HashMap<ToolId, Box<dyn Fn(JsonValue) -> Pin<Box<dyn Future<Output = Result<ToolResult>>>> + Send + Sync>>,
}
```

---

## 4. 事件发射（`loon-emission`）

> 对应 Parlant: `src/parlant/core/emissions.py`, `emission/event_buffer.py`, `emission/event_publisher.py`

这是原方案完全遗漏的关键层。Parlant 通过 `EventEmitter` / `EventBuffer` / `EventPublisher` 实现了事件驱动的引擎通信。

### 4.1 核心抽象

```rust
// crates/loon-emission/src/emitter.rs

/// 未持久化的事件表示
#[derive(Debug, Clone)]
pub struct EmittedEvent {
    pub source: EventSource,
    pub kind: EventKind,
    pub trace_id: String,
    pub data: JsonValue,
    pub metadata: Option<HashMap<String, JsonValue>>,
}

/// 消息事件句柄——允许在流式生成过程中原地更新消息内容
#[derive(Debug, Clone)]
pub struct MessageEventHandle {
    pub event: EmittedEvent,
    pub update: Arc<dyn Fn(MessageEventData) -> Pin<Box<dyn Future<Output = Result<MessageEventHandle>>>> + Send + Sync>,
}

/// 事件发射器 trait
#[async_trait]
pub trait EventEmitter: Send + Sync {
    async fn emit_status_event(
        &self, trace_id: &str, data: StatusEventData,
        metadata: Option<HashMap<String, JsonValue>>,
    ) -> Result<EmittedEvent>;

    async fn emit_message_event(
        &self, trace_id: &str, data: MessageEmitData,
        metadata: Option<HashMap<String, JsonValue>>,
    ) -> Result<MessageEventHandle>;

    async fn emit_tool_event(
        &self, trace_id: &str, data: ToolEventData,
        metadata: Option<HashMap<String, JsonValue>>,
    ) -> Result<EmittedEvent>;

    async fn emit_custom_event(
        &self, trace_id: &str, data: JsonValue,
        metadata: Option<HashMap<String, JsonValue>>,
    ) -> Result<EmittedEvent>;
}

/// 消息数据可以是字符串（简单）或结构化 MessageEventData
pub enum MessageEmitData {
    Simple(String),
    Structured(MessageEventData),
}

/// EventEmitter 工厂
#[async_trait]
pub trait EventEmitterFactory: Send + Sync {
    async fn create_event_emitter(
        &self, emitting_agent_id: &AgentId, session_id: &SessionId,
    ) -> Result<Box<dyn EventEmitter>>;
}
```

### 4.2 EventBuffer（内存实现）

```rust
// crates/loon-emission/src/buffer.rs

/// 内存事件缓冲器——所有事件暂存到 Vec<EmittedEvent>
/// 用于引擎内部收集事件，处理完成后再批量持久化
pub struct EventBuffer {
    pub agent: Agent,
    pub events: Vec<EmittedEvent>,
}

impl EventBuffer {
    pub fn new(emitting_agent: Agent) -> Self;
}

#[async_trait]
impl EventEmitter for EventBuffer {
    // emit_status_event → push to self.events
    // emit_message_event → push + 返回带 updater 的 MessageEventHandle
    // emit_tool_event → push
    // emit_custom_event → push
}

pub struct EventBufferFactory { agent_store: Arc<dyn AgentStore> }
impl EventEmitterFactory for EventBufferFactory { /* ... */ }
```

### 4.3 EventPublisher（持久化实现）

```rust
// crates/loon-emission/src/publisher.rs

/// 持久化事件发布器——每个事件直接写 SessionStore
pub struct EventPublisher {
    agent: Agent,
    session_store: Arc<dyn SessionStore>,
    session_id: SessionId,
}

#[async_trait]
impl EventEmitter for EventPublisher {
    // 每次 emit 都调用 session_store.create_event(...)
    // MessageEventHandle 的 updater 调用 session_store.update_event(...)
}

pub struct EventPublisherFactory {
    agent_store: Arc<dyn AgentStore>,
    session_store: Arc<dyn SessionStore>,
}
impl EventEmitterFactory for EventPublisherFactory { /* ... */ }
```

### 4.4 使用模式

引擎内部使用 `EventBuffer` 收集本轮所有事件（status / tool / message），处理完成后：
1. 通过 `EventPublisher` 将 buffer 中的事件逐条持久化到 SessionStore
2. 或者直接通过 `EventPublisher` 流式写入（适合 WS 实时推送）

---

## 5. AppModules 层（`loon-app-modules`）

> 对应 Parlant: `src/parlant/core/app_modules/` (13 个文件, ~78 KB)

原方案完全遗漏了这一层。AppModules 封装 Store 的业务语义，是连接域实体与引擎/API 的胶水层。

### 5.1 模块清单

```rust
// crates/loon-app-modules/src/lib.rs
pub mod agents;
pub mod canned_responses;
pub mod capabilities;
pub mod context_variables;
pub mod customers;
pub mod evaluations;
pub mod glossary;
pub mod guidelines;
pub mod journeys;
pub mod relationships;
pub mod sessions;
pub mod tags;
pub mod services;
```

### 5.2 典型模块接口（以 `GuidelineAppModule` 为例）

```rust
// crates/loon-app-modules/src/guidelines.rs

pub struct GuidelineAppModule {
    store: Arc<dyn GuidelineStore>,
    relationship_store: Arc<dyn RelationshipStore>,
    association_store: Arc<dyn GuidelineToolAssociationStore>,
    id_generator: Arc<Mutex<IdGenerator>>,
}

impl GuidelineAppModule {
    pub fn new(/* ... */) -> Self;

    pub async fn create_guideline(
        &self, params: GuidelineCreateParams,
    ) -> Result<Guideline>;

    pub async fn read_guideline(&self, id: &GuidelineId) -> Result<Option<Guideline>>;

    pub async fn update_guideline(
        &self, id: &GuidelineId, params: GuidelineUpdateParams,
    ) -> Result<Guideline>;

    pub async fn delete_guideline(&self, id: &GuidelineId) -> Result<()>;

    pub async fn list_guidelines(
        &self, agent_id: &AgentId, tags: &[TagId],
    ) -> Result<Vec<Guideline>>;

    /// 创建 guideline 间的依赖关系
    pub async fn add_dependency(
        &self, source_id: &GuidelineId, target_tag: &TagId,
    ) -> Result<Relationship>;

    /// 创建 guideline 间的互斥关系
    pub async fn exclude(
        &self, excluder_id: &GuidelineId, excluded_tag: &TagId,
    ) -> Result<Relationship>;

    /// 关联 guideline 与 tool
    pub async fn associate_tool(
        &self, guideline_id: &GuidelineId, tool_id: &ToolId,
    ) -> Result<GuidelineToolAssociation>;
}
```

类似结构覆盖其余 12 个模块，每个模块封装对应 Store + 相关 Store 的跨实体业务操作。

---

## 6. 持久化（`loon-persistence`）

> 对应 Parlant: `src/parlant/core/persistence/` (document_database, vector_database, common, 及辅助), `adapters/db/`

### 6.1 文档数据库抽象

```rust
// crates/loon-persistence/src/document.rs

#[async_trait]
pub trait DocumentDatabase: Send + Sync {
    async fn get_or_create_collection<TDocument: Document>(
        &self, name: &str, schema: DocumentSchema, document_loader: DocumentLoader<TDocument>,
    ) -> Result<Box<dyn DocumentCollection<TDocument>>>;
}

#[async_trait]
pub trait DocumentCollection<TDocument: Document>: Send + Sync {
    async fn insert_one(&self, document: TDocument) -> Result<InsertResult>;
    async fn find_one(&self, filters: &DocumentFilter) -> Result<Option<TDocument>>;
    async fn find(&self, filters: &DocumentFilter) -> Result<Vec<TDocument>>;
    async fn update_one(&self, filters: &DocumentFilter, update: DocumentUpdate) -> Result<UpdateResult>;
    async fn delete_one(&self, filters: &DocumentFilter) -> Result<DeleteResult>;
    async fn count(&self, filters: &DocumentFilter) -> Result<u64>;
}

/// 文档必须实现此 trait
pub trait Document: Serialize + DeserializeOwned + Send + Sync + 'static {
    /// 文档格式版本号——预留迁移
    const VERSION: &'static str;
    type Id: Serialize + DeserializeOwned + Send + Sync + Eq + Hash;
    fn id(&self) -> &Self::Id;
}

/// 版本化文档辅助类型
pub struct VersionedDocument<T> {
    pub version: String,       // 如 "0.1.0"
    pub content: T,
}

/// 文档加载器：从原始 BaseDocument 反序列化为具体类型，支持多版本兼容
pub type DocumentLoader<T> = Arc<dyn Fn(&BaseDocument) -> Option<T> + Send + Sync>;
```

### 6.2 Filter 表达式

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DocumentFilter {
    Eq { field: String, value: JsonValue },
    In { field: String, values: Vec<JsonValue> },
    And(Vec<DocumentFilter>),
    Or(Vec<DocumentFilter>),
    Not(Box<DocumentFilter>),
}
```

### 6.3 新增：DataCollection 辅助

> 对应 Parlant: `src/parlant/core/persistence/data_collection.py` (3676 bytes)

在 `DocumentCollection` trait 上追加便捷方法（分页、排序），Phase 1 在 trait 中声明默认实现：

```rust
// crates/loon-persistence/src/data_collection.rs
// 作为 DocumentCollection trait 的扩展方法

#[async_trait]
pub trait DocumentCollection<TDocument: Document>: Send + Sync {
    // ... 基础 CRUD ...

    /// 分页查询
    async fn find_paginated(
        &self, filter: &DocumentFilter, offset: usize, limit: usize,
    ) -> Result<PaginatedResult<TDocument>> {
        let all = self.find(filter).await?;
        let total = all.len();
        let items = all.into_iter().skip(offset).take(limit).collect();
        Ok(PaginatedResult { items, total, offset, limit })
    }

    /// 排序查询
    async fn find_sorted(
        &self, filter: &DocumentFilter, sort_by: &str, ascending: bool,
    ) -> Result<Vec<TDocument>>;
}

pub struct PaginatedResult<T> {
    pub items: Vec<T>,
    pub total: usize,
    pub offset: usize,
    pub limit: usize,
}
```

### 6.4 JSON 文件后端（`backends/json_file.rs`）

与初版设计一致，增加：

- 每个 collection 一个目录，文件命名 `<doc_id>.json`
- 写入用临时文件 + 原子 `rename`
- 启动时全量加载到 `Arc<RwLock<HashMap<UniqueId, TDocument>>>`
- 后台 flush 任务（默认 5 秒间隔）
- 支持 `DocumentFilter` 的内存过滤

```rust
pub struct JsonFileDocumentDatabase {
    root_path: PathBuf,
    flush_interval: Duration,   // 默认 5s
}

pub struct JsonFileDocumentCollection<T: Document> {
    name: String,
    dir: PathBuf,
    cache: Arc<RwLock<HashMap<UniqueId, T>>>,
    loader: DocumentLoader<T>,
    flush_interval: Duration,
}
```

### 6.4 向量数据库抽象（占位）

与初版设计一致，`VectorDatabase` trait 仅声明，Phase 1 无实现。

### 6.5 DocumentStoreMigrationHelper（占位）

```rust
// crates/loon-persistence/src/migration.rs

/// 迁移辅助——Phase 1 仅检查 version 是否匹配，不执行迁移逻辑
pub struct DocumentStoreMigrationHelper {
    database: Arc<dyn DocumentDatabase>,
    allow_migration: bool,
}

impl DocumentStoreMigrationHelper {
    pub async fn enter(&self) -> Result<()>;
    // Phase 1 实现：遍历所有 collection，跳过 version!=当前版本的文档，
    // 若 allow_migration=true 且存在旧版本文档则 panic（提醒实现迁移逻辑）
}
```

---

## 7. NLP（`loon-nlp`）

> 对应 Parlant: `src/parlant/core/nlp/` (generation, embedding, moderation, policies, tokenization, service)

与初版设计保持一致，补充以下细节：

### 7.1 核心抽象（不变）

```rust
pub struct NlpConfig { /* provider, model, endpoint, api_key, max_retries, timeout, temperature */ }

#[async_trait]
pub trait NlpService: Send + Sync {
    fn config(&self) -> &NlpConfig;
    async fn text_generator(&self) -> Result<Box<dyn StreamingTextGenerator>>;
    async fn schematic_generator<T: Schematic>(&self) -> Result<Box<dyn SchematicGenerator<T>>>;
    async fn embedder(&self) -> Result<Box<dyn Embedder>>;
    async fn tokenizer(&self) -> Result<Box<dyn Tokenizer>>;
    async fn moderater(&self) -> Result<Box<dyn Moderater>>;
}
```

### 7.2 新增：GenerationInfo

```rust
// crates/loon-nlp/src/generation_info.rs
pub struct GenerationInfo {
    pub model: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub finish_reason: String,
    pub latency_ms: u64,
}
```

### 7.3 新增：Moderation

```rust
// crates/loon-nlp/src/moderation.rs
#[async_trait]
pub trait Moderater: Send + Sync {
    async fn moderate(&self, text: &str) -> Result<ModerationResult>;
}

pub struct ModerationResult {
    pub flagged: bool,
    pub categories: HashMap<String, bool>,
    pub scores: HashMap<String, f32>,
}
```

### 7.4 新增：Policies（限速/重试）

```rust
// crates/loon-nlp/src/policies.rs
pub struct RetryPolicy {
    pub max_retries: u32,
    pub backoff_base_ms: u64,
    pub backoff_multiplier: f64,
    pub retry_on_status: Vec<u16>,     // 默认 [429, 500, 502, 503, 504]
}

pub struct RateLimitPolicy {
    pub max_requests_per_minute: u32,
    pub max_tokens_per_minute: u32,
}
```

### 7.5 OpenAI provider / Schematic macro / Fallback

与初版设计一致。

---

## 8. 引擎（`loon-engine`）

> 对应 Parlant: `src/parlant/core/engines/alpha/` (20 个文件, ~500 KB), `engines/types.py`

这是原方案简化最严重的部分。修订后完整覆盖所有子组件。

### 8.1 Engine trait

```rust
// crates/loon-engine/src/engine.rs

/// 引擎上下文（来自 parlcant engines/types.py Context）
pub struct Context {
    pub session_id: SessionId,
    pub agent_id: AgentId,
}

/// Utterance 请求（引擎主动发起的说话请求）
pub struct UtteranceRequest {
    pub action: String,
    pub rationale: UtteranceRationale,
}

pub enum UtteranceRationale { Unspecified, BuyTime, FollowUp }

#[async_trait]
pub trait Engine: Send + Sync {
    /// 处理一个会话回合——主入口
    async fn process(
        &self, context: &Context, event_emitter: &dyn EventEmitter,
    ) -> Result<bool>;

    /// 主动发话（如旅程中的 follow-up）
    async fn utter(
        &self, context: &Context, event_emitter: &dyn EventEmitter,
        requests: &[UtteranceRequest],
    ) -> Result<bool>;
}
```

### 8.2 EngineContext

```rust
// crates/loon-engine/src/engine_context.rs

/// 交互历史（从 session events 重建）
pub struct Interaction {
    pub events: Vec<Event>,

    /// 提取所有消息事件为 InteractionMessage
    pub fn messages(&self) -> Vec<InteractionMessage>;
    /// 获取最后一条客户消息
    pub fn last_customer_message(&self) -> Option<InteractionMessage>;
}

pub struct InteractionMessage {
    pub source: EventSource,
    pub participant: Participant,
    pub trace_id: String,
    pub content: String,
    pub creation_utc: DateTime<Utc>,
}

/// 单次迭代状态
pub struct IterationState {
    pub matched_guidelines: Vec<GuidelineMatch>,
    pub resolved_guidelines: Vec<GuidelineMatch>,
    pub tool_insights: ToolInsights,
    pub executed_tools: Vec<ToolId>,
}

/// 响应状态（可变的中间状态）
pub struct ResponseState {
    pub context_variables: Vec<(ContextVariable, ContextVariableValue)>,
    pub glossary_terms: HashSet<Term>,
    pub capabilities: Vec<Capability>,
    pub iterations: Vec<IterationState>,
    pub ordinary_guideline_matches: Vec<GuidelineMatch>,
    pub tool_enabled_guideline_matches: HashMap<GuidelineMatch, Vec<ToolId>>,
    pub journeys: Vec<Journey>,
    pub journey_paths: HashMap<JourneyId, Vec<Option<String>>>,
    pub tool_events: Vec<EmittedEvent>,
    pub tool_insights: ToolInsights,
    pub prepared_to_respond: bool,
    pub message_events: Vec<EmittedEvent>,
    pub usable_guidelines: Vec<Guideline>,
    pub additional_canned_response_fields: HashMap<String, JsonValue>,
}

/// 引擎上下文——贯穿整个 process_turn 调用
pub struct EngineContext {
    pub info: Context,
    pub logger: Arc<dyn Logger>,
    pub tracer: Arc<dyn Tracer>,
    pub agent: Agent,
    pub customer: Customer,
    pub session: Session,
    pub session_event_emitter: Box<dyn EventEmitter>,
    pub response_event_emitter: Box<dyn EventEmitter>,
    pub interaction: Interaction,
    pub state: ResponseState,
    pub creation: Stopwatch,

    pub async fn add_tool_event(&mut self, tool_id: &ToolId, arguments: JsonValue, result: ToolResult);
}
```

### 8.3 EntityContext

```rust
// crates/loon-engine/src/entity_context.rs

/// 利用 tokio task-local 在整个引擎处理链路中传递 EngineContext
/// 等价于 parlcant 的 contextvars.ContextVar
pub struct EntityContext;

impl EntityContext {
    pub fn get() -> Option<EngineContext>;
    pub fn set(ctx: EngineContext);
    pub fn get_agent() -> Option<Agent>;
    pub fn get_customer() -> Option<Customer>;
    pub fn get_session() -> Option<Session>;
    pub fn get_interaction() -> Option<Interaction>;
    pub fn get_variable_value(variable_id: &ContextVariableId) -> Option<ContextVariableValue>;
}
```

### 8.4 EngineHooks（引擎生命周期钩子）

```rust
// crates/loon-engine/src/hooks.rs

pub enum EngineHookResult { CallNext, Resolve, Bail }

pub type EngineHook = Arc<
    dyn Fn(EngineContext, Option<JsonValue>, Option<anyhow::Error>)
        -> Pin<Box<dyn Future<Output = Result<EngineHookResult>>>> + Send + Sync
>;

pub struct EngineHooks {
    pub on_error: Vec<EngineHook>,
    pub on_acknowledging: Vec<EngineHook>,
    pub on_acknowledged: Vec<EngineHook>,
    pub on_generating_preamble: Vec<EngineHook>,
    pub on_preamble_generated: Vec<EngineHook>,
    pub on_preamble_emitted: Vec<EngineHook>,
    pub on_preparing: Vec<EngineHook>,
    pub on_preparation_iteration_start: Vec<EngineHook>,
    pub on_preparation_iteration_end: Vec<EngineHook>,
    pub on_generating_messages: Vec<EngineHook>,
    pub on_draft_generated: Vec<EngineHook>,
    pub on_message_generated: Vec<EngineHook>,
    pub on_messages_emitted: Vec<EngineHook>,

    // Per-entity hooks
    pub on_guideline_selected: HashMap<GuidelineId, Vec</* ... */>>,
    pub on_journey_selected: HashMap<JourneyId, Vec</* ... */>>,
}
```

### 8.5 Guideline 匹配子系统（`guideline_matching/`）

> 对应 Parlant: `src/parlant/core/engines/alpha/guideline_matching/` (7 个文件)

```rust
// crates/loon-engine/src/guideline_matching/matcher.rs

#[async_trait]
pub trait GuidelineMatcher: Send + Sync {
    async fn match_guidelines(
        &self, ctx: &GuidelineMatchingContext,
    ) -> Result<Vec<GuidelineMatch>>;
}

// crates/loon-engine/src/guideline_matching/context.rs
pub struct GuidelineMatchingContext {
    pub agent: Agent,
    pub session: Session,
    pub interaction: Interaction,
    pub guidelines: Vec<Guideline>,
    pub glossary_terms: Vec<Term>,
    pub nlp: Arc<dyn NlpService>,
}

// crates/loon-engine/src/guideline_matching/llm_matcher.rs
pub struct LlmGuidelineMatcher {
    nlp: Arc<dyn NlpService>,
    resolver: GenericGuidelineMatchingStrategyResolver,
}

#[async_trait]
impl GuidelineMatcher for LlmGuidelineMatcher {
    async fn match_guidelines(&self, ctx: &GuidelineMatchingContext) -> Result<Vec<GuidelineMatch>> {
        // 1. 将 guideline 按 strategy 分组
        // 2. 对每组调 SchematicGenerator 生成匹配结果
        // 3. 合并所有匹配
    }
}

// crates/loon-engine/src/guideline_matching/custom_strategy.rs
/// 自定义匹配策略——允许用户替换默认 LLM 匹配逻辑
#[async_trait]
pub trait CustomGuidelineMatchingStrategy: Send + Sync {
    async fn match_guidelines(
        &self, guidelines: &[Guideline], ctx: &GuidelineMatchingContext,
    ) -> Result<Vec<GuidelineMatch>>;
}
```

### 8.6 Tool 调用子系统（`tool_calling/`）

> 对应 Parlant: `src/parlant/core/engines/alpha/tool_calling/` (4 个文件, ~163 KB)

```rust
// crates/loon-engine/src/tool_calling/caller.rs

pub enum ToolCallEvaluation { NeedsToRun, DataAlreadyInContext, Skipped }
pub struct ToolInsights { pub evaluations: HashMap<ToolId, ToolCallEvaluation> }

#[async_trait]
pub trait ToolCaller: Send + Sync {
    /// 生成 tool insights（决定哪些工具需要运行）
    async fn generate_insights(
        &self, ctx: &EngineContext, guidelines: &[GuidelineMatch],
    ) -> Result<ToolInsights>;

    /// 执行工具调用
    async fn call_tools(
        &self, ctx: &EngineContext, insights: &ToolInsights,
    ) -> Result<Vec<ToolExecutionResult>>;
}

// crates/loon-engine/src/tool_calling/batcher.rs
pub struct DefaultToolCallBatcher {
    nlp: Arc<dyn NlpService>,
    tool_service: Arc<dyn ToolService>,
}

// crates/loon-engine/src/tool_calling/single_tool_batch.rs
/// 单工具批次：一个工具一个 LLM 调用来决定参数
pub struct SingleToolBatch { /* ... */ }

// crates/loon-engine/src/tool_calling/overlapping_tools_batch.rs
/// 重叠工具批次：多个重叠工具批量调 LLM 决定参数
pub struct OverlappingToolsBatch { /* ... */ }
```

### 8.7 关系解析器（`RelationalResolver`）

> 对应 Parlant: `src/parlant/core/engines/alpha/relational_resolver.py` (69022 bytes)

```rust
// crates/loon-engine/src/relational_resolver.rs

/// 基于 Relationship 图解析 guideline 的依赖和互斥
pub struct RelationalResolver {
    relationship_store: Arc<dyn RelationshipStore>,
}

impl RelationalResolver {
    /// 给定已匹配的 guideline 集合，移除被排除的 guideline
    pub async fn resolve_exclusions(
        &self, matches: Vec<GuidelineMatch>,
    ) -> Result<Vec<GuidelineMatch>>;

    /// 给定已匹配的 guideline 集合，添加被依赖的 guideline
    pub async fn resolve_dependencies(
        &self, matches: Vec<GuidelineMatch>, all_guidelines: &[Guideline],
    ) -> Result<Vec<GuidelineMatch>>;

    /// 综合解析：先排除、再补依赖
    pub async fn resolve(
        &self, matches: Vec<GuidelineMatch>, all_guidelines: &[Guideline],
    ) -> Result<Vec<GuidelineMatch>>;
}
```

### 8.8 Prompt 构建器（`PromptBuilder`）

> 对应 Parlant: `src/parlant/core/engines/alpha/prompt_builder.py` (27510 bytes)

```rust
// crates/loon-engine/src/prompt_builder.rs

pub struct PromptBuilder {
    tokenizer: Arc<dyn Tokenizer>,
    max_tokens: usize,
}

impl PromptBuilder {
    /// 组装 agent 的最终 LLM prompt
    pub async fn build_prompt(
        &self,
        agent: &Agent,
        interaction: &Interaction,
        matched_guidelines: &[GuidelineMatch],
        glossary_terms: &[Term],
        context_variables: &[(ContextVariable, ContextVariableValue)],
        tool_results: &[ToolExecutionResult],
        journey_state: Option<&JourneyState>,
        capabilities: &[Capability],
        canned_responses: &[CannedResponse],
    ) -> Result<String>;

    /// 组装 guideline 匹配 prompt（用于 LlmGuidelineMatcher）
    pub async fn build_guideline_matching_prompt(
        &self,
        guidelines: &[Guideline],
        interaction: &Interaction,
        glossary_terms: &[Term],
    ) -> Result<String>;
}
```

### 8.9 Canned Response 生成器（`CannedResponseGenerator`）

> 对应 Parlant: `src/parlant/core/engines/alpha/canned_response_generator.py` (133705 bytes)

```rust
// crates/loon-engine/src/canned_response_generator.rs

pub struct CannedResponseSelection {
    pub canned_response: CannedResponse,
    pub score: f32,
    pub filled_fields: HashMap<String, String>,
}

pub struct CannedResponseGenerator {
    nlp: Arc<dyn NlpService>,
}

impl CannedResponseGenerator {
    /// 从候选罐头回复中选出最佳匹配
    pub async fn select_best(
        &self,
        canned_responses: &[CannedResponse],
        draft_message: &str,
        agent: &Agent,
        interaction: &Interaction,
    ) -> Result<Option<CannedResponseSelection>>;

    /// 用 tool 结果填充罐头回复模板中的 {field} 占位符
    pub fn fill_template(
        template: &str, fields: &HashMap<String, String>,
    ) -> String;
}
```

### 8.10 Message 生成器（`MessageGenerator`）

> 对应 Parlant: `src/parlant/core/engines/alpha/message_generator.py` (76104 bytes)

```rust
// crates/loon-engine/src/message_generator.rs

pub struct MessageGenerator {
    nlp: Arc<dyn NlpService>,
    prompt_builder: Arc<PromptBuilder>,
    canned_response_generator: Arc<CannedResponseGenerator>,
}

impl MessageGenerator {
    /// 流体模式：直接生成消息
    pub async fn generate_fluid_message(
        &self, ctx: &EngineContext,
    ) -> Result<Vec<MessageEventData>>;

    /// 严格模式：生成 draft → 匹配罐头 → 返回罐头或降级
    pub async fn generate_strict_message(
        &self, ctx: &EngineContext, canned_responses: &[CannedResponse],
    ) -> Result<Vec<MessageEventData>>;

    /// 流式生成（用于 WS 推送 delta）
    pub async fn generate_streaming(
        &self, ctx: &EngineContext,
    ) -> Result<Box<dyn Stream<Item = Result<String>>>>;
}
```

### 8.11 Tool Event 生成器 & Message Event 组合器

```rust
// crates/loon-engine/src/tool_event_generator.rs
pub struct ToolEventGenerator;
impl ToolEventGenerator {
    pub fn generate_tool_event_data(
        tool_calls: &[ToolCallData],
    ) -> ToolEventData;
}

// crates/loon-engine/src/message_event_composer.rs
pub struct MessageEventComposer;
impl MessageEventComposer {
    pub fn compose_message_event(
        message: MessageEventData, agent: &Agent,
    ) -> EmittedEvent;
}
```

### 8.12 Planner + 优化策略

```rust
// crates/loon-engine/src/planner.rs
#[async_trait]
pub trait Planner: Send + Sync {
    async fn plan(&self, ctx: &EngineContext) -> Result<Plan>;
}
pub struct NoopPlanner;
impl Planner for NoopPlanner { /* 直接返回 Plan::Done */ }

// crates/loon-engine/src/optimization_policy.rs
pub trait OptimizationPolicy: Send + Sync {
    fn should_skip_tool(&self, tool_id: &ToolId, ctx: &EngineContext) -> bool;
    fn should_skip_guideline_matching(&self, ctx: &EngineContext) -> bool;
}

// crates/loon-engine/src/perceived_performance_policy.rs
pub struct PerceivedPerformancePolicy { /* 感知性能优化 */ }
```

### 8.13 AlphaEngine 主类

```rust
// crates/loon-engine/src/alpha_engine.rs

pub struct AlphaEngine {
    queries: Arc<EntityQueries>,
    commands: Arc<EntityCommands>,
    matcher: Arc<dyn GuidelineMatcher>,
    tool_caller: Arc<dyn ToolCaller>,
    planner: Arc<dyn Planner>,
    message_generator: Arc<MessageGenerator>,
    relational_resolver: Arc<RelationalResolver>,
    hooks: EngineHooks,
    optimization_policy: Arc<dyn OptimizationPolicy>,
    performance_policy: Arc<PerceivedPerformancePolicy>,
    session_store: Arc<dyn SessionStore>,
    nlp: Arc<dyn NlpService>,
}

#[async_trait]
impl Engine for AlphaEngine {
    async fn process(
        &self, context: &Context, event_emitter: &dyn EventEmitter,
    ) -> Result<bool> {
        // === 完整流水线（对齐 parlcant AlphaEngine.process） ===
        // 0. 加载实体 → 构建 EngineContext
        // 1. 发射 STATUS ack 事件（on_acknowledging → emit → on_acknowledged）
        // 2. 生成 preamble（可选）
        // 3. 装填上下文
        //    a. find_guidelines_for_context
        //    b. find_context_variables_for_context
        //    c. find_journeys_for_context → sort + 选 top-N
        //    d. find_canned_responses_for_context
        //    e. find_glossary_terms_for_context
        //    f. find_capabilities_for_agent
        //    g. journey → guideline 投影
        // 4. 准备循环（preparation iterations）：
        //    while !prepared_to_respond:
        //        a. GuidelineMatcher.match_guidelines
        //        b. RelationalResolver.resolve (exclusions + dependencies)
        //        c. ToolCaller.generate_insights → call_tools
        //        d. 若有 tool 被执行 → 重新装填上下文 → 重新匹配 → 继续循环
        //        e. 若无 tool 被执行 → prepared_to_respond = true
        // 5. 消息生成
        //    a. Fluid mode → MessageGenerator.generate_fluid_message
        //    b. Strict mode → MessageGenerator.generate_strict_message
        // 6. 发射消息事件（on_message_generated → emit → on_messages_emitted）
        // 7. 持久化所有事件
        // 8. 返回 true
    }

    async fn utter(&self, context: &Context, event_emitter: &dyn EventEmitter, requests: &[UtteranceRequest]) -> Result<bool> {
        // 主动发话：轻量版 process，跳过用户消息处理
    }
}
```

---

## 9. 索引服务（`loon-engine` 内 `indexing/` 子模块）

> 对应 Parlant: `src/parlant/core/services/indexing/` (10 个文件, ~121 KB)

Phase 1 仅声明 trait，默认实现为 noop/stub。

```rust
// crates/loon-engine/src/indexing/mod.rs
pub mod indexer;
pub mod behavioral_change_evaluation;
pub mod guideline_action_proposer;
pub mod guideline_agent_intention_proposer;
pub mod guideline_continuous_proposer;
pub mod journey_reachable_nodes_evaluation;
pub mod relative_action_proposer;
pub mod tool_running_action_detector;
pub mod customer_dependent_action_detector;
pub mod common;
```

| 组件 | 作用 | Phase 1 默认 |
|------|------|-------------|
| `Indexer` | 为 guideline 建立语义索引加速匹配 | Noop（直接用 LLM 匹配） |
| `BehavioralChangeEvaluation` | 评估 guideline 变更的行为影响 | Noop |
| `GuidelineActionProposer` | 基于 action 相似度推荐 guideline | Noop |
| `GuidelineAgentIntentionProposer` | 基于 agent 意图推荐 guideline | Noop |
| `GuidelineContinuousProposer` | 持续推荐上下文相关的 guideline | Noop |
| `JourneyReachableNodesEvaluation` | 评估 journey 状态的可达性 | BFS 可达性分析（不调 LLM） |
| `RelativeActionProposer` | 基于相对关系推荐 action | Noop |
| `ToolRunningActionDetector` | 检测 tool 运行触发的 action | Noop |
| `CustomerDependentActionDetector` | 检测依赖客户状态的 action | Noop |

---

## 10. SDK（`loon-sdk`）

> 对应 Parlant: `src/parlant/sdk.py` (5918 行)

### 10.1 公开 API

```rust
use loon_sdk as p;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = p::Server::builder()
        .with_document_db(JsonFileDocumentDatabase::new("./data")?)
        .with_nlp_service(OpenAiProvider::new(NlpConfig::from_env()?))
        .build()
        .await?;

    server.run(|server| async move {
        // 创建 agent
        let agent = server.create_agent(p::AgentCreateParams {
            name: "Customer Support".into(),
            description: "Handles customer inquiries for an airline".into(),
            composition_mode: Some(p::CompositionMode::Fluid),
            ..Default::default()
        }).await?;

        // 创建 observation（条件 + 工具绑定）
        let expert = agent.create_observation(p::ObservationCreateParams {
            condition: "customer uses financial terminology like DTI or amortization".into(),
            tools: vec!["research_deep_answer".into()],
            ..Default::default()
        }).await?;

        // 创建 guideline（依赖 observation）
        agent.create_guideline(p::GuidelineCreateParams {
            matcher: p::GuidelineMatcher::Always,
            action: "respond with technical depth — skip basic explanations".into(),
            dependencies: vec![expert.id().clone()],
            ..Default::default()
        }).await?;

        // 创建互斥 guideline
        let beginner = agent.create_guideline(p::GuidelineCreateParams {
            condition: "customer seems new to the topic".into(),
            action: "simplify and use concrete examples".into(),
            ..Default::default()
        }).await?;

        beginner.exclude(&expert).await?;

        // 创建 journey
        let journey = agent.create_journey(p::JourneyCreateParams {
            title: "Book Flight".into(),
            description: "Guide the customer through flight booking".into(),
            conditions: vec!["customer wants to book a flight".into()],
            ..Default::default()
        }).await?;

        Ok(())
    }).await?;

    Ok(())
}
```

### 10.2 公开类型清单

与 parlcant SDK 对齐：

| Rust 类型 | Parlant 来源 | 说明 |
|-----------|-------------|------|
| `Server` / `ServerBuilder` | `sdk.Server` | 服务器生命周期 |
| `Agent` | `sdk.Agent` | agent 句柄 |
| `Guideline` | `sdk.Guideline` | guideline 句柄 |
| `GuidelineMatch` | `sdk.GuidelineMatch` | 匹配结果 |
| `GuidelineMatchingContext` | `sdk.GuidelineMatchingContext` | 匹配上下文 |
| `GuidelineMatcher` (enum) | `sdk.MATCH_ALWAYS` 等 | `Always` / `IfConditionMatches` |
| `Observation` | agent.create_observation 返回 | observation 句柄 |
| `Journey` / `JourneyState` / `JourneyTransition` | `sdk.Journey` 等 | journey 状态机 |
| `Tool` / `ToolCall` / `ToolContext` | `sdk.Tool` / `sdk.ToolContext` | 工具定义与调用 |
| `Capability` / `Retriever` | `sdk.Capability` / `sdk.Retriever` | 检索能力 |
| `Term` (Glossary) | `sdk.Term` | 术语 |
| `Variable` | `sdk.Variable` | 变量 |
| `ContextVariable` | `sdk.ContextVariable` | 上下文变量 |
| `Customer` | `sdk.Customer` | 客户 |
| `Session` / `SessionMode` / `SessionMetadata` / `SessionLabels` | `sdk.Session` 等 | 会话 |
| `Tag` / `AnyOf` / `AllOf` | `sdk.Tag` / `sdk.AnyOf` / `sdk.AllOf` | 标签与组合匹配 |
| `Relationship` | `sdk.Relationship` | 关系 |
| `CannedResponse` | `sdk.CannedResponse` | 罐头回复 |
| `CompositionMode` | `sdk.CompositionMode` | 组成模式 |
| `MessageOutputMode` | `sdk.MessageOutputMode` | 输出模式 |

### 10.3 SDK 错误

```rust
#[derive(Debug, thiserror::Error)]
pub enum SdkError {
    #[error("agent not found: {0}")]          AgentNotFound(AgentId),
    #[error("guideline not found: {0}")]      GuidelineNotFound(GuidelineId),
    #[error("journey not found: {0}")]        JourneyNotFound(JourneyId),
    #[error("session not found: {0}")]        SessionNotFound(SessionId),
    #[error("tool not found: {0}")]           ToolNotFound(ToolId),
    #[error("customer not found: {0}")]       CustomerNotFound(CustomerId),
    #[error("NLP error: {0}")]                Nlp(#[from] NlpError),
    #[error("persistence error: {0}")]        Persistence(#[from] PersistenceError),
    #[error("engine error: {0}")]             Engine(#[from] EngineError),
    #[error("emission error: {0}")]           Emission(#[from] EmissionError),
    #[error("validation error: {0}")]         Validation(String),
    #[error(transparent)]                     Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}
```

---

## 11. HTTP/WS 服务（`loon-server`）

与初版设计保持一致，补充以下端点：

### 11.1 完整端点表

| Method | Path | 说明 |
|--------|------|------|
| GET | `/health` | 健康检查 |
| GET | `/version` | 服务版本 |
| GET | `/v1/agents` | 列出 agents |
| POST | `/v1/agents` | 创建 agent |
| GET | `/v1/agents/{id}` | 读 agent |
| PATCH | `/v1/agents/{id}` | 更新 agent |
| DELETE | `/v1/agents/{id}` | 删除 agent |
| GET | `/v1/agents/{id}/guidelines` | 列出 guidelines |
| POST | `/v1/agents/{id}/guidelines` | 创建 guideline |
| PATCH | `/v1/agents/{id}/guidelines/{gid}` | 更新 guideline |
| DELETE | `/v1/agents/{id}/guidelines/{gid}` | 删除 guideline |
| GET | `/v1/agents/{id}/journeys` | 列出 journeys |
| POST | `/v1/agents/{id}/journeys` | 创建 journey |
| GET | `/v1/agents/{id}/journeys/{jid}` | 读 journey |
| PATCH | `/v1/agents/{id}/journeys/{jid}` | 更新 journey |
| DELETE | `/v1/agents/{id}/journeys/{jid}` | 删除 journey |
| GET | `/v1/agents/{id}/journeys/{jid}/states` | 列出 journey states |
| GET | `/v1/agents/{id}/tools` | 列出 tools |
| POST | `/v1/agents/{id}/tools` | 创建 tool |
| DELETE | `/v1/agents/{id}/tools/{tid}` | 删除 tool |
| GET | `/v1/agents/{id}/observations` | 列出 observations |
| POST | `/v1/agents/{id}/observations` | 创建 observation |
| DELETE | `/v1/agents/{id}/observations/{oid}` | 删除 observation |
| GET | `/v1/agents/{id}/canned_responses` | 列出罐头回复 |
| POST | `/v1/agents/{id}/canned_responses` | 创建罐头回复 |
| GET | `/v1/agents/{id}/glossary` | 列出术语 |
| POST | `/v1/agents/{id}/glossary` | 创建术语 |
| GET | `/v1/sessions` | 列出会话 |
| POST | `/v1/sessions` | 创建会话 |
| GET | `/v1/sessions/{id}` | 读会话 |
| POST | `/v1/sessions/{id}/events` | 推送消息事件 |
| GET | `/v1/sessions/{id}/events` | 流式拉取事件 |
| WS | `/v1/sessions/{id}/chat` | WS 双向聊天 |
| GET | `/v1/customers` | 列出客户 |
| POST | `/v1/customers` | 创建客户 |
| GET | `/v1/tags` | 列出标签 |
| GET | `/v1/relationships` | 列出关系 |
| POST | `/v1/relationships` | 创建关系 |
| DELETE | `/v1/relationships/{id}` | 删除关系 |

### 11.2 WS 聊天协议

```json
// Client → Server
{"type": "user_message", "content": "I need to book a flight"}

// Server → Client (流式 delta)
{"type": "agent_message", "delta": "Sure! Let me"}
{"type": "agent_message", "delta": " help you find"}

// Server → Client (tool call)
{"type": "tool_call", "tool_id": "...", "arguments": {...}}

// Server → Client (tool result)
{"type": "tool_result", "tool_id": "...", "result": {...}}

// Server → Client (done)
{"type": "done"}
```

---

## 12. CLI 客户端（`loon` 二进制）

与初版设计一致。

---

## 13. API 公共模式（`api/common.rs`）

> 对应 Parlant: `src/parlant/api/common.py` (17153 bytes)

### 13.1 统一 API 错误

```rust
// crates/loon-server/src/api/common.rs

#[derive(Debug, Serialize)]
pub struct ApiErrorBody {
    pub error: String,
    pub detail: Option<String>,
    pub code: String,               // e.g. "AGENT_NOT_FOUND"
}

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("not found: {0}")]
    NotFound(String, String),       // (message, code)
    #[error("invalid argument: {0}")]
    InvalidArgument(String, String),
    #[error("conflict: {0}")]
    Conflict(String, String),
    #[error("rate limited")]
    RateLimited(String),
    #[error("upstream error: {0}")]
    Upstream(String),
    #[error("internal error: {0}")]
    Internal(String),
}

impl ApiError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::NotFound(_, _) => StatusCode::NOT_FOUND,          // 404
            Self::InvalidArgument(_, _) => StatusCode::BAD_REQUEST,  // 400
            Self::Conflict(_, _) => StatusCode::CONFLICT,            // 409
            Self::RateLimited(_) => StatusCode::TOO_MANY_REQUESTS,   // 429
            Self::Upstream(_) => StatusCode::BAD_GATEWAY,            // 502
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,  // 500
        }
    }
}

impl axum::response::IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let body = ApiErrorBody {
            error: self.to_string(),
            detail: None,
            code: match &self {
                Self::NotFound(_, c) | Self::InvalidArgument(_, c) | Self::Conflict(_, c) => c.clone(),
                Self::RateLimited(_) => "RATE_LIMITED".into(),
                Self::Upstream(_) => "UPSTREAM_ERROR".into(),
                Self::Internal(_) => "INTERNAL".into(),
            },
        };
        (self.status_code(), Json(body)).into_response()
    }
}
```

### 13.2 统一 API 响应包装

```rust
/// 泛型 API 响应包装
#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub data: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<ApiMeta>,
}

#[derive(Debug, Serialize)]
pub struct ApiMeta {
    pub total: Option<usize>,
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}

/// 列表响应
#[derive(Debug, Serialize)]
pub struct ApiListResponse<T: Serialize> {
    pub items: Vec<T>,
    pub total: usize,
}

impl<T: Serialize> From<PaginatedResult<T>> for ApiListResponse<T> {
    fn from(p: PaginatedResult<T>) -> Self {
        Self { items: p.items, total: p.total }
    }
}
```

### 13.3 请求 DTO 模式

```rust
/// 所有创建请求的 DTO 都应实现 Into<对应 CreateParams>
/// 例如：
#[derive(Debug, Deserialize)]
pub struct CreateAgentRequest {
    pub name: String,
    pub description: Option<String>,
    pub composition_mode: Option<CompositionMode>,
    pub message_output_mode: Option<MessageOutputMode>,
    pub tags: Option<Vec<TagId>>,
}

impl From<CreateAgentRequest> for AgentCreateParams {
    fn from(r: CreateAgentRequest) -> Self {
        AgentCreateParams {
            name: r.name,
            description: r.description.unwrap_or_default(),
            composition_mode: r.composition_mode.unwrap_or(CompositionMode::Fluid),
            message_output_mode: r.message_output_mode.unwrap_or(MessageOutputMode::Fluid),
            tags: r.tags.unwrap_or_default(),
        }
    }
}
```

---

## 13. 健康检查（`loon-engine` 内 `health/` 子模块）

> 对应 Parlant: `src/parlant/core/health/` (4 个文件)

```rust
// crates/loon-engine/src/health/mod.rs
pub struct HealthReporter {
    engine: Arc<dyn Engine>,
    nlp: Arc<dyn NlpService>,
}

impl HealthReporter {
    pub async fn check(&self) -> HealthStatus;
    pub async fn engine_view(&self) -> EngineHealthView;
    pub async fn nlp_view(&self) -> NlpHealthView;
    pub async fn event_loop_view(&self) -> EventLoopHealthView;
}
```

---

## 14. 错误处理

与初版设计一致，补充：

```rust
// 每个 crate 定义自己的 Error enum
// loon-core:
pub enum CoreError { NotFound(UniqueId), InvalidArgument(String), Conflict(String), Internal(String) }

// loon-emission:
pub enum EmissionError { EmitterNotFound, PersistenceFailed(anyhow::Error) }

// loon-engine:
pub enum EngineError {
    ContextLoadFailed(anyhow::Error),
    GuidelineMatchingFailed(anyhow::Error),
    ToolCallFailed(ToolId, anyhow::Error),
    MessageGenerationFailed(anyhow::Error),
    HookBail,
}
```

HTTP 状态码映射不变。

---

## 15. 测试策略

在原基础上扩展：

### 15.1 单元测试

- `loon-core`：实体构造、ID 确定性、JSON 序列化往返、EntityQueries mock 测试、JourneyGuidelineProjection 单元测试
- `loon-emission`：EventBuffer 收集 + 顺序验证、EventPublisher 持久化验证
- `loon-app-modules`：每个模块的 CRUD + 跨实体业务逻辑
- `loon-persistence`：JSON 后端 CRUD、filter 表达式、并发读写、原子 rename
- `loon-nlp`：OpenAI mock（`wiremock`）、retry 策略、SSE 解析、schema 校验、错误回退
- `loon-engine`：
  - `LlmGuidelineMatcher`：带 mock NLP 的匹配测试
  - `ToolCaller`：fake tool 实现的调用链测试
  - `RelationalResolver`：dependency graph 的 exclusion/dependency 传播
  - `PromptBuilder`：token 预算管理测试
  - `EngineHooks`：钩子链的 CALL_NEXT/RESOLVE/BAIL 行为
  - `AlphaEngine`：用 fake 实现替换所有 strategy 的编排测试

### 15.2 集成测试

`tests/e2e_agent_loop.rs`：
- 启动 in-memory JSON DB + mock NLP
- 创建 agent + guideline + observation + tool (stub)
- 多轮对话测试：
  - guideline 命中 / 不命中
  - tool 触发 / 跳过
  - strict mode → canned response 选择
  - journey 状态推进
  - event 正确持久化 + 可回放

### 15.3 文档测试 + 覆盖率

不变。

---

## 16. CI 与质量门

与初版设计一致。

---

## 17. 文档结构

不变。

---

## 18. 风险与缓解（修订）

| 风险 | 影响 | 缓解 |
|------|------|------|
| EntityCQ 引入增加复杂度 | 中 | 严格按 parlcant EntityQueries/EntityCommands 接口 1:1 映射；不自行扩展 |
| 引擎组件多（20+ 文件），集成调试困难 | 高 | 每个 strategy 可注入 fake 实现；EngineContext 透明化；Phase 1 先写所有 trait + 骨架实现 |
| PromptBuilder 质量影响引擎输出 | 高 | 用 parlcant 的 prompt 模板直接翻译；不自行优化 |
| CannedResponseGenerator 依赖 LLM 匹配 | 中 | 对 parlcant 的 schema 做 1:1 翻译 |
| 索引服务的 noop 实现可能导致后续 phase 大改 API | 低 | trait 先与 parlcant 接口对齐；实现替换不影响调用方 |

---

## 19. 待用户确认的关键点

- [x] Phase 1+2 合并（含 axum HTTP/WS）
- [x] 持久化：文档抽象 + JSON 文件后端
- [x] NLP：自研 trait + reqwest，仅 OpenAI
- [x] 工程结构：Cargo workspace，新增 `loon-emission` / `loon-app-modules` crate
- [x] DI：手工（无框架）
- [x] 引擎编排：策略模式 + 完整子组件
- [ ] EntityCQ 作为独立 `loon-core` 子模块还是独立 crate？（建议：`loon-core` 子模块）
- [ ] 索引服务是否应在 Phase 1 就有 trait 声明？（建议：是，避免后续 API 断裂）

---

## 20. 后续 phase 简表

- **Phase 3**：Anthropic + Gemini provider
- **Phase 4**：MongoDB 后端
- **Phase 5**：Chroma / Qdrant 向量后端
- **Phase 6**：MCP 服务
- **Phase 7**：OpenAPI 服务
- **Phase 8**：Plugin 系统
- **Phase 9**：文档迁移系统
- **Phase 10**：OpenTelemetry / 限速 / 鉴权 / 完整 indexing
- **Phase 11**：TS 前端（独立仓库 `loon-chat-ui`）

---

## 21. Rust crate 依赖（`Cargo.toml`）

```toml
[workspace]
members = [
    "crates/loon-core",
    "crates/loon-emission",
    "crates/loon-app-modules",
    "crates/loon-persistence",
    "crates/loon-nlp",
    "crates/loon-engine",
    "crates/loon-sdk",
    "crates/loon-server",
    "crates/loon",
]

[workspace.dependencies]
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
anyhow = "1"
tracing = "0.1"
uuid = { version = "1", features = ["v4"] }
nanoid = "0.4"
chrono = { version = "0.4", features = ["serde"] }
reqwest = { version = "0.12", features = ["json", "stream"] }
futures = "0.3"
tokio-stream = "0.1"
axum = { version = "0.8", features = ["ws"] }
tower = "0.5"
tower-http = { version = "0.6", features = ["cors", "trace"] }
clap = { version = "4", features = ["derive"] }
dialoguer = "0.11"
console = "0.15"
toml = "0.8"
xxhash-rust = { version = "0.8", features = ["xxh3"] }
jsonpath-rust = "0.7"
jsonschema = "0.18"

[workspace.lints.rust]
unsafe_code = "deny"
```

---

## 22. 与初版的主要差异总结

| 维度 | 初版 v1 | 修订版 v2 |
|------|---------|----------|
| Crate 数量 | 7 | 9 (新增 `loon-emission` + `loon-app-modules`) |
| 实体 Store | 15 个 Store trait | 15 个 Store trait + EntityCQ 模式 |
| 事件系统 | 无（仅在 Session 中有 events 字段） | 完整的 EventEmitter/EventBuffer/EventPublisher 三层 |
| AppModules | 无（合并到服务层） | 13 个模块，独立 crate |
| 引擎子组件 | 4 个 strategy trait + 简单实现 | 17 个组件/模块的 trait + 骨架设计 |
| Guideline 匹配 | 单个 `LlmGuidelineMatcher` | 策略模式 + CustomStrategy + Resolver |
| Tool 调用 | 简单并发调用 | 完整 batcher 系统（Single + Overlapping） |
| 关系解析 | 仅在实体表中提及 | 独立的 `RelationalResolver` |
| Prompt 构建 | 仅提模块名 | 完整 `PromptBuilder` 接口 |
| Canned Response | 仅在实体表中提及 | 完整的 `CannedResponseGenerator` 接口 |
| 索引服务 | 无 | 9 个 trait 声明（Phase 1 noop） |
| 钩子系统 | 无 | 15 个钩子点的 `EngineHooks` |
| Health | 无 | `HealthReporter` + 3 个 view |
| SDK | ~80 行伪代码 | 对齐 5918 行 `sdk.py` 的完整类型清单 + API 示例 |
| API 端点 | ~25 个 | ~42 个 (含 journey states / canned_responses / relationships) |
| ServiceRegistry | 无 | `ServiceRegistry` + `ToolService` trait (Phase 1 InMemory 实现) |
| Shots | 无 | `Shot` 实体 + `ShotStore` |
| Logger / Tracer | 仅 EngineContext 字段名 | 完整 trait 定义 + Phase 1 简单实现 |
| DataCollection | 无 | 分页/排序辅助方法 |
| API 公共模式 | 无 | `ApiError` + `ApiResponse<T>` + 请求 DTO |

---

## 23. 参考

- <https://github.com/emcie-co/parlant> (develop 分支)
- `docs/reference/parlant-overview.md`
- Parlant 核心源文件索引（按审查顺序）：
  - `src/parlant/core/common.py` (5848 bytes)
  - `src/parlant/core/entity_cq.py` (19312 bytes)
  - `src/parlant/core/emissions.py` (3038 bytes)
  - `src/parlant/core/emission/event_buffer.py` (4986 bytes)
  - `src/parlant/core/emission/event_publisher.py` (5891 bytes)
  - `src/parlant/core/journey_guideline_projection.py` (5257 bytes)
  - `src/parlant/core/guideline_tool_associations.py` (6705 bytes)
  - `src/parlant/core/engines/types.py` (1474 bytes)
  - `src/parlant/core/engines/alpha/engine.py` (94188 bytes)
  - `src/parlant/core/engines/alpha/engine_context.py` (8510 bytes)
  - `src/parlant/core/engines/alpha/entity_context.py` (4035 bytes)
  - `src/parlant/core/engines/alpha/hooks.py` (7542 bytes)
  - `src/parlant/core/engines/alpha/relational_resolver.py` (69022 bytes)
  - `src/parlant/core/engines/alpha/prompt_builder.py` (27510 bytes)
  - `src/parlant/core/engines/alpha/canned_response_generator.py` (133705 bytes)
  - `src/parlant/core/engines/alpha/message_generator.py` (76104 bytes)
  - `src/parlant/core/engines/alpha/tool_event_generator.py` (11286 bytes)
  - `src/parlant/core/engines/alpha/tool_calling/` (4 files, ~163 KB)
  - `src/parlant/core/engines/alpha/guideline_matching/` (7 files)
  - `src/parlant/core/services/indexing/` (10 files, ~121 KB)
  - `src/parlant/core/health/` (4 files)
  - `src/parlant/core/app_modules/` (13 files, ~78 KB)
  - `src/parlant/sdk.py` (5918 行)

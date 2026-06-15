# Parlant 项目调研笔记

参考仓库：<https://github.com/emcie-co/parlant> （Apache-2.0，Python 3.10+，~18k stars）

本文档是 loon（Rust 复刻）实现过程中的参考材料。**所有事实均来自 GitHub 上 emcie-co/parlant 仓库的源码与 README**，不包含本文档作者的发挥。

---

## 1. Parlant 是什么

> Build reliable customer-facing AI agents with Parlant: an interaction control harness optimized for controlled, consistent, and predictable LLM interactions.

针对客服/售前/合规等 B2C、B2B 场景的对话式 AI 框架。它的核心思路不是写 system prompt、也不是堆路由图，而是在每一轮动态地把**当时真正相关的指令、知识、工具**注入上下文，从而让模型在多规则场景下保持一致与可解释。

定位区别：
- 与 LangGraph：LangGraph 偏工作流自动化，Parlant 偏对话治理与一致性。
- 与 DSPy：DSPy 偏底层 prompt 优化，Parlant 偏高层的对话建模。

---

## 2. 核心域概念

来自 `src/parlant/core/` 下各实体文件，主要类型如下（与 SDK 直接对应）：

| 概念 | 文件 | 一句话作用 |
|------|------|------------|
| `Agent` | `core/agents.py` | 一个独立的 AI agent，有名字、描述、组成模式、最大输出模式 |
| `Guideline` | `core/guidelines.py` | 带 `condition` 的指令；当 condition 命中时模型应执行对应 `action` |
| `Observation` | `core/evaluations.py` | 比 Guideline 更强的前置条件，可挂 `tools` 列表 |
| `Journey` | `core/journeys.py` | 多步 SOP / 流程图，由状态和转移构成 |
| `JourneyState` / `JourneyTransition` | 同上 | 状态机的节点与边，支持 `Initial`/`Tool`/`Chat`/`Fork` |
| `Tool` | `core/tools.py` | agent 可调用的能力（本地函数 / OpenAPI / MCP） |
| `ToolCall` / `ToolResult` | 同上 | 工具调用与其结果 |
| `Retriever` | `core/capabilities.py` | 把外部知识（向量库 / 文档）按条件拉入上下文 |
| `Glossary` | `core/glossary.py` | 领域术语表，注入到上下文帮助模型对齐语义 |
| `Variable` / `ContextVariable` | `core/context_variables.py` | agent / 会话级记忆与键值变量 |
| `Customer` | `core/customers.py` | 对话主体，带 `metadata` 与标签 |
| `Session` | `core/sessions.py` | 一次会话，含消息事件流 |
| `Tag` | `core/tags.py` | 给任意实体打的标签，用于检索和分类 |
| `Relationship` | `core/relationships.py` | 实体间关系（如 guideline 之间的互斥 / 依赖） |
| `CannedResponse` | `core/canned_responses.py` | 预置回复，可在严格输出模式下取代自由生成 |
| `Capability` | `core/capabilities.py` | retriever、罐头回复等的统一抽象 |
| `CompositionMode` / `MessageOutputMode` | `core/agents.py` | 自由 / 严格；流体 / 罐头 |

所有这些实体在 SDK 中均有同名类型（`Agent` / `Guideline` / `Journey` / `Tool` / `Capability` / `Tag` / `Term` / `Variable` / `Customer` / `Session` 等）。

---

## 3. 引擎流水线（AlphaEngine）

来源：`src/parlant/core/engines/alpha/engine.py`（约 2284 行）

`AlphaEngine` 是 Parlant 默认的推理引擎，单轮推理分四阶段：

```
User Input
   │
   ▼
[1] Match Guidelines & Resolve Journey States
   │   ← 从所有 Guideline/Observation/Journey 中筛出本轮相关的子集
   ▼
[2] Call Contextually-Associated Tools & Workflows
   │   ← 只调用本轮相关的工具，按批次（default_tool_call_batcher）执行
   │   ← MCP / OpenAPI / 本地工具均在这里接入
   ▼
[3] Compose Output
   ├── Fluid Output Mode  → 自由生成（StreamingTextGenerator + SchematicGenerator）
   └── Strict Output Mode → 罐头回复命中则直接返回，否则降级到生成
   ▼
Message Event
```

辅助子系统：
- `guideline_matching/`：guideline 的语义匹配（generic + custom strategies）。
- `tool_calling/`：批次执行工具，处理重叠工具。
- `planning/`：单步规划。
- `planners.py`、`optimization_policy.py`、`perceived_performance_policy.py`：调度与体验优化。

---

## 4. NLP 抽象

来源：`src/parlant/core/nlp/`

```text
                ┌─────────────────────────────┐
                │      NLPService (容器)      │
                └─────────────────────────────┘
                            │
        ┌───────────────────┼───────────────────┐
        ▼                   ▼                   ▼
StreamingTextGenerator  SchematicGenerator[T]  Embedder / Tokenizer / Moderation
        │                   │
        ▼                   ▼
   自由文本流          结构化 Pydantic 输出（schema 校验 / 重试 / 回退）
```

关键抽象：
- `StreamingTextGenerator` / `BaseStreamingTextGenerator`：自由文本生成接口。
- `SchematicGenerator[T]` / `BaseSchematicGenerator[T]`：基于 Pydantic schema 的结构化输出。
- `FallbackSchematicGenerator`：多 provider 回退链。
- `Embedding` / `Tokenization` / `Moderation`：embedding、token 计算、敏感词过滤。
- `policies.py`：限速、配额、重试策略。

可插拔 provider（`src/parlant/adapters/nlp/`）：
- OpenAI、Anthropic、AWS Bedrock、Together、Cerebras、DeepSeek、Gemini、Vertex、Ollama、LiteLLM。
- 默认 provider 为 OpenAI。

---

## 5. 持久化抽象

来源：`src/parlant/core/persistence/`

两套抽象：

1. **文档数据库**（`document_database.py`）
   - `DocumentDatabase` / `DocumentCollection[TDocument]` 抽象。
   - 文档为 TypedDict，版本化（`GuidelineDocument_v0_1_0` … `v0_10_0` 共 11 个迁移）。
   - 后端：`MongoDB`（默认）、可扩展其它。

2. **向量数据库**（`vector_database.py`）
   - `VectorDatabase` 抽象。
   - 后端：`Chroma`、`Qdrant`、`MongoDB`（向量能力）。

辅助：迁移工具（`bin/prepare_migration.py`）。

---

## 6. 服务层

来源：`src/parlant/core/services/`

- `service_registry.py`：服务容器（基于 lagom 的依赖注入）。
- `tools/openapi.py`：把 OpenAPI 文档转换为 Tool。
- `tools/mcp_service.py`：接入 MCP（Model Context Protocol）服务器。
- `tools/plugins.py`：插件机制，可在运行时注入自定义 Tool / Guideline / Journey。

---

## 7. API 层

来源：`src/parlant/api/`

- HTTP + WebSocket，基于 FastAPI。
- 模块：`agents.py`、`guidelines.py`、`journeys.py`、`sessions.py`、`customers.py`、`chat/`（含 WS 聊天端点）、`health.py`、`logs.py` 等。
- TS 前端：`api/chat/`（React + Vite + Tailwind），`npm run build` 后由后端挂载。

授权：`authorization.py`（OAuth / API Key）。

---

## 8. SDK

来源：`src/parlant/sdk.py`（5918 行，公开 API）

核心类：

```python
class Server: ...
class Agent: ...
class Guideline: ...
class GuidelineMatch: ...
class GuidelineMatchingContext: ...
class Observation: ...           # via create_observation on Agent
class Journey: ...
class JourneyState / Transition / InitialJourneyState / ToolJourneyState / ChatJourneyState / ForkJourneyState
class Tool / ToolContextAccessor / ToolCall
class Capability / Retriever / Term / Variable
class Customer / Session / SessionMetadata / SessionLabels
class Tag / AnyOf / AllOf / Relationship
class CompositionMode / ExperimentalAgentFeatures
```

典型用法（来自 README）：

```python
import parlant.sdk as p

async with p.Server():
    agent = await server.create_agent(
        name="Customer Support",
        description="Handles customer inquiries for an airline",
    )

    expert_customer = await agent.create_observation(
        condition="customer uses financial terminology like DTI or amortization",
        tools=[research_deep_answer],
    )

    expert_answers = await agent.create_guideline(
        matcher=p.MATCH_ALWAYS,
        action="respond with technical depth",
        dependencies=[expert_customer],
    )

    beginner_answers = await agent.create_guideline(
        condition="customer seems new to the topic",
        action="simplify and use concrete examples",
    )

    await beginner_answers.exclude(expert_customer)
```

CLI 入口：
- `parlant`：客户端 CLI。
- `parlant-server`：启动服务器。
- `parlant-prepare-migration`：文档迁移工具。

---

## 9. 跨切关注点

- **依赖注入**：`lagom`（轻量 DI 容器）。
- **异步**：`asyncio` + `aiofiles`。
- **日志**：`structlog` + `coloredlogs`。
- **限速 / 配额**：`limits` 库。
- **Tracing / Metrics**：`opentelemetry`（API + SDK + OTLP exporter + instrumentation）。
- **Token 计数**：`tiktoken` + `tokenizers`。
- **Schema 校验**：`pydantic` + `jsonschema` + `jsonfinder`（从自由文本中提取 JSON）。
- **WebSocket**：`wsproto` + `websocket-client`。
- **MCP 客户端**：`mcp`、`fastmcp`。
- **OpenAPI**：`aiopenapi3` + `openapi3-parser`。
- **图算法**（journey）：`networkx`。

---

## 10. 代码体量参考

| 文件 | 行数 |
|------|------|
| `src/parlant/sdk.py` | 5918 |
| `src/parlant/core/engines/alpha/engine.py` | 2284 |
| `src/parlant/core/guidelines.py` | 多版本迁移，主体较重 |
| `src/parlant/core/engines/alpha/tool_calling/tool_caller.py` | 重 |
| `src/parlant/core/engines/alpha/guideline_matching/guideline_matcher.py` | 重 |

总体：`src/` 主体 Python 代码约 5.6 MB；测试代码另算。

---

## 11. 对 loon 的启示（仅供后续 spec 参考，非决策）

- 域概念数量 ≈ 15 个，建议每个域都建独立 crate 模块（`loon-core-agent`、`loon-core-guideline` 等）或单 crate 内子模块，按可独立测试与可独立演进原则拆分。
- 引擎流水线四阶段天然适合用 trait 抽象为 4 个 stage，方便单元测试。
- NLP 抽象与持久化抽象从一开始就要写“trait + 多实现”的形态，否则后期重写代价大。
- 文档版本迁移机制是 parlcant 演进的关键支撑，loon 第一版也要预留 `DocumentVersion` 概念。
- 不要试图一开始就把所有 LLM provider 都实现，先 OpenAI 一种把 trait 设计稳定下来。

---

> 本文档为调研参考，最终设计以 `docs/superpowers/specs/` 下的 spec 为准。

# 补强 3 — sdd 收官计划

**基于：** `docs/reviews/2026-06-25-loon-final-review.md`

## 7 个剩余项（按优先级排列）

**批次 A（必须修复）：**
1. **真实 LLM 集成测试** — wiremock mock OpenAI endpoint, 验证 request shape + response parsing
2. **MongoDB / Vector DB e2e** — testcontainers + `docker compose` 集成测试骨架
3. **WS chat 端到端测试** — axum Router + tokio_tungstenite，全链路验证

**批次 B（补强现有）：**
4. **Token 预算截断** — PromptBuilder 当超预算时截断历史的最老消息
5. **LlmGuidelineMatcher built-in always-match** — 当 guideline 的 condition 为 "always" 时返回 confidence 1.0，不调 LLM
6. **ToolContext 传递到 hander** — LocalToolService 接受 ToolContext，tool handler 签名扩展
7. **HealthReporter 动态** — 实时检查 engine + NLP 可接触性
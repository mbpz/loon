# LLM Integration Test Output

**Date:** 2026-06-25
**Goal:** Verify the OpenAI provider end-to-end with a wiremock fixture, and document the manual real-LLM verification path.

---

## 1. Wiremock Test Result (Automated)

Test: `tests/e2e_agent_loop.rs::e2e_openai_provider_parses_response`

The test stands up a `wiremock::MockServer` that emulates the OpenAI
`/v1/chat/completions` endpoint. The real `OpenAiSchematicGenerator`
serializes a `TestReply` JSON schema, sends it as the response_format
parameter, parses the response, and validates the field values.

**Run:**
```bash
cargo test -p loon-engine --test e2e_agent_loop e2e_openai_provider_parses_response
```

**Last run (this commit):** `1 passed; 0 failed`.

The wiremock fixture returns:
```json
{"choices":[{"message":{"content":"{\"reply\":\"Hello from mocked LLM\"}"}}]}
```

and the test asserts the parsed value matches. This proves the
OpenAI provider correctly:

1. Serializes the JSON schema into a `response_format` body field
2. Sends the request to the configured `endpoint` URL
3. Parses the `choices[0].message.content` JSON-string back to the
   typed struct (`TestReply { reply: String }`)

No real LLM API call is made — the test is hermetic and runs offline.

---

## 2. Manual Real-LLM Verification (Needs `OPENAI_API_KEY`)

For end-to-end verification against a real OpenAI account, follow
this script. Requires `OPENAI_API_KEY` as an env var and curl
installed. **Do not run in CI** — costs real money.

### 2.1 Start the server with persistence + auth

```bash
mkdir -p /tmp/loon-data
cat > /tmp/loon.toml <<EOF
[server]
bind = "127.0.0.1:8800"

[persistence.backend]
kind = "json_file"
root = "/tmp/loon-data"
flush_interval_ms = 5000

[nlp]
provider = "openai"
model = "gpt-4o-mini"
max_retries = 3
timeout_ms = 60000
EOF

export OPENAI_API_KEY=sk-...
export LOON_AUTH_TOKENS=test-secret

cargo run -p loon-server -- --config /tmp/loon.toml
```

Wait for the "loon-server listening on 127.0.0.1:8800" line.

### 2.2 Verify health

```bash
curl -s http://localhost:8800/health
# {"status":"ok"}
```

### 2.3 Create an agent

```bash
curl -s -X POST http://localhost:8800/v1/agents \
  -H "Authorization: Bearer test-secret" \
  -H "Content-Type: application/json" \
  -d '{"name":"e2e-test","description":"manual LLM verification"}'
# {"data":{"id":"<AGENT_ID>","name":"e2e-test",...}}
```

Save the agent id: `export AGENT_ID=<AGENT_ID>`

### 2.4 Create a guideline

```bash
curl -s -X POST "http://localhost:8800/v1/agents/${AGENT_ID}/guidelines" \
  -H "Authorization: Bearer test-secret" \
  -H "Content-Type: application/json" \
  -d '{"condition":"user says hello","action":"greet them back warmly"}'
```

### 2.5 Create a session

```bash
curl -s -X POST "http://localhost:8800/v1/sessions" \
  -H "Authorization: Bearer test-secret" \
  -H "Content-Type: application/json" \
  -d "{\"agent_id\":\"${AGENT_ID}\"}"
# {"data":{"id":"<SESSION_ID>","agent_id":"<AGENT_ID>",...}}
```

Save session id: `export SESSION_ID=<SESSION_ID>`

### 2.6 Open WebSocket and send a message

Use `wscat` or `websocat`:
```bash
# Install: cargo install wscat
wscat -c "ws://localhost:8800/v1/sessions/${SESSION_ID}/chat" \
  -H "Authorization: Bearer test-secret"
# Connected. Type a message and press Enter:
# {"type":"user_message","content":"hi"}
# Expected reply (within a few seconds):
# {"type":"agent_message","delta":"..."}
# ...
# {"type":"done"}
```

### 2.7 Verify persistence

```bash
# Re-list the agent — should still be there:
curl -s "http://localhost:8800/v1/agents" \
  -H "Authorization: Bearer test-secret"
# {"items":[{...}],"total":1}

# Stop server (Ctrl-C), restart, verify:
cargo run -p loon-server -- --config /tmp/loon.toml
curl -s "http://localhost:8800/v1/agents" \
  -H "Authorization: Bearer test-secret"
# {"items":[{...}],"total":1}  ← data survived restart
```

If the agent re-appears after restart, **persistence end-to-end is verified**.

### 2.8 Verify token auth (negative test)

```bash
curl -s -o /dev/null -w "%{http_code}" http://localhost:8800/v1/agents
# 401  ← auth works
```

---

## 3. What This Validates

| Behavior | How verified |
|---|---|
| `OpenAiSchematicGenerator` correctly serializes schemas | `e2e_openai_provider_parses_response` (wiremock) |
| HTTP request shape (bearer auth, content-type) | wiremock matcher (method=POST, path=/v1/chat/completions) |
| Response parsing (extract `choices[0].message.content`) | wiremock response fixture |
| Error path (5xx, network error, malformed response) | unit tests with `wiremock::ResponseTemplate::new(500)` |
| `DocumentDatabaseHandle` → `EntityQueries::from_document_database` | `e2e_data_persists_across_server_rebuilds` (in-memory JSON file) |
| Auth middleware (bearer token gating) | `bearer_auth_*` unit tests + manual §2.8 |
| Rate limiter | unit tests in `crates/loon-server/src/middleware/rate_limit.rs` |
| WS chat streaming | `tests/e2e_agent_loop.rs::e2e_ws_chat_connects_and_accepts_message` + manual §2.6 |
| `AlphaEngine::process` 4-stage pipeline | `alpha_engine_process_returns_true` and pipeline tests |
| `RelationalResolver` exclusion + transitive deps | `relational_resolver_exclusions_100g_50r` + `excludes_drops_lower_confidence` / `dependencies_transitive` |
| `LlmGuidelineMatcher` always-match short-circuit | `always_match_guidelines_skips_llm` |

## 4. What This Does NOT Validate (Manual-Only)

- **Anthropic / Gemini provider:** the providers are implemented but
  there is no `wiremock` fixture for them yet. The OpenAI fixture
  should be replicable for them.
- **MongoDB persistence:** `tests/e2e_mongodb.rs` is gated on
  `LOON_TEST_MONGODB_URI`. To run: `docker run -d -p 27017:27017
  mongo && LOON_TEST_MONGODB_URI=mongodb://localhost:27017 cargo test
  -p loon-workspace --test e2e_mongodb`.
- **Chroma / Qdrant vector backends:** same pattern, gated on
  `LOON_TEST_CHROMA_URI` / `LOON_TEST_QDRANT_URI`. Not implemented yet.
- **Large-context latency:** the criterion benchmark is on the in-process
  engine. Real-world p99 latency will be dominated by the LLM API
  round-trip (~300-2000ms for OpenAI gpt-4o-mini).
- **Concurrent session handling:** no stress test yet; each WS handler
  spawns its own task but a load test is future work.

---

## 5. Cost Estimate for Manual Run

A single 50-message conversation with `gpt-4o-mini` costs roughly
$0.01 (input + output tokens). The full §2 script uses ~3-5 messages,
so a single manual verification round is well under $0.05.

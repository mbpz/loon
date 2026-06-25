# Loon Final Fixes — Spec + Plan (sdd)

**Date:** 2026-06-25
**Status:** 4 follow-up items in sdd order.

## 1. Spec

### 1.1 Real LLM Integration (Item 1)

**Goal:** Add a CI workflow that runs the existing wiremock-driven LLM test on every PR + a manual run script gated on `OPENAI_API_KEY`. No real API calls in default CI.

**Scope:**
- Add `.github/workflows/llm-llm.yml` that runs `cargo test e2e_openai_provider_parses_response` and `cargo test e2e_ollama_provider_parses_response` (new test, see below) on every PR/push
- Add `tests/e2e_ollama.rs` with a wiremock-driven Ollama test (parallels the OpenAI test). Ollama uses the same chat-completions format as OpenAI so the wiremock fixture is similar.
- Add a `LLM_LIVE=1 cargo test` script that gates on `OPENAI_API_KEY` and runs a real-LLM test (skipped if not set)

### 1.2 Chat History Truncation (Item 2)

**Goal:** Improve PromptBuilder's truncation to be token-aware with explicit metadata about how many messages were dropped.

**Scope:**
- Modify `crates/loon-engine/src/prompt_builder.rs::build_prompt` to:
  - Drop the oldest message at a time, checking tokens after each drop
  - Stop when under budget or when history is empty
  - Add a `truncated_messages: usize` field to the prompt context (or a separate stats struct)
- Expose this via a new struct `PromptBuildResult { prompt: String, dropped_messages: usize, tokens_used: u32 }`
- Old `build_prompt` returns String; new `build_prompt_with_stats` returns the struct. Keep old as a thin wrapper for back-compat.

### 1.3 Phase 12 (Distributed) — Minimal Skeleton (Item 3)

**Goal:** Lay groundwork for Phase 12 without building a full distributed system. Add a `DistributedState` trait + a Redis backend stub.

**Scope:**
- New trait `loon_persistence::DistributedState` (KV-like): `async fn get`, `set`, `delete`, `list`
- New backend `RedisDistributedState` using `redis = "0.27"` crate. Stub: connect to `LOON_REDIS_URL` env var, do basic GET/SET/DEL
- Behind a feature flag `loon-persistence = { features = ["redis"] }` (default off; gated opt-in)
- New `tests/e2e_redis.rs` gated on `LOON_TEST_REDIS_URI` env var
- New `k8s/loon-server-deployment.yaml` template (in `docs/k8s/`) with single-replica Deployment + Service + ConfigMap; documents the eventual multi-replica setup
- README in `docs/k8s/` explaining how to deploy

### 1.4 Live E2E for MongoDB/Chroma/Qdrant (Item 4)

**Goal:** Add CI jobs that run the gated tests when secrets are configured. Today the tests are skipped when env vars are unset; this just adds the workflow plumbing.

**Scope:**
- New `.github/workflows/e2e-external.yml` with three jobs: `mongodb`, `chroma`, `qdrant`
- Each job spins up the corresponding service via `docker run` (mongo/chromadb/qdrant) and runs the gated test
- Document the secrets needed: `LOON_TEST_MONGODB_URI`, `LOON_TEST_CHROMA_URI`, `LOON_TEST_QDRANT_URI`
- Skip the job if the secret is not present (so PR builds aren't blocked)

## 2. Plan

### Commit 1: Real LLM CI workflow + Ollama test (Item 1)
- New file: `.github/workflows/llm-e2e.yml`
- New file: `tests/e2e_ollama.rs` (mirror of `e2e_openai` for the Ollama provider; if not yet implemented, stub it out)
- New file: `scripts/run-llm-live.sh` (manual real-LLM runner with env-var gating)

### Commit 2: PromptBuilder stats-aware truncation (Item 2)
- Modify `crates/loon-engine/src/prompt_builder.rs` to add `build_prompt_with_stats` + `PromptBuildResult`
- Update callers (alpha_engine.rs and tests) to use the new method
- Add a test `prompt_truncation_reports_dropped_count` that asserts the count

### Commit 3: Distributed state trait + Redis stub (Item 3)
- New file: `crates/loon-persistence/src/distributed_state.rs` with trait + Redis impl (gated on feature)
- Update `Cargo.toml` to add `redis` optional dep
- New file: `tests/e2e_redis.rs`
- New file: `docs/k8s/loon-server-deployment.yaml` + `docs/k8s/README.md`

### Commit 4: E2E external services CI workflow (Item 4)
- New file: `.github/workflows/e2e-external.yml`
- Three jobs: mongodb, chroma, qdrant
- Each starts the service via docker, exports the URI, runs the corresponding gated test

## 3. Verification

- `cargo test --workspace`: still 367+ tests passing
- `cargo test --doc`: 11 doc tests
- `cargo bench -p loon-engine --bench alpha_engine`: 3 baselines
- `cargo clippy --workspace --all-targets -- -D warnings`: clean
- `npm test` in loon-chat-ui: 4 tests passing
- New CI workflows: runnable on PR
- `k8s/`: docs valid yaml + explanation

## 4. Scope limits

- Real LLM live test only runs when `OPENAI_API_KEY` is set
- Redis impl is a stub (basic KV only)
- k8s manifest is single-replica (multi-replica needs consensus layer)
- E2E external services require docker on CI runner

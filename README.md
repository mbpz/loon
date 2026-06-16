# loon

A Rust reimplementation of [Parlant](https://github.com/emcie-co/parlant):
an interaction control harness for customer-facing AI agents.

## Status

Phase 1+2 in progress. See `docs/superpowers/specs/2026-06-15-loon-phase1-design.md` for the design spec and `docs/superpowers/plans/2026-06-15-loon-phase1.md` for the implementation plan.

## Crates

| Crate | Purpose |
|-------|---------|
| `loon-core` | Domain entities + Store traits + EntityCQ |
| `loon-emission` | EventEmitter / EventBuffer / EventPublisher |
| `loon-app-modules` | 13 business modules wrapping stores |
| `loon-persistence` | DocumentDatabase trait + JSON file backend |
| `loon-nlp` | NlpService trait + OpenAI provider |
| `loon-engine` | AlphaEngine 4-stage pipeline |
| `loon-sdk` | Public builder API |
| `loon-server` | axum HTTP/WS service |
| `loon` | CLI binary |

## Build

```bash
cargo build --workspace
cargo test --workspace
```

## Quickstart

```rust
use loon_sdk as p;
use loon_persistence::JsonFileDocumentDatabase;
use loon_nlp::providers::openai::OpenAiProvider;
use loon_nlp::NlpConfig;
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = p::Server::builder()
        .with_document_db(Arc::new(JsonFileDocumentDatabase::new("./data", Duration::from_secs(5))?))
        .with_nlp_service(Arc::new(OpenAiProvider::new(Arc::new(NlpConfig::from_env()?))))
        .build()
        .await?;

    server.run(|server| async move {
        let _session_id = loon_core::SessionId::new();
        let _response = server.process_message(&loon_core::SessionId::new(), "hi").await?;
        Ok(())
    }).await?;
    Ok(())
}
```

## License

Apache-2.0
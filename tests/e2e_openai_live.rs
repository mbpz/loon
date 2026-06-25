//! Real-LLM integration test against the OpenAI API.
//!
//! Skipped unless `LOON_TEST_LIVE_OPENAI=1` and `OPENAI_API_KEY` are
//! both set. Used by `scripts/run-llm-live.sh` for manual verification
//! of the live HTTP wiring (auth headers, JSON encoding, response
//! deserialization) against a real upstream provider.

use std::sync::Arc;
use std::time::Duration;

use loon_nlp::providers::openai::OpenAiSchematicGenerator;
use loon_nlp::{define_schematic, NlpConfig, SchematicGenerator};

define_schematic! {
    pub struct TestReply { pub reply: String }
}

#[tokio::test]
async fn e2e_openai_live_call() {
    if std::env::var("LOON_TEST_LIVE_OPENAI").ok().as_deref() != Some("1") {
        eprintln!("SKIP: LOON_TEST_LIVE_OPENAI not set");
        return;
    }
    let api_key = match std::env::var("OPENAI_API_KEY") {
        Ok(k) => k,
        Err(_) => {
            eprintln!("SKIP: OPENAI_API_KEY not set");
            return;
        }
    };
    let config = Arc::new(NlpConfig {
        provider: "openai".into(),
        model: "gpt-4o-mini".into(),
        endpoint: None,
        api_key,
        max_retries: 0,
        timeout: Duration::from_secs(30),
        temperature: 0.0,
    });
    let generator = OpenAiSchematicGenerator::<TestReply>::new(config);
    let result = generator
        .generate("reply with just the word 'pong'".into(), Default::default())
        .await
        .expect("real OpenAI call failed");
    assert!(
        !result.value.reply.is_empty(),
        "reply should not be empty: {result:?}"
    );
    eprintln!("OpenAI replied: {}", result.value.reply);
}

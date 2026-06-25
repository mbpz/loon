//! End-to-end integration test that exercises the OpenAI-compatible
//! schematic generator against a wiremock-served Ollama-style endpoint.
//!
//! Ollama exposes `/v1/chat/completions` with the same JSON shape as
//! OpenAI's chat-completions API, so `OpenAiSchematicGenerator` works
//! against an Ollama URL out of the box — this test locks down that
//! contract without burning real provider credits.

use std::sync::Arc;
use std::time::Duration;

use loon_nlp::providers::openai::OpenAiSchematicGenerator;
use loon_nlp::{define_schematic, NlpConfig, SchematicGenerator};

define_schematic! {
    pub struct TestReply { pub reply: String }
}

#[tokio::test]
async fn e2e_ollama_provider_parses_response() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"choices":[{"message":{"content":"{\"reply\":\"Hi from mocked Ollama\"}"},"finish_reason":"stop"}],"usage":{"prompt_tokens":2,"completion_tokens":3,"total_tokens":5}}"#,
        ))
        .mount(&server)
        .await;

    let config = Arc::new(NlpConfig {
        provider: "ollama".into(),
        model: "llama3".into(),
        endpoint: Some(server.uri()),
        api_key: "ollama".into(),
        max_retries: 0,
        timeout: Duration::from_secs(5),
        temperature: 0.2,
    });

    let generator = OpenAiSchematicGenerator::<TestReply>::new(config);
    let result = generator
        .generate("hello".into(), Default::default())
        .await
        .expect("schematic generation succeeds against wiremock Ollama");
    assert_eq!(result.value.reply, "Hi from mocked Ollama");
    assert_eq!(result.info.model, "llama3");
    assert_eq!(result.info.total_tokens, 5);
}

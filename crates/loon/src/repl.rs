//! Interactive REPL that streams chat over WebSocket.
//!
//! Connects to `<server>/v1/sessions/<session_id>/chat`, sends each
//! line of stdin as a `user_message`, and prints streamed
//! `agent_message` deltas until a `done` marker is received.

use anyhow::Context;
use futures::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;

pub async fn run(server_url: &str, session_id: &str) -> anyhow::Result<()> {
    let url = format!(
        "{}/v1/sessions/{}/chat",
        server_url.trim_end_matches('/'),
        session_id
    );
    let (mut ws, _) = tokio_tungstenite::connect_async(&url)
        .await
        .context("connecting to WS")?;

    println!("Connected. Type a message and press Enter. Ctrl-D to exit.");

    let mut input = String::new();
    loop {
        print!("> ");
        use std::io::Write;
        std::io::stdout().flush()?;
        let n = std::io::stdin().read_line(&mut input)?;
        if n == 0 {
            break;
        }
        let escaped = input.trim().replace('"', "\\\"");
        let msg = format!(r#"{{"type":"user_message","content":"{}"}}"#, escaped);
        ws.send(Message::Text(msg.into())).await?;
        input.clear();
        while let Some(Ok(msg)) = ws.next().await {
            if let Message::Text(t) = msg {
                if t == r#"{"type":"done"}"# {
                    println!();
                    break;
                }
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&t) {
                    if v["type"] == "agent_message" {
                        if let Some(delta) = v["delta"].as_str() {
                            print!("{}", delta);
                            std::io::stdout().flush()?;
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Compile-time test: ensures the REPL module is reachable from
    /// the lib root and its public signature is stable. We do not
    /// actually open a WebSocket in the unit test; that would
    /// require a real server. The test just exercises the symbol
    /// reference.
    #[tokio::test]
    async fn repl_run_signature_compiles() {
        // We never call run() (no real server). The point of this
        // test is that the symbol resolves and the function
        // signature is `async fn(&str, &str) -> anyhow::Result<()>`.
        let _f: fn(&'static str, &'static str) -> _ = |_s, _sid| run(_s, _sid);
    }
}

//! Live check that a native-tool-calling model actually calls a tool.
//!
//! This is the regression that unit tests cannot catch. `minimax-m3` is trained
//! against a tool API; describing tools only in the system prompt left it with
//! nothing to call, so it narrated its intent ("Let me first check the
//! workspace…") and returned an empty turn. Every schema assertion in the unit
//! suite passed throughout — only a real model exercises the behaviour.
//!
//! Ignored by default: needs a running Ollama with cloud access.
//!
//! ```bash
//! cargo test -p vibe-ai --test ollama_native_tools_live -- --ignored --nocapture
//! ```

use vibe_ai::provider::{AIProvider, Message, MessageRole, ProviderConfig};
use vibe_ai::providers::ollama::OllamaProvider;

const MODEL: &str = "minimax-m3:cloud";
const OLLAMA: &str = "http://127.0.0.1:11434";

/// The prompt from the original bug report.
const TASK: &str = "create a small program that displays mandelbrot set in color";

fn provider() -> OllamaProvider {
    OllamaProvider::new(ProviderConfig {
        provider_type: "ollama".into(),
        api_key: None,
        api_url: Some(OLLAMA.into()),
        model: MODEL.into(),
        max_tokens: None,
        temperature: None,
        ..Default::default()
    })
}

fn agent_conversation() -> Vec<Message> {
    vec![
        Message {
            role: MessageRole::System,
            content: vibe_ai::tools::TOOL_SYSTEM_PROMPT.to_string(),
        },
        Message {
            role: MessageRole::User,
            content: TASK.to_string(),
        },
    ]
}

#[tokio::test]
#[ignore = "requires a live Ollama with cloud access"]
async fn agent_turn_produces_a_parsable_tool_call() {
    let reply = provider()
        .chat(&agent_conversation(), None)
        .await
        .expect("chat failed — is ollama running and signed in?");

    println!("\n--- raw turn ---\n{reply}\n----------------\n");

    let visible = vibe_ai::tools::strip_thinking(&reply);
    let calls = vibe_ai::tools::parse_tool_calls(&visible);
    println!("parsed {} tool call(s)", calls.len());

    assert!(
        !calls.is_empty(),
        "model returned no parsable tool call — this is exactly the bug. \
         Visible turn was: {visible:?}"
    );
}

/// The failure mode itself: with no tools advertised, the turn is reasoning and
/// nothing else. Not asserted as a hard requirement — a model is free to answer
/// in prose — but printed so the A/B is visible in the log.
#[tokio::test]
#[ignore = "requires a live Ollama with cloud access"]
async fn without_the_agent_prompt_no_tools_are_sent() {
    // No TOOL_SYSTEM_PROMPT → `tools_for` returns None → plain chat.
    let reply = provider()
        .chat(
            &[Message {
                role: MessageRole::User,
                content: TASK.to_string(),
            }],
            None,
        )
        .await
        .expect("chat failed");

    let visible = vibe_ai::tools::strip_thinking(&reply);
    let calls = vibe_ai::tools::parse_tool_calls(&visible);
    println!(
        "\n[plain chat] tool calls: {} | visible chars: {}\n",
        calls.len(),
        visible.trim().len()
    );
    assert!(
        calls.is_empty(),
        "a plain chat must never emit tool markup — it would render as literal text"
    );
}

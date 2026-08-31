//! Live check that a truncated reply is *reported* as truncated.
//!
//! This is the regression unit tests cannot catch. The unit suite asserts that
//! `done_reason` maps to [`StopReason`] correctly, but it feeds the parser
//! hand-written JSON — it cannot prove the real server sends the field, that it
//! survives the streaming path, or that the sink is filled by the time the
//! caller reads it. The original bug was exactly a field nobody read, and every
//! unit test passed while the chat stalled on every long answer.
//!
//! Ignored by default: needs a running Ollama.
//!
//! ```bash
//! cargo test -p vibe-ai --test ollama_stop_reason_live -- --ignored --nocapture
//! ```

use futures::StreamExt;
use vibe_ai::provider::{AIProvider, Message, MessageRole, ProviderConfig};
use vibe_ai::providers::ollama::OllamaProvider;
use vibe_ai::{stop_reason_sink, taken_stop_reason, StopReason};

const MODEL: &str = "llama3.2:latest";
const OLLAMA: &str = "http://127.0.0.1:11434";

fn provider(max_tokens: Option<usize>) -> OllamaProvider {
    OllamaProvider::new(ProviderConfig {
        provider_type: "ollama".into(),
        api_key: None,
        api_url: Some(OLLAMA.into()),
        model: MODEL.into(),
        max_tokens,
        temperature: None,
        ..Default::default()
    })
}

fn ask(prompt: &str) -> Vec<Message> {
    vec![Message {
        role: MessageRole::User,
        content: prompt.to_string(),
    }]
}

/// Drain the reporting stream and hand back the text plus the recorded reason.
async fn stream_and_report(
    p: &OllamaProvider,
    prompt: &str,
) -> (String, Option<StopReason>) {
    let sink = stop_reason_sink();
    let mut stream = p
        .stream_chat_reporting(&ask(prompt), sink.clone())
        .await
        .expect("ollama must accept the request");

    let mut text = String::new();
    while let Some(chunk) = stream.next().await {
        text.push_str(&chunk.expect("no transport error"));
    }
    // Read *after* the stream is drained, which is the contract the chat
    // command relies on.
    let reason = taken_stop_reason(&sink);
    (text, reason)
}

#[tokio::test]
#[ignore = "needs a running Ollama"]
async fn a_reply_cut_at_the_cap_reports_length() {
    // A tiny cap against a prompt that cannot possibly be answered within it.
    let p = provider(Some(16));
    let (text, reason) = stream_and_report(
        &p,
        "Write a detailed 2000-word essay about the history of computing.",
    )
    .await;

    println!("truncated text: {text:?}");
    println!("reported reason: {reason:?}");

    assert_eq!(
        reason,
        Some(StopReason::Length),
        "a reply cut at num_predict must report Length — this is what the chat \
         panel keys off to continue the work instead of going quiet"
    );
    assert!(
        reason.as_ref().is_some_and(StopReason::is_truncated),
        "Length must satisfy is_truncated, which is the auto-continue predicate"
    );
    assert!(!text.is_empty(), "the truncated reply still carries its text");
}

#[tokio::test]
#[ignore = "needs a running Ollama"]
async fn a_reply_that_finishes_reports_natural_not_length() {
    // The contrast that matters: if everything reported Length the panel would
    // continue forever, so a finished reply must be distinguishable.
    let p = provider(Some(200));
    let (text, reason) = stream_and_report(&p, "Reply with exactly the word: ok").await;

    println!("finished text: {text:?}");
    println!("reported reason: {reason:?}");

    assert_eq!(reason, Some(StopReason::Natural));
    assert!(
        !reason.as_ref().is_some_and(StopReason::is_truncated),
        "a finished reply must never trigger an auto-continue"
    );
}
